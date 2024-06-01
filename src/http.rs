use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use tokio::io::AsyncWriteExt;

// needed to convert async-std Async to a tokio Async
use tokio_util::compat::FuturesAsyncWriteCompatExt;

use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper_util::rt::TokioIo;

use crate::listener::Listener;
use crate::store::Store;

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub proto: String,
    #[serde(with = "http_serde::method")]
    pub method: http::method::Method,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_ip: Option<std::net::IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<u16>,
    #[serde(with = "http_serde::header_map")]
    pub headers: http::header::HeaderMap,
    #[serde(with = "http_serde::uri")]
    pub uri: http::Uri,
    pub path: String,
    pub query: HashMap<String, String>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type HTTPResult = Result<hyper::Response<BoxBody<Bytes, BoxError>>, BoxError>;

async fn handle(
    mut store: Store,
    req: hyper::Request<hyper::body::Incoming>,
    addr: Option<SocketAddr>,
) -> HTTPResult {
    let (parts, mut body) = req.into_parts();

    let uri = parts.uri.clone().into_parts();

    let authority: Option<String> = uri.authority.as_ref().map(|a| a.to_string()).or_else(|| {
        parts
            .headers
            .get("host")
            .map(|a| a.to_str().unwrap().to_owned())
    });

    let path = parts.uri.path().to_string();

    let query: HashMap<String, String> = parts
        .uri
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new);

    let req_meta = Request {
        proto: format!("{:?}", parts.version),
        method: parts.method,
        authority,
        remote_ip: addr.as_ref().map(|a| a.ip()),
        remote_port: addr.as_ref().map(|a| a.port()),
        headers: parts.headers,
        uri: parts.uri,
        path,
        query,
    };

    let writer = store.cas_writer().await?;
    // convert writer from async-std -> tokio
    let mut writer = writer.compat_write();
    while let Some(frame) = body.frame().await {
        let data = frame?.into_data().unwrap();
        writer.write_all(&data).await?;
    }
    // get the original writer back
    let writer = writer.into_inner();
    let hash = writer.commit().await?;

    let frame = store
        .append(
            "http.request",
            Some(hash),
            Some(serde_json::to_value(&req_meta).unwrap()),
        )
        .await;

    Ok(hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(full(serde_json::to_string(&frame).unwrap()))?)
}

pub async fn serve(
    store: Store,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("starting http interface: {:?}", addr);
    let mut listener = Listener::bind(addr).await?;
    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let store = store.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| handle(store.clone(), req, remote_addr)),
                )
                .await
            {
                // Match against the error kind to selectively ignore `NotConnected` errors
                if let Some(std::io::ErrorKind::NotConnected) = err.source().and_then(|source| {
                    source
                        .downcast_ref::<std::io::Error>()
                        .map(|io_err| io_err.kind())
                }) {
                    // Silently ignore the NotConnected error
                } else {
                    // Handle or log other errors
                    println!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, BoxError> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
