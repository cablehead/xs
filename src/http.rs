use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;

use scru128::Scru128Id;

use serde::{Deserialize, Serialize};

use tokio::io::AsyncWriteExt;

use tokio_stream::{Stream, StreamExt};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::compat::FuturesAsyncWriteCompatExt;
use tokio_util::io::ReaderStream;

use http_body_util::StreamBody;
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Body;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;

use crate::listener::Listener;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

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
pub struct ResponseMeta {
    pub request_id: Scru128Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<bool>,
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

    let hash = if body.is_end_stream() {
        None
    } else {
        let writer = store.cas_writer().await?;

        // convert writer from async-std -> tokio
        let mut writer = writer.compat_write();
        while let Some(frame) = body.frame().await {
            let data = frame?.into_data().unwrap();
            writer.write_all(&data).await?;
        }
        // get the original writer back
        let writer = writer.into_inner();
        Some(writer.commit().await?)
    };

    let frame = store
        .append(
            Frame::builder()
                .topic("http.request".to_string())
                .maybe_hash(hash)
                .maybe_meta(serde_json::to_value(&req_meta).ok())
                .build(),
        )
        .await;

    let (meta, hashes) = wait_for_response(&store, frame.id).await.unwrap();

    let res = hyper::Response::builder();
    let mut res = res.status(meta.status.unwrap_or(200));
    {
        let res_headers = res.headers_mut().unwrap();
        if let Some(headers) = meta.headers {
            for (key, value) in headers {
                res_headers.insert(
                    http::header::HeaderName::from_bytes(key.as_bytes()).unwrap(),
                    http::header::HeaderValue::from_bytes(value.as_bytes()).unwrap(),
                );
            }
        }

        if !res_headers.contains_key("content-type") {
            res_headers.insert("content-type", "text/plain".parse().unwrap());
        }
    }

    let stream = transform_hash_stream(store.clone(), hashes).await;
    let body = StreamBody::new(stream).boxed();
    Ok(res.body(body)?)
}

pub async fn serve(
    store: Store,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("starting http interface: {:?}", addr);
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
                    tracing::error!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}

use tokio_stream::wrappers::ReceiverStream;

async fn wait_for_response(
    store: &Store,
    frame_id: Scru128Id,
) -> Result<(ResponseMeta, impl Stream<Item = ssri::Integrity>), &'static str> {
    let recver = store
        .read(
            ReadOptions::builder()
                .follow(FollowOption::On)
                .last_id(frame_id)
                .build(),
        )
        .await;

    let mut stream = ReceiverStream::new(recver)
        .filter(|frame| frame.topic == "http.response")
        .filter_map(move |frame| {
            frame.meta.and_then(|meta| {
                serde_json::from_value::<ResponseMeta>(meta.clone())
                    .ok()
                    .and_then(|res| {
                        if res.request_id == frame_id {
                            Some((frame.hash, res))
                        } else {
                            None
                        }
                    })
            })
        });

    if let Some((first_hash, meta)) = stream.next().await {
        let hash_stream = tokio_stream::once((first_hash, meta.clone()))
            .chain(stream)
            .take_while_inclusive(|(_, meta)| meta.more.unwrap_or(false))
            .filter_map(|(hash, _)| hash);
        Ok((meta, hash_stream))
    } else {
        Err("timeout")
    }
}

type ResultFrame = Result<hyper::body::Frame<bytes::Bytes>, Box<dyn Error + Send + Sync>>;

async fn transform_hash_stream(
    store: Store,
    hash_stream: impl futures::Stream<Item = ssri::Integrity>,
) -> impl futures::Stream<Item = ResultFrame> {
    let mapped_stream = hash_stream.then(move |hash| {
        let store = store.clone();
        async move {
            match store.cas_reader(hash).await {
                Ok(reader) => {
                    let reader = reader.compat();
                    let stream = ReaderStream::new(reader);
                    Ok::<_, Box<dyn Error + Send + Sync>>(futures::StreamExt::map(
                        stream,
                        |frame| {
                            let frame = frame.unwrap();
                            Ok(hyper::body::Frame::data(frame))
                        },
                    ))
                }
                Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync>),
            }
        }
    });

    futures::stream::TryStreamExt::try_flatten(mapped_stream)
}

use std::pin::Pin;
use std::task::{Context, Poll};

pub struct TakeWhileInclusive<St, F> {
    stream: St,
    predicate: F,
    done: bool,
}

impl<St, F> TakeWhileInclusive<St, F>
where
    St: Stream,
    F: FnMut(&St::Item) -> bool,
{
    pub fn new(stream: St, predicate: F) -> Self {
        Self {
            stream,
            predicate,
            done: false,
        }
    }
}

impl<St, F> Stream for TakeWhileInclusive<St, F>
where
    St: Stream,
    F: FnMut(&St::Item) -> bool,
{
    type Item = St::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        let this = unsafe { self.as_mut().get_unchecked_mut() };
        let stream = unsafe { Pin::new_unchecked(&mut this.stream) };

        match stream.poll_next(cx) {
            Poll::Ready(Some(item)) => {
                let keep = (this.predicate)(&item);
                if !keep {
                    this.done = true;
                }
                Poll::Ready(Some(item))
            }
            other => other,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.stream.size_hint();
        (0, upper)
    }
}

pub trait TakeWhileInclusiveExt: Stream + Sized {
    fn take_while_inclusive<F>(self, predicate: F) -> TakeWhileInclusive<Self, F>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        TakeWhileInclusive::new(self, predicate)
    }
}

impl<T: Stream> TakeWhileInclusiveExt for T {}
