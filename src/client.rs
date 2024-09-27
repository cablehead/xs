use serde_json::Value;

use futures::StreamExt;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};
use tokio::sync::mpsc::Receiver;

use http_body_util::{combinators::BoxBody, BodyExt, Empty};
use hyper::body::Bytes;
use hyper::client::conn::http1;
use hyper::{Method, Request, StatusCode};
use hyper_util::rt::TokioIo;

use crate::listener::AsyncReadWriteBox;
use crate::store::TTL;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

async fn connect(addr: &str) -> Result<AsyncReadWriteBox, BoxError> {
    if addr.starts_with('/') || addr.starts_with('.') {
        let path = std::path::Path::new(addr);
        let addr = if path.is_dir() {
            path.join("sock")
        } else {
            path.to_path_buf()
        };
        let stream = UnixStream::connect(addr).await?;
        Ok(Box::new(stream))
    } else {
        let addr = if addr.starts_with(':') {
            format!("127.0.0.1{}", addr)
        } else {
            addr.to_string()
        };
        let stream = TcpStream::connect(addr).await?;
        Ok(Box::new(stream))
    }
}

fn empty() -> BoxBody<Bytes, BoxError> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

pub async fn cat(
    addr: &str,
    follow: bool,
    pulse: Option<u64>,
    tail: bool,
    last_id: Option<String>,
    limit: Option<u64>,
    sse: bool,
) -> Result<Receiver<Bytes>, BoxError> {
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

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

    let uri = if !params.is_empty() {
        format!("http://localhost/?{}", params.join("&"))
    } else {
        "http://localhost/".to_string()
    };

    let mut req = Request::builder().method(Method::GET).uri(uri);

    if sse {
        req = req.header("Accept", "text/event-stream");
    }

    let req = req.body(empty())?;

    let res = sender.send_request(req).await?;

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
                    // Ignore non-data frames
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
