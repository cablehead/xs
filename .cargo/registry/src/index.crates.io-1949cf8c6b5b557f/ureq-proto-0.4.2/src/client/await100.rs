use http::StatusCode;

use crate::parser::try_parse_response;
use crate::Error;

use super::state::Await100;
use super::{Await100Result, Call};
use crate::CloseReason;

impl Call<Await100> {
    /// Attempt to read a 100-continue response.
    ///
    /// Tries to interpret bytes sent by the server as a 100-continue response. The expect-100 mechanic
    /// means we hope the server will give us an indication on whether to upload a potentially big
    /// request body, before we start doing it.
    ///
    /// * If the server supports expect-100, it will respond `HTTP/1.1 100 Continue\r\n\r\n`, or
    ///   some other response code (such as 403) if we are not allowed to post the body.
    /// * If the server does not support expect-100, it will not respond at all, in which case
    ///   we will proceed to sending the request body after some timeout.
    ///
    /// The results are:
    ///
    /// * `Ok(0)` - not enough data yet, continue waiting (or `proceed()` if you think we waited enough)
    /// * `Ok(n)` - `n` number of input bytes were consumed. Call `proceed()` next
    /// * `Err(e)` - some error that is not recoverable
    pub fn try_read_100(&mut self, input: &[u8]) -> Result<usize, Error> {
        // Try parsing a status line without any headers. The line we are looking for is:
        //
        //   HTTP/1.1 100 Continue\r\n\r\n
        //
        // There should be no headers.
        match try_parse_response::<0>(input) {
            Ok(v) => match v {
                Some((input_used, response)) => {
                    self.inner.await_100_continue = false;

                    if response.status() == StatusCode::CONTINUE {
                        // should_send_body ought to be true since initialization.
                        assert!(self.inner.should_send_body);
                        Ok(input_used)
                    } else {
                        // We encountered a status line, without headers, but it wasn't 100,
                        // so we should not continue to send the body. Furthermore we mustn't
                        // reuse the connection.
                        // https://curl.se/mail/lib-2004-08/0002.html
                        self.inner.close_reason.push(CloseReason::Not100Continue);
                        self.inner.should_send_body = false;
                        Ok(0)
                    }
                }
                // Not enough input yet.
                None => Ok(0),
            },
            Err(e) => {
                self.inner.await_100_continue = false;

                if e == Error::HttpParseTooManyHeaders {
                    // We encountered headers after the status line. That means the server did
                    // not send 100-continue, and also continued to produce an answer before we
                    // sent the body. Regardless of what the answer is, we must not send the body.
                    // A 200-answer would be nonsensical given we haven't yet sent the body.
                    //
                    // We do however want to receive the response to be able to provide
                    // the Response<()> to the user. Hence this is not considered an error.
                    self.inner.close_reason.push(CloseReason::Not100Continue);
                    self.inner.should_send_body = false;
                    Ok(0)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Tell if there is any point in waiting for more data from the server.
    ///
    /// Becomes `false` as soon as `try_read_100()` got enough data to determine what to do next.
    /// This might become `false` even if `try_read_100` returns `Ok(0)`.
    ///
    /// If this returns `false`, the user should continue with `proceed()`.
    pub fn can_keep_await_100(&self) -> bool {
        self.inner.await_100_continue
    }

    /// Proceed to the next state.
    pub fn proceed(self) -> Result<Await100Result, Error> {
        // We can always proceed out of Await100

        if self.inner.should_send_body {
            // TODO(martin): do i need this?
            // call.inner.call.analyze_request()?;
            let call = Call::wrap(self.inner);
            Ok(Await100Result::SendBody(call))
        } else {
            Ok(Await100Result::RecvResponse(Call::wrap(self.inner)))
        }
    }
}
