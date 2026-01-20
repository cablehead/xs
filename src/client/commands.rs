use futures::StreamExt;

use base64::Engine;
use ssri::Integrity;

use http_body_util::{combinators::BoxBody, BodyExt, Empty, StreamBody};
use hyper::body::Bytes;
use hyper::Method;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::Receiver;
use tokio_util::io::ReaderStream;

use super::request;
use crate::store::{ReadOptions, TTL};

pub async fn cat(
    addr: &str,
    options: ReadOptions,
    sse: bool,
) -> Result<Receiver<Bytes>, Box<dyn std::error::Error + Send + Sync>> {
    // Convert any usize limit to u64
    let query = if options == ReadOptions::default() {
        None
    } else {
        Some(options.to_query_string())
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
                    eprintln!("Error reading body: {e}");
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
    let query = ttl.map(|t| t.to_query());

    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });
    let body = StreamBody::new(mapped_stream);

    let headers = meta.map(|meta_value| {
        let json_string = serde_json::to_string(meta_value).unwrap();
        let encoded = base64::prelude::BASE64_STANDARD.encode(json_string);
        vec![("xs-meta".to_string(), encoded)]
    });

    let res = request::request(
        addr,
        Method::POST,
        &format!("append/{topic}"),
        query.as_deref(),
        body,
        headers,
    )
    .await?;
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
    let parts = super::types::RequestParts::parse(addr, &format!("cas/{integrity}"), None)?;

    match parts.connection {
        super::types::ConnectionKind::Unix(path) => {
            // Direct CAS access for local path
            let store_path = path.parent().unwrap_or(&path).to_path_buf();
            let cas_path = store_path.join("cacache");
            match cacache::Reader::open_hash(&cas_path, integrity).await {
                Ok(mut reader) => {
                    tokio::io::copy(&mut reader, writer).await?;
                    writer.flush().await?;
                    Ok(())
                }
                Err(e) => {
                    // Check if this is an entry not found error
                    if matches!(e, cacache::Error::EntryNotFound(_, _)) {
                        return Err(Box::new(crate::error::NotFound));
                    }
                    // Also check for IO not found errors in the chain
                    let boxed_err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
                    if crate::error::has_not_found_io_error(&boxed_err) {
                        return Err(Box::new(crate::error::NotFound));
                    }
                    Err(boxed_err)
                }
            }
        }
        _ => {
            // Remote HTTP access
            let res = request::request(
                addr,
                Method::GET,
                &format!("cas/{integrity}"),
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
    }
}

pub async fn cas_post<R>(
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

    let res = request::request(addr, Method::POST, "cas", None, body, None).await?;
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

pub async fn last(
    addr: &str,
    topic: &str,
    follow: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let query = if follow { Some("follow=true") } else { None };

    let res = request::request(
        addr,
        Method::GET,
        &format!("last/{topic}"),
        query,
        empty(),
        None,
    )
    .await?;

    let mut body = res.into_body();
    let mut stdout = tokio::io::stdout();

    while let Some(frame) = body.frame().await {
        let frame = frame?;
        if let Ok(chunk) = frame.into_data() {
            stdout.write_all(&chunk).await?;
        }
    }
    stdout.flush().await?;
    Ok(())
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
            if crate::error::NotFound::is_not_found(&e) {
                Ok(Bytes::from(r#"{"version":"0.0.9"}"#))
            } else {
                Err(e)
            }
        }
    }
}

fn empty() -> BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

pub async fn eval(
    addr: &str,
    script: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let res = request::request(addr, Method::POST, "eval", None, script, None).await?;

    let mut body = res.into_body();
    let mut stdout = tokio::io::stdout();

    while let Some(frame) = body.frame().await {
        let frame = frame?;
        if let Ok(chunk) = frame.into_data() {
            stdout.write_all(&chunk).await?;
        }
    }
    stdout.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cas_get_not_found_local() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_str().unwrap();

        // Create a fake hash that doesn't exist
        let fake_hash = "sha256-fakehashnotfound0000000000000000000000000000000=";
        let integrity = Integrity::from_str(fake_hash).unwrap();

        let mut output = Vec::new();
        let result = cas_get(store_path, integrity, &mut output).await;

        // Should return NotFound error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(crate::error::NotFound::is_not_found(&err));
    }
}
