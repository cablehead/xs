use hyper::{Method, Request};
use hyper_util::rt::TokioIo;

use super::connect::connect;
use super::types::{BoxError, RequestParts};

pub async fn request<B>(
    addr: &str,
    method: Method,
    path: &str,
    query: Option<&str>,
    body: B,
    headers: Option<Vec<(String, String)>>,
) -> Result<hyper::Response<hyper::body::Incoming>, BoxError>
where
    B: hyper::body::Body<Data = hyper::body::Bytes> + Send + 'static,
    B::Error: Into<BoxError> + Send,
{
    let parts = RequestParts::parse(addr, path, query)?;
    let stream = connect(&parts).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let mut builder = Request::builder()
        .method(method)
        .uri(parts.uri)
        .header(hyper::header::USER_AGENT, "xs/0.1")
        .header(hyper::header::ACCEPT, "*/*");

    if let Some(host) = parts.host {
        builder = builder.header(hyper::header::HOST, host);
    }
    if let Some(auth) = parts.authorization {
        builder = builder.header(hyper::header::AUTHORIZATION, auth);
    }

    if let Some(extra_headers) = headers {
        for (name, value) in extra_headers {
            builder = builder.header(name, value);
        }
    }

    let req = builder.body(body)?;
    sender.send_request(req).await.map_err(Into::into)
}
