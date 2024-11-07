use crate::store::TTL;
use futures::StreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, StreamBody};
use hyper::body::Bytes;
use hyper::{Method, Request};
use ssri::Integrity;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::Receiver;
use tokio_util::io::ReaderStream;

use super::request;

pub async fn cat(
    addr: &str,
    follow: bool,
    pulse: Option<u64>,
    tail: bool,
    last_id: Option<String>,
    limit: Option<u64>,
    sse: bool,
) -> Result<Receiver<Bytes>, Box<dyn std::error::Error + Send + Sync>> {
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

    let res = request::request(addr, Method::GET, "", query.as_deref(), empty(), headers).await?;

    if res.status() != hyper::StatusCode::OK {
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

pub async fn append<R>(
    addr: &str,
    topic: &str,
    data: R,
    meta: Option<&serde_json::Value>,
    ttl: Option<TTL>,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let stream = super::connect(addr).await?;
    let io = hyper_util::rt::TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
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

    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });

    let body = StreamBody::new(mapped_stream);
    let req = req.body(body)?;
    let res = sender.send_request(req).await?;

    if res.status() != hyper::StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn cas_get<W>(
    addr: &str,
    integrity: Integrity,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    W: AsyncWrite + Unpin,
{
    let stream = super::connect(addr).await?;
    let io = hyper_util::rt::TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

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

    if res.status() != hyper::StatusCode::OK {
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

pub async fn pipe<R>(
    addr: &str,
    id: &str,
    data: R,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let stream = super::connect(addr).await?;
    let io = hyper_util::rt::TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let uri = format!("http://localhost/pipe/{}", id);
    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });

    let body = StreamBody::new(mapped_stream);
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .body(body)?;

    let res = sender.send_request(req).await?;

    if res.status() != hyper::StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn get(addr: &str, id: &str) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let res = request::request(addr, Method::GET, id, None, empty(), None).await?;

    if res.status() != hyper::StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn remove(addr: &str, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stream = super::connect(addr).await?;
    let io = hyper_util::rt::TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

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
        hyper::StatusCode::NO_CONTENT => Ok(()),
        hyper::StatusCode::NOT_FOUND => Err(format!("not found: {}", id).into()),
        _ => Err(format!("HTTP error: {}", res.status()).into()),
    }
}

pub async fn head(
    addr: &str,
    topic: &str,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let stream = super::connect(addr).await?;
    let io = hyper_util::rt::TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

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

    if res.status() != hyper::StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

fn empty() -> BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
