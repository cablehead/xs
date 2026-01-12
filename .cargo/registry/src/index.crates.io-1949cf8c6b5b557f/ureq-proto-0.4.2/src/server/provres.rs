use http::{header, Response};

use crate::{CloseReason, Error};

use super::state::{ProvideResponse, SendResponse};
use super::{append_request, Reply};

impl Reply<ProvideResponse> {
    /// Provide a response to the client's request.
    ///
    /// Takes a Response object and transitions to the SendResponse state.
    /// Handles setting appropriate headers for the response body if they weren't already set.
    pub fn provide(self, response: Response<()>) -> Result<Reply<SendResponse>, Error> {
        if self.inner.expect_100_reject
            && !response.status().is_client_error()
            && !response.status().is_server_error()
        {
            return Err(Error::BadReject100Status(response.status()));
        }

        let mut inner = append_request(self.inner, response);

        // unwrap are correct due to state we should be in when we get here.
        let response = inner.response.as_mut().unwrap();

        if response
            .headers()
            .any(|(h, v)| h == header::CONNECTION && v == "close")
        {
            inner.close_reason.push(CloseReason::ServerConnectionClose);
        }

        let writer = inner.state.writer.take().unwrap();
        let info = response.analyze(writer)?;

        if !info.res_body_header && info.body_mode.has_body() {
            // User did not set a body header, we set one.
            let header = info.body_mode.body_header();
            response.set_header(header.0, header.1)?;
        }

        inner.state.writer = Some(info.body_mode);

        Ok(Reply::wrap(inner))
    }
}
