use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, UnixListener};
#[cfg(test)]
use tokio::net::{TcpStream, UnixStream};
use iroh::{Endpoint, RelayMode};
use iroh::endpoint::Connection;

pub trait AsyncReadWrite: AsyncRead + AsyncWrite {}

impl<T: AsyncRead + AsyncWrite> AsyncReadWrite for T {}

pub type AsyncReadWriteBox = Box<dyn AsyncReadWrite + Unpin + Send>;

pub struct IrohStream {
    // TODO: Implement proper iroh stream handling
    // For now, use a placeholder that compiles
    _placeholder: (),
}

impl IrohStream {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    pub fn from_connection(_conn: Connection) -> Self {
        // TODO: Wrap the actual iroh connection
        Self { _placeholder: () }
    }
}

impl AsyncRead for IrohStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // For now, return pending - we'll implement this properly later
        Poll::Pending
    }
}

impl AsyncWrite for IrohStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // For now, return pending - we'll implement this properly later
        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub enum Listener {
    Tcp(TcpListener),
    Unix(UnixListener),
    Iroh(Endpoint, String), // Endpoint and ticket
}

impl Listener {
    pub async fn accept(
        &mut self,
    ) -> io::Result<(AsyncReadWriteBox, Option<std::net::SocketAddr>)> {
        match self {
            Listener::Tcp(listener) => {
                let (stream, addr) = listener.accept().await?;
                Ok((Box::new(stream), Some(addr)))
            }
            Listener::Unix(listener) => {
                let (stream, _) = listener.accept().await?;
                Ok((Box::new(stream), None))
            }
            Listener::Iroh(endpoint, _) => {
                // Accept incoming connections
                let incoming = endpoint.accept().await
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No incoming connection"))?;
                
                let conn = incoming.await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                
                let stream = IrohStream::from_connection(conn);
                Ok((Box::new(stream), None))
            }
        }
    }

    pub async fn bind(addr: &str) -> io::Result<Self> {
        if addr.starts_with("iroh://") {
            let endpoint = Endpoint::builder()
                .alpns(vec![b"xs".to_vec()])
                .relay_mode(RelayMode::Default)
                .bind()
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            
            // Generate ticket from endpoint node_addr
            let node_addr = endpoint.node_addr().await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let ticket = format!("{:?}", node_addr);
            
            Ok(Listener::Iroh(endpoint, ticket))
        } else if addr.starts_with('/') || addr.starts_with('.') {
            // attempt to remove the socket unconditionally
            let _ = std::fs::remove_file(addr);
            let listener = UnixListener::bind(addr)?;
            Ok(Listener::Unix(listener))
        } else {
            let mut addr = addr.to_owned();
            if addr.starts_with(':') {
                addr = format!("127.0.0.1{}", addr);
            };
            let listener = TcpListener::bind(addr).await?;
            Ok(Listener::Tcp(listener))
        }
    }

    pub fn get_ticket(&self) -> Option<&str> {
        match self {
            Listener::Iroh(_, ticket) => Some(ticket),
            _ => None,
        }
    }

    #[cfg(test)]
    pub async fn connect(&self) -> io::Result<AsyncReadWriteBox> {
        match self {
            Listener::Tcp(listener) => {
                let stream = TcpStream::connect(listener.local_addr()?).await?;
                Ok(Box::new(stream))
            }
            Listener::Unix(listener) => {
                let stream =
                    UnixStream::connect(listener.local_addr()?.as_pathname().unwrap()).await?;
                Ok(Box::new(stream))
            }
        }
    }
}

impl std::fmt::Display for Listener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Listener::Tcp(listener) => {
                let addr = listener.local_addr().unwrap();
                write!(f, "{}:{}", addr.ip(), addr.port())
            }
            Listener::Unix(listener) => {
                let addr = listener.local_addr().unwrap();
                let path = addr.as_pathname().unwrap();
                write!(f, "{}", path.display())
            }
            Listener::Iroh(_, ticket) => {
                write!(f, "iroh://{}", ticket)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    async fn exercise_listener(addr: &str) {
        let mut listener = Listener::bind(addr).await.unwrap();
        let mut client = listener.connect().await.unwrap();

        let (mut serve, _) = listener.accept().await.unwrap();
        let want = b"Hello from server!";
        serve.write_all(want).await.unwrap();
        drop(serve);

        let mut got = Vec::new();
        client.read_to_end(&mut got).await.unwrap();
        assert_eq!(want.to_vec(), got);
    }

    #[tokio::test]
    async fn test_bind_tcp() {
        exercise_listener(":0").await;
    }

    #[tokio::test]
    async fn test_bind_unix() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.sock");
        let path = path.to_str().unwrap();
        exercise_listener(path).await;
    }
}
