use serde_json::Value;

use futures::StreamExt;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::Receiver;

use http_body_util::{combinators::BoxBody, BodyExt, Empty};
use hyper::body::Bytes;
use hyper::client::conn::http1;
use hyper::{Method, Request, StatusCode};
use hyper_util::rt::TokioIo;

use crate::connect::connect;
use crate::store::TTL;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn cat(
    addr: &str,
    follow: bool,
    pulse: Option<u64>,
    tail: bool,
    last_id: Option<String>,
    limit: Option<u64>,
    sse: bool,
) -> Result<Receiver<Bytes>, BoxError> {
    let mut params = Vec::new();
    if follow {
        if let Some(pulse_value) = pulse {
            params.push(format!("follow={}", pulse_value));
        } else {
            params.push("follow=true".to_string());
        }
    }
    if tail {
        params.push("tail".to_string());
    }
    if let Some(ref last_id_value) = last_id {
        params.push(format!("last-id={}", last_id_value));
    }
    if let Some(limit_value) = limit {
        params.push(format!("limit={}", limit_value));
    }

    let query = if !params.is_empty() {
        Some(params.join("&"))
    } else {
        None
    };

    let headers = if sse {
        Some(vec![(
            "Accept".to_string(),
            "text/event-stream".to_string(),
        )])
    } else {
        None
    };

    let res = request(addr, Method::GET, "", query.as_deref(), empty(), headers).await?;

    if res.status() != StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let (_parts, mut body) = res.into_parts();
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        while let Some(frame_result) = body.frame().await {
            match frame_result {
                Ok(frame) => {
                    if let Ok(bytes) = frame.into_data() {
                        if tx.send(bytes).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading body: {}", e);
                    break;
                }
            }
        }
    });

    Ok(rx)
}

use http_body_util::StreamBody;
use tokio_util::io::ReaderStream;

pub async fn append<R>(
    addr: &str,
    topic: &str,
    data: R,
    meta: Option<&Value>,
    ttl: Option<TTL>,
) -> Result<Bytes, BoxError>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let mut uri = format!("http://localhost/{}", topic);
    if let Some(ttl) = ttl {
        uri = format!("{}?{}", uri, ttl.to_query());
    }

    let mut req = Request::builder().method(Method::POST).uri(uri);

    if let Some(meta_value) = meta {
        req = req.header("xs-meta", serde_json::to_string(meta_value)?);
    }

    // Create a stream from the AsyncRead
    let reader_stream = ReaderStream::new(data);

    // Map the stream to convert io::Error to BoxError
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as BoxError)
    });

    // Create a StreamBody
    let body = StreamBody::new(mapped_stream);

    let req = req.body(body)?;
    let res = sender.send_request(req).await?;

    if res.status() != StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn cas_get<W>(
    addr: &str,
    integrity: ssri::Integrity,
    writer: &mut W,
) -> Result<(), BoxError>
where
    W: AsyncWrite + Unpin,
{
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let uri = format!("http://localhost/cas/{}", integrity);
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(empty())?;

    let res = sender.send_request(req).await?;

    if res.status() != StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let mut body = res.into_body();

    while let Some(frame) = body.frame().await {
        let frame = frame?;
        if let Ok(chunk) = frame.into_data() {
            writer.write_all(&chunk).await?;
        }
    }

    writer.flush().await?;

    Ok(())
}

