use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;

use scru128::Scru128Id;

use base64::Engine;

use tokio::io::AsyncWriteExt;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

use http_body_util::StreamBody;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::header::ACCEPT;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;

use crate::listener::Listener;
use crate::nu;
use crate::store::{self, FollowOption, Frame, ReadOptions, Store, TTL};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type HTTPResult = Result<Response<BoxBody<Bytes, BoxError>>, BoxError>;

#[derive(Debug, PartialEq, Clone)]
enum AcceptType {
    Ndjson,
    EventStream,
}

enum Routes {
    StreamCat {
        accept_type: AcceptType,
        options: ReadOptions,
    },
    StreamAppend {
        topic: String,
        ttl: Option<TTL>,
    },
    LastGet {
        topic: Option<String>,
        last: usize,
        follow: bool,
    },
    StreamItemGet(Scru128Id),
    StreamItemRemove(Scru128Id),
    CasGet(ssri::Integrity),
    CasPost,
    Import,
    Eval,
    Version,
    NotFound,
    BadRequest(String),
}

/// Validates an Integrity object to ensure all its hashes are properly formatted
fn validate_integrity(integrity: &ssri::Integrity) -> bool {
    // Check if there are any hashes
    if integrity.hashes.is_empty() {
        return false;
    }

    // For each hash, check if it has a valid base64-encoded digest
    for hash in &integrity.hashes {
        // Check if digest is valid base64 using the modern API
        if base64::engine::general_purpose::STANDARD
            .decode(&hash.digest)
            .is_err()
        {
            return false;
        }
    }

    true
}

fn match_route(
    method: &Method,
    path: &str,
    headers: &hyper::HeaderMap,
    query: Option<&str>,
) -> Routes {
    let params: HashMap<String, String> =
        url::form_urlencoded::parse(query.unwrap_or("").as_bytes())
            .into_owned()
            .collect();

    match (method, path) {
        (&Method::GET, "/version") => Routes::Version,

        (&Method::GET, "/") => {
            let accept_type = match headers.get(ACCEPT) {
                Some(accept) if accept == "text/event-stream" => AcceptType::EventStream,
                _ => AcceptType::Ndjson,
            };

            let options = ReadOptions::from_query(query);

            match options {
                Ok(options) => Routes::StreamCat {
                    accept_type,
                    options,
                },
                Err(e) => Routes::BadRequest(e.to_string()),
            }
        }

        (&Method::GET, "/last") => {
            let follow = params.contains_key("follow");
            let last = params.get("last").and_then(|v| v.parse().ok()).unwrap_or(1);
            Routes::LastGet {
                topic: None,
                last,
                follow,
            }
        }

        (&Method::GET, p) if p.starts_with("/last/") => {
            let topic = p.strip_prefix("/last/").unwrap().to_string();
            let follow = params.contains_key("follow");
            let last = params.get("last").and_then(|v| v.parse().ok()).unwrap_or(1);
            Routes::LastGet {
                topic: Some(topic),
                last,
                follow,
            }
        }

        (&Method::GET, p) if p.starts_with("/cas/") => {
            if let Some(hash) = p.strip_prefix("/cas/") {
                match ssri::Integrity::from_str(hash) {
                    Ok(integrity) => {
                        if validate_integrity(&integrity) {
                            Routes::CasGet(integrity)
                        } else {
                            Routes::BadRequest(format!("Invalid CAS hash format: {hash}"))
                        }
                    }
                    Err(e) => Routes::BadRequest(format!("Invalid CAS hash: {e}")),
                }
            } else {
                Routes::NotFound
            }
        }

        (&Method::POST, "/cas") => Routes::CasPost,
        (&Method::POST, "/import") => Routes::Import,
        (&Method::POST, "/eval") => Routes::Eval,

        (&Method::GET, p) => match Scru128Id::from_str(p.trim_start_matches('/')) {
            Ok(id) => Routes::StreamItemGet(id),
            Err(e) => Routes::BadRequest(format!("Invalid frame ID: {e}")),
        },

        (&Method::DELETE, p) => match Scru128Id::from_str(p.trim_start_matches('/')) {
            Ok(id) => Routes::StreamItemRemove(id),
            Err(e) => Routes::BadRequest(format!("Invalid frame ID: {e}")),
        },

        (&Method::POST, path) if path.starts_with("/append/") => {
            let topic = path.strip_prefix("/append/").unwrap().to_string();

            match TTL::from_query(query) {
                Ok(ttl) => Routes::StreamAppend {
                    topic,
                    ttl: Some(ttl),
                },
                Err(e) => Routes::BadRequest(e.to_string()),
            }
        }

        _ => Routes::NotFound,
    }
}

