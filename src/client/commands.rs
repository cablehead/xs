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
use crate::store::{ReadError, ReadOptions, TTL};

pub async fn cat(
    addr: &str,
    options: ReadOptions,
    sse: bool,
    with_timestamp: bool,
) -> Result<Receiver<Result<Bytes, crate::error::Error>>, crate::error::Error> {
    // Convert any usize limit to u64
    let mut query_parts = Vec::new();
    if options != ReadOptions::default() {
        query_parts.push(options.to_query_string());
    }
    if with_timestamp {
        query_parts.push("with-timestamp".to_string());
    }
    let query = if query_parts.is_empty() {
        None
    } else {
        Some(query_parts.join("&"))
    };

    let mut headers = vec![trailers()];
    if sse {
        headers.push(("Accept".to_string(), "text/event-stream".to_string()));
    }

    let res = request::request(
        addr,
        Method::GET,
        "",
        query.as_deref(),
        empty(),
        Some(headers),
    )
    .await?;

    let (_parts, mut body) = res.into_parts();
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    // The server's bytes, as they arrive; a read that fell behind ends with
    // its error, from the `xs-lagged` trailer (ADR 0008), and a broken
    // connection with its own.
    tokio::spawn(async move {
        while let Some(frame) = body.frame().await {
            let item = match frame {
                Ok(frame) => match frame.into_data() {
                    Ok(bytes) => Ok(bytes),
                    Err(frame) => match lagged(frame) {
                        Some(err) => Err(err.into()),
                        None => continue,
                    },
                },
                // A broken connection ends the read with its error too.
                Err(e) => Err(e.into()),
            };
            let end = item.is_err();
            if tx.send(item).await.is_err() || end {
                return;
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
    with_timestamp: bool,
) -> Result<Bytes, crate::Error>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut query_parts = Vec::new();
    if let Some(t) = ttl {
        query_parts.push(t.to_query());
    }
    if with_timestamp {
        query_parts.push("with-timestamp".to_string());
    }
    let query = if query_parts.is_empty() {
        None
    } else {
        Some(query_parts.join("&"))
    };

    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as crate::Error)
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
) -> Result<(), crate::Error>
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
                    let boxed_err: crate::Error = Box::new(e);
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

pub async fn cas_post<R>(addr: &str, data: R) -> Result<Bytes, crate::Error>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as crate::Error)
    });
    let body = StreamBody::new(mapped_stream);

    let res = request::request(addr, Method::POST, "cas", None, body, None).await?;
    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn get(addr: &str, id: &str, with_timestamp: bool) -> Result<Bytes, crate::Error> {
    let query = if with_timestamp {
        Some("with-timestamp")
    } else {
        None
    };
    let res = request::request(addr, Method::GET, id, query, empty(), None).await?;
    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn remove(addr: &str, id: &str) -> Result<(), crate::Error> {
    let _ = request::request(addr, Method::DELETE, id, None, empty(), None).await?;
    Ok(())
}

pub async fn last(
    addr: &str,
    topic: Option<&str>,
    last: usize,
    follow: bool,
    with_timestamp: bool,
) -> Result<(), crate::Error> {
    let mut query_parts = Vec::new();
    if last != 1 {
        query_parts.push(format!("last={last}"));
    }
    if follow {
        query_parts.push("follow=true".to_string());
    }
    if with_timestamp {
        query_parts.push("with-timestamp".to_string());
    }
    let query = if query_parts.is_empty() {
        None
    } else {
        Some(query_parts.join("&"))
    };

    let path = match topic {
        Some(t) => format!("last/{t}"),
        None => "last".to_string(),
    };

    let res = request::request(
        addr,
        Method::GET,
        &path,
        query.as_deref(),
        empty(),
        Some(vec![trailers()]),
    )
    .await?;

    let mut body = res.into_body();
    let mut stdout = tokio::io::stdout();

    while let Some(frame) = body.frame().await {
        match frame?.into_data() {
            Ok(chunk) => stdout.write_all(&chunk).await?,
            Err(frame) => {
                if let Some(err) = lagged(frame) {
                    stdout.flush().await?;
                    return Err(err.into());
                }
            }
        }
    }
    stdout.flush().await?;
    Ok(())
}

pub async fn import<R>(addr: &str, data: R) -> Result<Bytes, crate::Error>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let reader_stream = ReaderStream::new(data);
    let mapped_stream = reader_stream.map(|result| {
        result
            .map(hyper::body::Frame::data)
            .map_err(|e| Box::new(e) as crate::Error)
    });
    let body = StreamBody::new(mapped_stream);

    let res = request::request(addr, Method::POST, "import", None, body, None).await?;
    let body = res.collect().await?.to_bytes();
    Ok(body)
}

pub async fn version(addr: &str) -> Result<Bytes, crate::Error> {
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

/// Asks the server for trailers: it sends them only when asked.
fn trailers() -> (String, String) {
    ("TE".to_string(), "trailers".to_string())
}

/// The read's error, if `frame` is the `xs-lagged` trailer (ADR 0008).
fn lagged(frame: hyper::body::Frame<Bytes>) -> Option<ReadError> {
    let trailers = frame.into_trailers().ok()?;
    let skipped = trailers
        .get(crate::api::LAGGED_TRAILER)?
        .to_str()
        .ok()?
        .parse()
        .ok()?;
    Some(ReadError::Lagged { skipped })
}

fn empty() -> BoxBody<Bytes, crate::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

pub async fn eval(addr: &str, script: String) -> Result<(), crate::Error> {
    let res = request::request(
        addr,
        Method::POST,
        "eval",
        None,
        script,
        Some(vec![trailers()]),
    )
    .await?;

    let mut body = res.into_body();
    let mut stdout = tokio::io::stdout();

    while let Some(frame) = body.frame().await {
        match frame?.into_data() {
            Ok(chunk) => stdout.write_all(&chunk).await?,
            Err(frame) => {
                // The script's error, sent after its output (api.rs).
                let error = frame.into_trailers().ok().and_then(|t| {
                    let v = t.get(crate::api::ERROR_TRAILER)?;
                    Some(v.to_str().ok()?.to_string())
                });
                if let Some(error) = error {
                    stdout.flush().await?;
                    return Err(error.into());
                }
            }
        }
    }
    stdout.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lagged_trailer_is_the_read_error() {
        let mut trailers = hyper::HeaderMap::new();
        trailers.insert(crate::api::LAGGED_TRAILER, 506.into());
        assert!(matches!(
            lagged(hyper::body::Frame::trailers(trailers)),
            Some(ReadError::Lagged { skipped: 506 })
        ));
        assert!(lagged(hyper::body::Frame::data(Bytes::from("{}"))).is_none());
    }
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