pub async fn pipe<R>(addr: &str, id: &str, data: R) -> Result<Bytes, BoxError>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let uri = format!("http://localhost/pipe/{}", id);

    // Create a stream from the AsyncRead
    let reader_stream = ReaderStream::new(data);

    // Map the stream to convert io::Error to BoxError
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as BoxError)
    });

    // Create a StreamBody
    let body = StreamBody::new(mapped_stream);

    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .body(body)?;

    let res = sender.send_request(req).await?;

    if res.status() != StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn get(addr: &str, id: &str) -> Result<Bytes, BoxError> {
    let res = request(addr, Method::GET, id, None, empty(), None).await?;

    if res.status() != StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn head(addr: &str, topic: &str) -> Result<Bytes, BoxError> {
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let uri = format!("http://localhost/head/{}", topic);
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(empty())?;

    let res = sender.send_request(req).await?;

    if res.status() != StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn remove(addr: &str, id: &str) -> Result<(), BoxError> {
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let uri = format!("http://localhost/{}", id);
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(uri)
        .body(empty())?;

    let res = sender.send_request(req).await?;

    match res.status() {
        StatusCode::NO_CONTENT => Ok(()),
        StatusCode::NOT_FOUND => Err(format!("not found: {}", id).into()),
        _ => Err(format!("HTTP error: {}", res.status()).into()),
    }
}

//
// HTTP request related functions

use base64::prelude::*;

#[derive(Default, Debug, PartialEq)]
struct RequestParts {
    uri: String,
    host: Option<String>,
    authorization: Option<String>,
}

fn parse_request_parts(
    addr: &str,
    path: &str,
    query: Option<&str>,
) -> Result<RequestParts, BoxError> {
    let mut parts = RequestParts::default();

    // Unix socket case
    if addr.starts_with('/') || addr.starts_with('.') {
        parts.uri = if let Some(q) = query {
            format!("http://localhost/{}?{}", path, q)
        } else {
            format!("http://localhost/{}", path)
        };
        return Ok(parts);
    }

    // Convert port-only or bare host to URL
    let addr = if addr.starts_with(':') {
        format!("http://127.0.0.1{}", addr)
    } else if !addr.contains("://") {
        format!("http://{}", addr)
    } else {
        addr.to_string()
    };

    let url = url::Url::parse(&addr)?;

    // Build the clean request URI (no auth)
    let scheme = url.scheme();
    let host = url.host_str().ok_or("Missing host")?;
    let port = url.port().map(|p| format!(":{}", p)).unwrap_or_default();

    parts.uri = if let Some(q) = query {
        format!("{}://{}{}/{}?{}", scheme, host, port, path, q)
    } else {
        format!("{}://{}{}/{}", scheme, host, port, path)
    };

    // Set host header
    parts.host = Some(format!("{}{}", host, port));

    // Set auth if present
    if let Some(password) = url.password() {
        let credentials = format!("{}:{}", url.username(), password);
        parts.authorization = Some(format!("Basic {}", BASE64_STANDARD.encode(credentials)));
    } else if !url.username().is_empty() {
        let credentials = format!("{}:", url.username());
        parts.authorization = Some(format!("Basic {}", BASE64_STANDARD.encode(credentials)));
    }

    Ok(parts)
}

async fn request(
    addr: &str,
    method: Method,
    path: &str,
    query: Option<&str>,
    body: BoxBody<Bytes, BoxError>,
    headers: Option<Vec<(String, String)>>,
) -> Result<hyper::Response<hyper::body::Incoming>, BoxError> {
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let parts = parse_request_parts(addr, path, query)?;

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

    // Add any additional headers
    if let Some(extra_headers) = headers {
        for (name, value) in extra_headers {
            builder = builder.header(name, value);
        }
    }

    let req = builder.body(body)?;
    sender.send_request(req).await.map_err(Into::into)
}

fn empty() -> BoxBody<Bytes, BoxError> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_socket() {
        let parts = parse_request_parts("./store", "foo", None).unwrap();
        assert_eq!(parts.uri, "http://localhost/foo");
        assert_eq!(parts.host, None);
        assert_eq!(parts.authorization, None);
    }

    #[test]
    fn test_port_only() {
        let parts = parse_request_parts(":8080", "bar", Some("q=1")).unwrap();
        assert_eq!(parts.uri, "http://127.0.0.1:8080/bar?q=1");
        assert_eq!(parts.host, Some("127.0.0.1:8080".to_string()));
        assert_eq!(parts.authorization, None);
    }

    #[test]
    fn test_https_url_with_auth() {
        let parts = parse_request_parts("https://user:pass@example.com:400", "", None).unwrap();
        assert_eq!(parts.uri, "https://example.com:400/");
        assert_eq!(parts.host, Some("example.com:400".to_string()));
        assert_eq!(parts.authorization, Some("Basic dXNlcjpwYXNz".to_string()));
    }
}