async fn handle(
    mut store: Store,
    _engine: nu::Engine, // TODO: potentially vestigial, will .process come back?
    req: Request<hyper::body::Incoming>,
) -> HTTPResult {
    let method = req.method();
    let path = req.uri().path();
    let headers = req.headers().clone();
    let query = req.uri().query();

    let res = match match_route(method, path, &headers, query) {
        Routes::Version => handle_version().await,

        Routes::StreamCat {
            accept_type,
            options,
        } => handle_stream_cat(&mut store, options, accept_type).await,

        Routes::StreamAppend { topic, ttl } => {
            handle_stream_append(&mut store, req, topic, ttl).await
        }

        Routes::CasGet(hash) => {
            let reader = store.cas_reader(hash).await?;
            let stream = ReaderStream::new(reader);

            let stream = stream.map(|frame| {
                let frame = frame.unwrap();
                Ok(hyper::body::Frame::data(frame))
            });

            let body = StreamBody::new(stream).boxed();
            Ok(Response::new(body))
        }

        Routes::CasPost => handle_cas_post(&mut store, req.into_body()).await,

        Routes::StreamItemGet(id) => response_frame_or_404(store.get(&id)),

        Routes::StreamItemRemove(id) => handle_stream_item_remove(&mut store, id).await,

        Routes::LastGet {
            topic,
            last,
            follow,
        } => handle_last_get(&store, topic.as_deref(), last, follow).await,

        Routes::Import => handle_import(&mut store, req.into_body()).await,

        Routes::Eval => handle_eval(&store, req.into_body()).await,

        Routes::NotFound => response_404(),
        Routes::BadRequest(msg) => response_400(msg),
    };

    res.or_else(|e| response_500(e.to_string()))
}

async fn handle_stream_cat(
    store: &mut Store,
    options: ReadOptions,
    accept_type: AcceptType,
) -> HTTPResult {
    let rx = store.read(options).await;
    let stream = ReceiverStream::new(rx);

    let accept_type_clone = accept_type.clone();
    let stream = stream.map(move |frame| {
        let bytes = match accept_type_clone {
            AcceptType::Ndjson => {
                let mut encoded = serde_json::to_vec(&frame).unwrap();
                encoded.push(b'\n');
                encoded
            }
            AcceptType::EventStream => format!(
                "id: {id}\ndata: {data}\n\n",
                id = frame.id,
                data = serde_json::to_string(&frame).unwrap_or_default()
            )
            .into_bytes(),
        };
        Ok(hyper::body::Frame::data(Bytes::from(bytes)))
    });

    let body = StreamBody::new(stream).boxed();

    let content_type = match accept_type {
        AcceptType::Ndjson => "application/x-ndjson",
        AcceptType::EventStream => "text/event-stream",
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .body(body)?)
}

