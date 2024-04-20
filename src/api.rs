use tokio::net::UnixListener;

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{StatusCode};
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type HTTPResult = Result<Response<BoxBody<Bytes, BoxError>>, BoxError>;

async fn handle(req: Request<hyper::body::Incoming>) -> HTTPResult {
    let preview = "hai".to_string();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(full(preview)))
}

pub async fn serve() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = UnixListener::bind("./sock").unwrap();
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handle(req)))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

// fit our broadened Response body type.
fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
