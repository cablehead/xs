use futures::StreamExt;

use ssri::Integrity;

use http_body_util::{combinators::BoxBody, BodyExt, Empty, StreamBody};
use hyper::body::Bytes;
use hyper::Method;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::Receiver;
use tokio_util::io::ReaderStream;

use crate::store::TTL;

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
    // Build query string if TTL is present
    let query = ttl.map(|t| t.to_query());

    // Setup stream from data
    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });
    let body = StreamBody::new(mapped_stream);

    // Add meta header if present
    let headers = meta.map(|meta_value| {
        vec![(
            "xs-meta".to_string(),
            serde_json::to_string(meta_value).unwrap(),
        )]
    });

    let res = request::request(addr, Method::POST, topic, query.as_deref(), body, headers).await?;

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
    let res = request::request(
        addr,
        Method::GET,
        &format!("cas/{}", integrity),
        None,
        empty(),
        None,
    )
    .await?;

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

pub async fn cas_post<R>(
    addr: &str,
    data: R,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    // Setup stream from data
    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });
    let body = StreamBody::new(mapped_stream);

    let res = request::request(addr, Method::POST, "cas", None, body, None).await?;

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn process<R>(
    addr: &str,
    id: &str,
    data: R,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    // Setup stream from data
    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });
    let body = StreamBody::new(mapped_stream);

    let res = request::request(
        addr,
        Method::POST,
        &format!("process/{}", id),
        None,
        body,
        None,
    )
    .await?;

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn get(addr: &str, id: &str) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let res = request::request(addr, Method::GET, id, None, empty(), None).await?;
    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn remove(addr: &str, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = request::request(addr, Method::DELETE, id, None, empty(), None).await?;
    Ok(())
}

pub async fn head(
    addr: &str,
    topic: &str,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let res = request::request(
        addr,
        Method::GET,
        &format!("head/{}", topic),
        None,
        empty(),
        None,
    )
    .await?;

    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn import<R>(
    addr: &str,
    data: R,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });
    let body = StreamBody::new(mapped_stream);

    let res = request::request(addr, Method::POST, "import", None, body, None).await?;
    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn version(addr: &str) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    match request::request(addr, Method::GET, "version", None, empty(), None).await {
        Ok(res) => {
            let body = res.collect().await?.to_bytes();
            Ok(body)
        }
        Err(e) => {
            // this was the version before the /version endpoint was added
            if e.to_string().contains("404 Not Found") {
                Ok(Bytes::from(r#"{"version":"0.0.9"}"#))
            } else {
                Err(e) // Propagate other errors
            }
        }
    }
}

fn empty() -> BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