async fn handle_stream_append(
    store: &mut Store,
    req: Request<hyper::body::Incoming>,
    topic: String,
    ttl: Option<TTL>,
) -> HTTPResult {
    let (parts, mut body) = req.into_parts();

    let hash = {
        let mut writer = store.cas_writer().await?;
        let mut bytes_written = 0;

        while let Some(frame) = body.frame().await {
            if let Ok(data) = frame?.into_data() {
                writer.write_all(&data).await?;
                bytes_written += data.len();
            }
        }

        if bytes_written > 0 {
            Some(writer.commit().await?)
        } else {
            None
        }
    };

    let meta = match parts
        .headers
        .get("xs-meta")
        .map(|x| x.to_str())
        .transpose()
        .unwrap()
        .map(|s| {
            // First decode the Base64-encoded string
            base64::prelude::BASE64_STANDARD
                .decode(s)
                .map_err(|e| format!("xs-meta isn't valid Base64: {e}"))
                .and_then(|decoded| {
                    // Then parse the decoded bytes as UTF-8 string
                    String::from_utf8(decoded)
                        .map_err(|e| format!("xs-meta isn't valid UTF-8: {e}"))
                        .and_then(|json_str| {
                            // Finally parse the UTF-8 string as JSON
                            serde_json::from_str(&json_str)
                                .map_err(|e| format!("xs-meta isn't valid JSON: {e}"))
                        })
                })
        })
        .transpose()
    {
        Ok(meta) => meta,
        Err(e) => return response_400(e.to_string()),
    };

    let frame = store.append(
        Frame::builder(topic)
            .maybe_hash(hash)
            .maybe_meta(meta)
            .maybe_ttl(ttl)
            .build(),
    )?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(full(serde_json::to_string(&frame).unwrap()))?)
}

async fn handle_cas_post(store: &mut Store, mut body: hyper::body::Incoming) -> HTTPResult {
    let hash = {
        let mut writer = store.cas_writer().await?;
        let mut bytes_written = 0;

        while let Some(frame) = body.frame().await {
            if let Ok(data) = frame?.into_data() {
                writer.write_all(&data).await?;
                bytes_written += data.len();
            }
        }

        if bytes_written == 0 {
            return response_400("Empty body".to_string());
        }

        writer.commit().await?
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(full(hash.to_string()))?)
}

async fn handle_version() -> HTTPResult {
    let version = env!("CARGO_PKG_VERSION");
    let version_info = serde_json::json!({ "version": version });
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(full(serde_json::to_string(&version_info).unwrap()))?)
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
    expose: Option<String>,
) -> Result<(), BoxError> {
    let path = store.path.join("sock").to_string_lossy().to_string();
    let listener = Listener::bind(&path).await?;

    let mut listeners = vec![listener];
    let mut expose_meta = None;

    if let Some(expose) = expose {
        let expose_listener = Listener::bind(&expose).await?;

        // Check if this is an iroh listener and get the ticket
        if let Some(ticket) = expose_listener.get_ticket() {
            expose_meta = Some(serde_json::json!({"expose": format!("iroh://{}", ticket)}));
        } else {
            expose_meta = Some(serde_json::json!({"expose": expose}));
        }

        listeners.push(expose_listener);
    }

    if let Err(e) = store.append(Frame::builder("xs.start").maybe_meta(expose_meta).build()) {
        tracing::error!("Failed to append xs.start frame: {}", e);
    }

    let mut tasks = Vec::new();
    for listener in listeners {
        let store = store.clone();
        let engine = engine.clone();
        let task = tokio::spawn(async move { listener_loop(listener, store, engine).await });
        tasks.push(task);
    }

    // TODO: graceful shutdown and error handling
    // Wait for all listener tasks to complete (or until the first error)
    for task in tasks {
        task.await??;
    }

    Ok(())
}

async fn listener_loop(
    mut listener: Listener,
    store: Store,
    engine: nu::Engine,
) -> Result<(), BoxError> {
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let store = store.clone();
        let engine = engine.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| handle(store.clone(), engine.clone(), req)),
                )
                .await
            {
                // Match against the error kind to selectively ignore `NotConnected` errors
                if let Some(std::io::ErrorKind::NotConnected) = err.source().and_then(|source| {
                    source
                        .downcast_ref::<std::io::Error>()
                        .map(|io_err| io_err.kind())
                }) {
                    // ignore the NotConnected error, hyper's way of saying the client disconnected
                } else {
                    // todo, Handle or log other errors
                    tracing::error!("TBD: {:?}", err);
                }
            }
        });
    }
}

