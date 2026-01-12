use http::{header, HeaderName, HeaderValue, Response, StatusCode, Version};

use crate::body::BodyReader;
use crate::ext::HeaderIterExt;
use crate::parser::{try_parse_partial_response, try_parse_response};
use crate::util::log_data;
use crate::Error;

use super::state::RecvResponse;
use super::MAX_RESPONSE_HEADERS;
use super::{Call, CloseReason, RecvResponseResult};

impl Call<RecvResponse> {
    /// Try reading a response from the input.
    ///
    /// * `allow_partial_redirect` - if `true`, we can accept to find the `Location` header
    ///   and proceed without reading the entire header. This is useful for broken servers that
    ///   don't send an entire \r\n at the end of the preamble.
    ///
    /// The `(usize, Option<Response()>)` is `(input amount consumed, response`).
    ///
    /// Notice that it's possible that we get an `input amount consumed` despite not returning
    /// a `Some(Response)`. This can happen if the server returned a 100-continue, and due to
    /// timing reasons we did not receive it while we were in the `Await100` call state. This
    /// "spurios" 100 will be discarded before we parse the actual response.
    pub fn try_response(
        &mut self,
        input: &[u8],
        allow_partial_redirect: bool,
    ) -> Result<(usize, Option<Response<()>>), Error> {
        let maybe_response = self.do_try_response(input, allow_partial_redirect)?;

        let (input_used, response) = match maybe_response {
            Some(v) => v,
            // Not enough input for a full response yet
            None => return Ok((0, None)),
        };

        if response.status() == StatusCode::CONTINUE && self.inner.await_100_continue {
            // We have received a "delayed" 100-continue. This means the server did
            // not produce the 100-continue response in time while we were in the
            // state Await100. This is not an error, it can happen if the network is slow.
            self.inner.await_100_continue = false;

            // We should consume the response and wait for the next.
            return Ok((input_used, None));
        }

        self.inner.status = Some(response.status());
        // We want the last Location header.
        self.inner.location = response
            .headers()
            .get_all(header::LOCATION)
            .into_iter()
            .next_back()
            .cloned();

        if response.headers().iter().has(header::CONNECTION, "close") {
            self.inner
                .close_reason
                .push(CloseReason::ServerConnectionClose);
        }

        Ok((input_used, Some(response)))
    }

    /// Try reading response headers
    ///
    /// A response is only possible once the `input` holds all the HTTP response
    /// headers. Before that this returns `None`. When the response is succesfully read,
    /// the return value `(usize, Response<()>)` contains how many bytes were consumed
    /// of the `input`.
    fn do_try_response(
        &mut self,
        input: &[u8],
        allow_partial_redirect: bool,
    ) -> Result<Option<(usize, Response<()>)>, Error> {
        // ~3k for 100 headers
        let (input_used, response) = match try_parse_response::<MAX_RESPONSE_HEADERS>(input)? {
            Some(v) => v,
            None => {
                // The caller decides whether to allow a partial parse.
                if !allow_partial_redirect {
                    return Ok(None);
                }

                // TODO(martin): I don't like this code. The mission is to be correct HTTP/1.1
                // and this is a hack to allow for broken servers.
                //
                // As a special case, to handle broken servers that does a redirect without
                // the final trailing \r\n, we try parsing the response as partial, and
                // if it is a redirect, we can allow the request to continue.
                let Some(mut r) = try_parse_partial_response::<MAX_RESPONSE_HEADERS>(input)? else {
                    return Ok(None);
                };

                // A redirection must have a location header.
                let is_complete_redirection =
                    r.status().is_redirection() && r.headers().contains_key(header::LOCATION);

                if !is_complete_redirection {
                    return Ok(None);
                }

                // Insert a synthetic connection: close, since the connection is
                // not valid after using a partial request.
                debug!("Partial redirection response, insert fake connection: close");
                r.headers_mut()
                    .insert(header::CONNECTION, HeaderValue::from_static("close"));

                (input.len(), r)
            }
        };

        log_data(&input[..input_used]);

        let http10 = response.version() == Version::HTTP_10;
        let status = response.status().as_u16();

        if status == StatusCode::CONTINUE {
            // There should be no headers for this response.
            if !response.headers().is_empty() {
                return Err(Error::HeadersWith100);
            }

            return Ok(Some((input_used, response)));
        }

        let header_lookup = |name: HeaderName| {
            if let Some(header) = response.headers().get(name) {
                return header.to_str().ok();
            }
            None
        };

        let recv_body_mode =
            BodyReader::for_response(http10, self.inner.request.method(), status, &header_lookup)?;

        self.inner.state.reader = Some(recv_body_mode);

        Ok(Some((input_used, response)))
    }

    /// Tell if we have finished receiving the response.
    pub fn can_proceed(&self) -> bool {
        self.inner.state.reader.is_some()
    }

    /// Tell if response body is closed delimited
    ///
    /// HTTP/1.0 does not have `content-length` to serialize many requests over the same
    /// socket. Instead it uses socket close to determine the body is finished.
    fn is_close_delimited(&self) -> bool {
        let rbm = self.inner.state.reader.as_ref().unwrap();
        matches!(rbm, BodyReader::CloseDelimited)
    }

    /// Proceed to the next state.
    ///
    /// This returns `None` if we have not finished receiving the response. It is guaranteed that if
    /// `can_proceed()` returns true, this will return `Some`.
    pub fn proceed(mut self) -> Option<RecvResponseResult> {
        if !self.can_proceed() {
            return None;
        }

        let has_response_body = self.inner.state.need_response_body();

        if has_response_body {
            if self.is_close_delimited() {
                self.inner
                    .close_reason
                    .push(CloseReason::CloseDelimitedBody);
            }

            Some(RecvResponseResult::RecvBody(Call::wrap(self.inner)))
        } else {
            Some(if self.inner.is_redirect() {
                RecvResponseResult::Redirect(Call::wrap(self.inner))
            } else {
                RecvResponseResult::Cleanup(Call::wrap(self.inner))
            })
        }
    }
}
