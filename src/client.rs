use tokio::net::{TcpStream, UnixStream};
use tokio::sync::mpsc::Receiver;

use hyper::body::Bytes;
use hyper::client::conn::http1;
use hyper::{Method, Request, StatusCode};
use hyper_util::rt::TokioIo;

use http_body_util::{combinators::BoxBody, BodyExt, Empty};

use crate::listener::AsyncReadWriteBox;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

async fn connect(addr: &str) -> Result<AsyncReadWriteBox, BoxError> {
    if addr.starts_with('/') || addr.starts_with('.') {
        let path = std::path::Path::new(addr);
        let addr = if path.is_dir() {
            path.join("sock")
        } else {
            path.to_path_buf()
        };
        let stream = UnixStream::connect(addr).await?;
        Ok(Box::new(stream))
    } else {
        let addr = if addr.starts_with(':') {
            format!("127.0.0.1{}", addr)
        } else {
            addr.to_string()
        };
        let stream = TcpStream::connect(addr).await?;
        Ok(Box::new(stream))
    }
}

fn empty() -> BoxBody<Bytes, BoxError> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

pub async fn cat(addr: &str, follow: bool) -> Result<Receiver<Bytes>, BoxError> {
    let stream = connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let uri = if follow {
        "http://localhost/?follow=true"
    } else {
        "http://localhost/"
    };

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(empty())?;

    let res = sender.send_request(req).await?;

    if res.status() != StatusCode::OK {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

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
                    // Ignore non-data frames
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