fn response_frame_or_404(frame: Option<store::Frame>) -> HTTPResult {
    if let Some(frame) = frame {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(full(serde_json::to_string(&frame).unwrap()))?)
    } else {
        response_404()
    }
}

async fn handle_stream_item_remove(store: &mut Store, id: Scru128Id) -> HTTPResult {
    match store.remove(&id) {
        Ok(()) => Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(empty())?),
        Err(e) => {
            tracing::error!("Failed to remove item {}: {:?}", id, e);

            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(full("internal-error"))?)
        }
    }
}

async fn handle_last_get(
    store: &Store,
    topic: Option<&str>,
    last: usize,
    follow: bool,
) -> HTTPResult {
    if !follow {
        let options = ReadOptions::builder()
            .last(last)
            .maybe_topic(topic.map(|t| t.to_string()))
            .build();

        let frames: Vec<Frame> = store.read_sync(options).collect();

        if frames.is_empty() {
            return response_404();
        }

        // Format based on request, not result count:
        // - last == 1 (default): JSON object
        // - last > 1: NDJSON (even if fewer frames returned)
        if last == 1 {
            return response_frame_or_404(Some(frames.into_iter().next().unwrap()));
        }

        let mut body = Vec::new();
        for frame in frames {
            serde_json::to_writer(&mut body, &frame).unwrap();
            body.push(b'\n');
        }

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/x-ndjson")
            .body(full(body))?);
    }

    // Follow mode: use ReadOptions::last to get historical + live frames
    // This emits xs.threshold after historical frames
    let rx = store
        .read(
            ReadOptions::builder()
                .last(last)
                .maybe_topic(topic.map(|t| t.to_string()))
                .follow(FollowOption::On)
                .build(),
        )
        .await;

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|frame| {
        let mut bytes = serde_json::to_vec(&frame).unwrap();
        bytes.push(b'\n');
        Ok::<_, BoxError>(hyper::body::Frame::data(Bytes::from(bytes)))
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/x-ndjson")
        .body(StreamBody::new(stream).boxed())?)
}

async fn handle_import(store: &mut Store, body: hyper::body::Incoming) -> HTTPResult {
    let bytes = body.collect().await?.to_bytes();
    let frame: Frame = match serde_json::from_slice(&bytes) {
        Ok(frame) => frame,
        Err(e) => return response_400(format!("Invalid frame JSON: {e}")),
    };

    store.insert_frame(&frame)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(full(serde_json::to_string(&frame).unwrap()))?)
}

fn response_404() -> HTTPResult {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(empty())?)
}

fn response_400(message: String) -> HTTPResult {
    let body = full(message);
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body)?)
}

fn response_500(message: String) -> HTTPResult {
    let body = full(message);
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body)?)
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, BoxError> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn empty() -> BoxBody<Bytes, BoxError> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

