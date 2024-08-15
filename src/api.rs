use std::error::Error;
use std::str::FromStr;

use scru128::Scru128Id;

use tokio::io::AsyncWriteExt;
use tokio::net::UnixListener;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

// needed to convert async-std AsyncRead/Write to a tokio AsyncRead/Write
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

use http_body_util::StreamBody;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Body;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;

use crate::nu::{value_to_json, Engine};
use crate::store::{ReadOptions, Store};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type HTTPResult = Result<Response<BoxBody<Bytes, BoxError>>, BoxError>;

enum Routes {
    StreamCat,
    StreamAppend,
    StreamItemGet(Scru128Id),
    KvGet(String),
    KvPut(String),
    CasGet(ssri::Integrity),
    PipePost(Scru128Id),
    NotFound,
}

fn match_route(method: &Method, path: &str) -> Routes {
    match (method, path) {
        (&Method::GET, "/") => Routes::StreamCat,
        (&Method::POST, "/") => Routes::StreamAppend,
        (&Method::POST, p) if p.starts_with("/pipe/") => {
            if let Some(id_str) = p.strip_prefix("/pipe/") {
                if let Ok(id) = Scru128Id::from_str(id_str) {
                    return Routes::PipePost(id);
                }
            }
            Routes::NotFound
        }
        (&Method::GET, p) if p.starts_with("/kv/") => {
            if let Some(key) = p.strip_prefix("/kv/") {
                if !key.is_empty() {
                    return Routes::KvGet(key.to_string());
                }
            }
            Routes::NotFound
        }
        (&Method::PUT, p) if p.starts_with("/kv/") => {
            if let Some(key) = p.strip_prefix("/kv/") {
                if !key.is_empty() {
                    return Routes::KvPut(key.to_string());
                }
            }
            Routes::NotFound
        }
        (&Method::GET, p) if p.starts_with("/cas/") => {
            if let Some(hash) = p.strip_prefix("/cas/") {
                if let Ok(integrity) = ssri::Integrity::from_str(hash) {
                    return Routes::CasGet(integrity);
                }
            }
            Routes::NotFound
        }
        (&Method::GET, p) => {
            if let Ok(id) = Scru128Id::from_str(p.trim_start_matches('/')) {
                Routes::StreamItemGet(id)
            } else {
                Routes::NotFound
            }
        }
        _ => Routes::NotFound,
    }
}

async fn handle(
    mut store: Store,
    engine: Engine,
    req: Request<hyper::body::Incoming>,
) -> HTTPResult {
    let method = req.method();
    let path = req.uri().path();

    match match_route(method, path) {
        Routes::StreamCat => {
            let options = match ReadOptions::from_query(req.uri().query()) {
                Ok(opts) => opts,
                Err(err) => return response_400(err.to_string()),
            };

            let rx = store.read(options).await;
            let stream = ReceiverStream::new(rx);
            let stream = stream.map(|frame| {
                let mut encoded = serde_json::to_vec(&frame).unwrap();
                encoded.push(b'\n');
                Ok(hyper::body::Frame::data(bytes::Bytes::from(encoded)))
            });
            let body = StreamBody::new(stream).boxed();
            Ok(Response::new(body))
        }

        Routes::StreamAppend => handle_stream_append(&mut store, req).await,

        Routes::KvGet(key) => {
            let value = store.kv.get(key.as_bytes()).unwrap();
            if let Some(value) = value {
                Ok(Response::new(full(value.to_vec())))
            } else {
                response_404()
            }
        }

        Routes::KvPut(key) => handle_kv_put(store, &key, req.into_body()).await,

        Routes::CasGet(hash) => {
            let reader = store.cas_reader(hash).await?;
            let reader = reader.compat();
            let stream = ReaderStream::new(reader);

            let stream = stream.map(|frame| {
                let frame = frame.unwrap();
                Ok(hyper::body::Frame::data(frame))
            });

            let body = StreamBody::new(stream).boxed();
            Ok(Response::new(body))
        }

        Routes::StreamItemGet(id) => {
            if let Some(frame) = store.get(&id) {
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(full(serde_json::to_string(&frame).unwrap()))?)
            } else {
                response_404()
            }
        }

        Routes::PipePost(id) => handle_pipe_post(&mut store, engine, id, req.into_body()).await,

        Routes::NotFound => response_404(),
    }
}

async fn handle_kv_put(store: Store, key: &str, body: hyper::body::Incoming) -> HTTPResult {
    let value = body.collect().await?.to_bytes();
    store.kv.insert(key.as_bytes(), value.clone()).unwrap();
    Ok(Response::new(full(value)))
}

async fn handle_stream_append(
    store: &mut Store,
    req: Request<hyper::body::Incoming>,
) -> HTTPResult {
    let (parts, mut body) = req.into_parts();

    let hash = if body.is_end_stream() {
        None
    } else {
        let writer = store.cas_writer().await?;
        let mut writer = writer.compat_write();
        while let Some(frame) = body.frame().await {
            let data = frame?.into_data().unwrap();
            writer.write_all(&data).await?;
        }
        let writer = writer.into_inner();
        Some(writer.commit().await?)
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

    let frame = store
        .append(parts.uri.path().trim_start_matches('/'), hash, meta)
        .await;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(full(serde_json::to_string(&frame).unwrap()))?)
}

async fn handle_pipe_post(
    store: &mut Store,
    engine: Engine,
    id: Scru128Id,
    body: hyper::body::Incoming,
) -> HTTPResult {
    let bytes = body.collect().await?.to_bytes();
    let closure_snippet = std::str::from_utf8(&bytes)?;
    let closure = engine.parse_closure(closure_snippet)?;

    if let Some(frame) = store.get(&id) {
        let value = closure.run(frame).await?;
        let json = value_to_json(&value);
        let bytes = serde_json::to_vec(&json)?;

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(full(bytes))?)
    } else {
        response_404()
    }
}

pub async fn serve(
    store: Store,
    engine: Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("starting api: {:?}", &store.path);
    let listener = UnixListener::bind(store.path.join("sock")).unwrap();
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
