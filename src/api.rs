use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;

use scru128::Scru128Id;

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
        context_id: Scru128Id,
    },
    HeadGet {
        topic: String,
        follow: bool,
        context_id: Scru128Id,
    },
    StreamItemGet(Scru128Id),
    StreamItemRemove(Scru128Id),
    CasGet(ssri::Integrity),
    CasPost,
    Import,
    Version,
    NotFound,
    BadRequest(String),
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

            match ReadOptions::from_query(query) {
                Ok(options) => {
                    Routes::StreamCat {
                        accept_type,
                        options,
                    }
                }
                Err(e) => Routes::BadRequest(e.to_string()),
            }
        }

        (&Method::GET, p) if p.starts_with("/head/") => {
            let topic = p.strip_prefix("/head/").unwrap().to_string();
            let follow = params.contains_key("follow");
            let context_id = match params.get("context") {
                None => crate::store::ZERO_CONTEXT,
                Some(ctx) => match ctx.parse() {
                    Ok(id) => id,
                    Err(e) => return Routes::BadRequest(format!("Invalid context ID: {}", e)),
                },
            };
            Routes::HeadGet { topic, follow, context_id }
        }

        (&Method::GET, p) if p.starts_with("/cas/") => {
            if let Some(hash) = p.strip_prefix("/cas/") {
                match ssri::Integrity::from_str(hash) {
                    Ok(integrity) => Routes::CasGet(integrity),
                    Err(e) => Routes::BadRequest(format!("Invalid CAS hash: {}", e)),
                }
            } else {
                Routes::NotFound
            }
        }

        (&Method::POST, "/cas") => Routes::CasPost,
        (&Method::POST, "/import") => Routes::Import,

        (&Method::GET, p) => match Scru128Id::from_str(p.trim_start_matches('/')) {
            Ok(id) => Routes::StreamItemGet(id),
            Err(e) => Routes::BadRequest(format!("Invalid frame ID: {}", e)),
        },

        (&Method::DELETE, p) => match Scru128Id::from_str(p.trim_start_matches('/')) {
            Ok(id) => Routes::StreamItemRemove(id),
            Err(e) => Routes::BadRequest(format!("Invalid frame ID: {}", e)),
        },

        (&Method::POST, path) if path.starts_with('/') => {
            let topic = path.trim_start_matches('/').to_string();
            let context_id = match params.get("context") {
                None => crate::store::ZERO_CONTEXT,
                Some(ctx) => match ctx.parse() {
                    Ok(id) => id,
                    Err(e) => return Routes::BadRequest(format!("Invalid context ID: {}", e)),
                },
            };

            match TTL::from_query(query) {
                Ok(ttl) => Routes::StreamAppend {
                    topic,
                    ttl: Some(ttl),
                    context_id,
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

        Routes::StreamAppend {
            topic,
            ttl,
            context_id,
        } => handle_stream_append(&mut store, req, topic, ttl, context_id).await,

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

        Routes::HeadGet {
            topic,
            follow,
            context_id,
        } => handle_head_get(&store, &topic, follow, context_id).await,

        Routes::Import => handle_import(&mut store, req.into_body()).await,

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
                "id: {}\ndata: {}\n\n",
                frame.id,
                serde_json::to_string(&frame).unwrap_or_default()
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
    context_id: Scru128Id,
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
        .map(|s| serde_json::from_str(s).map_err(|_| format!("xs-meta isn't valid JSON: {}", s)))
        .transpose()
    {
        Ok(meta) => meta,
        Err(e) => return response_400(e.to_string()),
    };

    let frame = store.append(
        Frame::with_topic(topic)
            .maybe_hash(hash)
            .maybe_meta(meta)
            .maybe_ttl(ttl)
            .context_id(context_id)
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Err(e) = store.append(
        Frame::with_topic("xs.start")
            .maybe_meta(expose.as_ref().map(|e| serde_json::json!({"expose": e})))
            .build(),
    ) {
        tracing::error!("Failed to append xs.start frame: {}", e);
    }

    let path = store.path.join("sock").to_string_lossy().to_string();
    let listener = Listener::bind(&path).await?;

    let mut listeners = vec![listener];

    if let Some(expose) = expose {
        listeners.push(Listener::bind(&expose).await?);
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

async fn handle_head_get(
    store: &Store,
    topic: &str,
    follow: bool,
    context_id: Scru128Id,
) -> HTTPResult {
    let current_head = store.head(topic, context_id);

    if !follow {
        return response_frame_or_404(current_head);
    }

    let rx = store
        .read(
            ReadOptions::builder()
                .follow(FollowOption::On)
                .tail(true)
                .maybe_last_id(current_head.as_ref().map(|f| f.id))
                .build(),
        )
        .await;

    let topic = topic.to_string();
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .filter(move |frame| frame.topic == topic)
        .map(|frame| {
            let mut bytes = serde_json::to_vec(&frame).unwrap();
            bytes.push(b'\n');
            Ok::<_, BoxError>(hyper::body::Frame::data(Bytes::from(bytes)))
        });

    let body = if let Some(frame) = current_head {
        let mut head_bytes = serde_json::to_vec(&frame).unwrap();
        head_bytes.push(b'\n');
        let head_chunk = Ok(hyper::body::Frame::data(Bytes::from(head_bytes)));
        StreamBody::new(futures::stream::once(async { head_chunk }).chain(stream)).boxed()
    } else {
        StreamBody::new(stream).boxed()
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/x-ndjson")
        .body(body)?)
}

async fn handle_import(store: &mut Store, body: hyper::body::Incoming) -> HTTPResult {
    let bytes = body.collect().await?.to_bytes();
    let frame: Frame = match serde_json::from_slice(&bytes) {
        Ok(frame) => frame,
        Err(e) => return response_400(format!("Invalid frame JSON: {}", e)),
    };

    store
        .insert_frame(&frame)
        .map_err(|e| Box::new(e) as BoxError)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_route_head_follow() {
        let headers = hyper::HeaderMap::new();

        assert!(matches!(
            match_route(&Method::GET, "/head/test", &headers, None),
            Routes::HeadGet { topic, follow: false, context_id: _ } if topic == "test"
        ));

        assert!(matches!(
            match_route(&Method::GET, "/head/test", &headers, Some("follow=true")),
            Routes::HeadGet { topic, follow: true, context_id: _ } if topic == "test"
        ));
    }
}