async fn handle_eval(store: &Store, body: hyper::body::Incoming) -> HTTPResult {
    // Read the script from the request body
    let bytes = body.collect().await?.to_bytes();
    let script =
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("Invalid UTF-8 in script: {e}"))?;

    // Create nushell engine with store helper commands
    let mut engine =
        nu::Engine::new().map_err(|e| format!("Failed to create nushell engine: {e}"))?;

    // Add core commands
    nu::add_core_commands(&mut engine, store)
        .map_err(|e| format!("Failed to add core commands to engine: {e}"))?;

    // Add streaming commands
    engine
        .add_commands(vec![
            Box::new(nu::commands::cat_stream_command::CatStreamCommand::new(
                store.clone(),
            )),
            Box::new(nu::commands::last_stream_command::LastStreamCommand::new(
                store.clone(),
            )),
            Box::new(nu::commands::append_command::AppendCommand::new(
                store.clone(),
                serde_json::Value::Null,
            )),
        ])
        .map_err(|e| format!("Failed to add streaming commands to engine: {e}"))?;

    // Execute the script
    let result = engine
        .eval(nu_protocol::PipelineData::empty(), script)
        .map_err(|e| format!("Script evaluation failed:\n{e}"))?;

    // Format output based on PipelineData type according to spec
    match result {
        nu_protocol::PipelineData::ByteStream(stream, ..) => {
            // ByteStream → raw bytes with proper streaming using channel pattern
            if let Some(mut reader) = stream.reader() {
                use std::io::Read;

                let (tx, rx) = tokio::sync::mpsc::channel(16);

                // Spawn sync task to read from nushell Reader and send to channel
                std::thread::spawn(move || {
                    let mut buffer = [0u8; 8192];
                    loop {
                        match reader.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                let chunk = Bytes::copy_from_slice(&buffer[..n]);
                                if tx
                                    .blocking_send(Ok(hyper::body::Frame::data(chunk)))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = tx.blocking_send(Err(
                                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                                ));
                                break;
                            }
                        }
                    }
                });

                let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
                let body = StreamBody::new(stream).boxed();
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/octet-stream")
                    .body(body)?)
            } else {
                // No reader available, return empty response
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/octet-stream")
                    .body(empty())?)
            }
        }
        nu_protocol::PipelineData::ListStream(stream, ..) => {
            // ListStream → JSONL stream with proper streaming using channel pattern
            let (tx, rx) = tokio::sync::mpsc::channel(16);

            // Spawn sync task to iterate stream and send JSONL to channel
            std::thread::spawn(move || {
                for value in stream.into_iter() {
                    let json = nu::value_to_json(&value);
                    match serde_json::to_vec(&json) {
                        Ok(mut json_bytes) => {
                            json_bytes.push(b'\n'); // Add newline for JSONL
                            let chunk = Bytes::from(json_bytes);
                            if tx
                                .blocking_send(Ok(hyper::body::Frame::data(chunk)))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.blocking_send(Err(
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                            ));
                            break;
                        }
                    }
                }
            });

            let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
            let body = StreamBody::new(stream).boxed();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/x-ndjson")
                .body(body)?)
        }
        nu_protocol::PipelineData::Value(value, ..) => {
            match &value {
                nu_protocol::Value::String { .. }
                | nu_protocol::Value::Int { .. }
                | nu_protocol::Value::Float { .. }
                | nu_protocol::Value::Bool { .. } => {
                    // Single primitive value → raw text
                    let text = match value {
                        nu_protocol::Value::String { val, .. } => val.clone(),
                        nu_protocol::Value::Int { val, .. } => val.to_string(),
                        nu_protocol::Value::Float { val, .. } => val.to_string(),
                        nu_protocol::Value::Bool { val, .. } => val.to_string(),
                        _ => value.into_string().unwrap_or_else(|_| "".to_string()),
                    };
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "text/plain")
                        .body(full(text))?)
                }
                _ => {
                    // Single structured value → JSON
                    let json = nu::value_to_json(&value);
                    let json_string = serde_json::to_string(&json)
                        .map_err(|e| format!("Failed to serialize JSON: {e}"))?;
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .body(full(json_string))?)
                }
            }
        }
        nu_protocol::PipelineData::Empty => {
            // Empty → nothing
            Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(empty())?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_route_last() {
        let headers = hyper::HeaderMap::new();

        // /last/topic without follow
        assert!(matches!(
            match_route(&Method::GET, "/last/test", &headers, None),
            Routes::LastGet { topic: Some(t), last: 1, follow: false } if t == "test"
        ));

        // /last/topic with follow
        assert!(matches!(
            match_route(&Method::GET, "/last/test", &headers, Some("follow=true")),
            Routes::LastGet { topic: Some(t), last: 1, follow: true } if t == "test"
        ));

        // /last without topic
        assert!(matches!(
            match_route(&Method::GET, "/last", &headers, None),
            Routes::LastGet {
                topic: None,
                last: 1,
                follow: false
            }
        ));

        // /last with last=5
        assert!(matches!(
            match_route(&Method::GET, "/last", &headers, Some("last=5")),
            Routes::LastGet {
                topic: None,
                last: 5,
                follow: false
            }
        ));

        // /last/topic with last=3 and follow
        assert!(matches!(
            match_route(&Method::GET, "/last/test", &headers, Some("last=3&follow=true")),
            Routes::LastGet { topic: Some(t), last: 3, follow: true } if t == "test"
        ));
    }

    #[tokio::test]
    async fn test_handle_last_get_ndjson_format() {
        use crate::store::Store;
        use http_body_util::BodyExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Add one frame
        store
            .append(crate::store::Frame::builder("test").build())
            .unwrap();

        // Request last=1 (default) - should return JSON object (no trailing newline)
        let response = handle_last_get(&store, Some("test"), 1, false)
            .await
            .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            !body_str.ends_with('\n'),
            "Response should be JSON (no trailing newline) when last=1, got: {:?}",
            body_str
        );

        // Request last=3 but only 1 frame exists - should still return NDJSON
        let response = handle_last_get(&store, Some("test"), 3, false)
            .await
            .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_str.ends_with('\n'),
            "Response should be NDJSON (end with newline) when last > 1, got: {:?}",
            body_str
        );

        // Verify full chain: match_route -> dispatch -> handle_last_get
        let headers = hyper::HeaderMap::new();
        let route = match_route(&Method::GET, "/last/test", &headers, Some("last=3"));
        if let Routes::LastGet {
            topic,
            last,
            follow,
        } = route
        {
            assert_eq!(last, 3, "Route should parse last=3");
            let response = handle_last_get(&store, topic.as_deref(), last, follow)
                .await
                .unwrap();
            let body = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8(body.to_vec()).unwrap();
            assert!(
                body_str.ends_with('\n'),
                "Full chain should return NDJSON when last=3, got: {:?}",
                body_str
            );
        } else {
            panic!("Expected Routes::LastGet");
        }
    }

    #[tokio::test]
    async fn test_handle_eval_logic() {
        // Test the core nushell execution logic by testing the engine directly
        use crate::nu::Engine;
        use crate::store::Store;
        use nu_protocol::PipelineData;

        // Create a temporary store for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Create nushell engine with store helper commands
        let mut engine = Engine::new().unwrap();

        // Add core commands
        crate::nu::add_core_commands(&mut engine, &store).unwrap();

        // Add streaming commands
        engine
            .add_commands(vec![
                Box::new(
                    crate::nu::commands::cat_stream_command::CatStreamCommand::new(store.clone()),
                ),
                Box::new(
                    crate::nu::commands::last_stream_command::LastStreamCommand::new(store.clone()),
                ),
                Box::new(crate::nu::commands::append_command::AppendCommand::new(
                    store.clone(),
                    serde_json::Value::Null,
                )),
            ])
            .unwrap();

        // Test simple string expression
        let result = engine
            .eval(PipelineData::empty(), r#""hello world""#.to_string())
            .unwrap();

        match result {
            PipelineData::Value(value, ..) => {
                let text = value.into_string().unwrap();
                assert_eq!(text, "hello world");
            }
            _ => panic!("Expected Value, got {:?}", result),
        }

        // Test simple math expression - result should be an integer
        let result = engine
            .eval(PipelineData::empty(), "2 + 3".to_string())
            .unwrap();

        match result {
            PipelineData::Value(nu_protocol::Value::Int { val, .. }, ..) => {
                assert_eq!(val, 5);
            }
            _ => panic!("Expected Int Value, got {:?}", result),
        }
    }
}
