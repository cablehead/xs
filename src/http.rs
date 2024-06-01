use std::collections::HashMap;
use std::error::Error;

use serde::{Deserialize, Serialize};

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
    pub stamp: scru128::Scru128Id,
    pub message: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,
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

async fn handle(_store: Store, req: hyper::Request<hyper::body::Incoming>) -> HTTPResult {
    eprintln!("\n\nreq: {:?}", &req);
    Ok(hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(full("Hello world.\n".to_string()))?)
}

pub async fn serve(
    store: Store,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("starting http interface: {:?}", addr);
    let mut listener = Listener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let store = store.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handle(store.clone(), req)))
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
