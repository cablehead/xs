use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use iroh::endpoint::{RecvStream, SendStream};
use iroh::{Endpoint, RelayMode, SecretKey, Watcher};
use iroh_base::ticket::NodeTicket;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;

#[cfg(unix)]
use tokio::net::UnixListener;
#[cfg(unix)]
#[cfg(test)]
use tokio::net::UnixStream;

#[cfg(windows)]
mod win_uds_compat {
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use win_uds::net::{AsyncListener, AsyncStream};

    /// Wrapper to adapt win_uds AsyncStream to tokio's AsyncRead/AsyncWrite
    pub struct WinUnixStream(tokio_util::compat::Compat<AsyncStream>);

    impl WinUnixStream {
        pub async fn connect<P: AsRef<std::path::Path>>(path: P) -> io::Result<Self> {
            use tokio_util::compat::FuturesAsyncReadCompatExt;
            let stream = AsyncStream::connect(path).await?;
            Ok(Self(stream.compat()))
        }
    }

    impl AsyncRead for WinUnixStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for WinUnixStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_shutdown(cx)
        }
    }

    /// Wrapper for win_uds AsyncListener
    pub struct WinUnixListener {
        inner: AsyncListener,
        path: std::path::PathBuf,
    }

    impl WinUnixListener {
        pub fn bind<P: AsRef<std::path::Path>>(path: P) -> io::Result<Self> {
            let path_buf = path.as_ref().to_path_buf();
            Ok(Self {
                inner: AsyncListener::bind(path)?,
                path: path_buf,
            })
        }

        pub async fn accept(&self) -> io::Result<(WinUnixStream, ())> {
            use tokio_util::compat::FuturesAsyncReadCompatExt;
            let (stream, _addr) = self.inner.accept().await?;
            Ok((WinUnixStream(stream.compat()), ()))
        }

        pub fn local_addr(&self) -> io::Result<std::path::PathBuf> {
            Ok(self.path.clone())
        }
    }
}

#[cfg(windows)]
use win_uds_compat::WinUnixListener as UnixListener;
#[cfg(windows)]
pub use win_uds_compat::WinUnixStream;

#[cfg(test)]
use tokio::net::TcpStream;

/// The ALPN for xs protocol.
pub const ALPN: &[u8] = b"XS/1.0";

/// The handshake to send when connecting.
/// The connecting side must send this handshake, the listening side must consume it.
pub const HANDSHAKE: [u8; 5] = *b"xs..!";

/// Get the secret key or generate a new one.
/// Uses IROH_SECRET environment variable if available, otherwise generates a new one.
fn get_or_create_secret() -> io::Result<SecretKey> {
    match std::env::var("IROH_SECRET") {
        Ok(secret) => {
            use std::str::FromStr;
            SecretKey::from_str(&secret).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid secret key: {e}"),
                )
            })
        }
        Err(_) => {
            let key = SecretKey::generate(rand::rngs::OsRng);
            tracing::info!(
                "Generated new secret key: {}",
                data_encoding::HEXLOWER.encode(&key.to_bytes())
            );
            Ok(key)
        }
    }
}

pub trait AsyncReadWrite: AsyncRead + AsyncWrite {}

impl<T: AsyncRead + AsyncWrite> AsyncReadWrite for T {}

pub type AsyncReadWriteBox = Box<dyn AsyncReadWrite + Unpin + Send>;

pub struct IrohStream {
    send_stream: SendStream,
    recv_stream: RecvStream,
}

impl IrohStream {
    pub fn new(send_stream: SendStream, recv_stream: RecvStream) -> Self {
        Self {
            send_stream,
            recv_stream,
        }
    }
}

impl Drop for IrohStream {
    fn drop(&mut self) {
        // Send reset/stop signals to the other side
        self.send_stream.reset(0u8.into()).ok();
        self.recv_stream.stop(0u8.into()).ok();

        tracing::debug!("IrohStream dropped with cleanup");
    }
}

impl AsyncRead for IrohStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.recv_stream).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for IrohStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.send_stream).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => Poll::Ready(Ok(n)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.send_stream).poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.send_stream).poll_shutdown(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
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
                let incoming = endpoint.accept().await.ok_or_else(|| {
                    tracing::error!("No incoming iroh connection available");
                    io::Error::other("No incoming connection")
                })?;

                let conn = incoming.await.map_err(|e| {
                    tracing::error!("Failed to accept iroh connection: {}", e);
                    io::Error::other(format!("Connection failed: {e}"))
                })?;

                let remote_node_id = "unknown"; // We'll use a placeholder for now
                tracing::info!("Got iroh connection from {}", remote_node_id);

                // Wait for the first incoming bidirectional stream
                let (send_stream, mut recv_stream) = conn.accept_bi().await.map_err(|e| {
                    tracing::error!(
                        "Failed to accept bidirectional stream from {}: {}",
                        remote_node_id,
                        e
                    );
                    io::Error::other(format!("Failed to accept stream: {e}"))
                })?;

                tracing::debug!("Accepted bidirectional stream from {}", remote_node_id);

                // Read and verify the handshake
                let mut handshake_buf = [0u8; HANDSHAKE.len()];
                #[allow(unused_imports)]
                use tokio::io::AsyncReadExt;
                recv_stream
                    .read_exact(&mut handshake_buf)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to read handshake from {}: {}", remote_node_id, e);
                        io::Error::other(format!("Failed to read handshake: {e}"))
                    })?;

                if handshake_buf != HANDSHAKE {
                    tracing::error!(
                        "Invalid handshake received from {}: expected {:?}, got {:?}",
                        remote_node_id,
                        HANDSHAKE,
                        handshake_buf
                    );
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid handshake from {remote_node_id}"),
                    ));
                }

                tracing::info!("Handshake verified successfully from {}", remote_node_id);

                let stream = IrohStream::new(send_stream, recv_stream);
                Ok((Box::new(stream), None))
            }
        }
    }

    pub async fn bind(addr: &str) -> io::Result<Self> {
        if addr.starts_with("iroh://") {
            tracing::info!("Binding iroh endpoint");

            let secret_key = get_or_create_secret()?;
            let endpoint = Endpoint::builder()
                .alpns(vec![ALPN.to_vec()])
                .relay_mode(RelayMode::Default)
                .secret_key(secret_key)
                .bind()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to bind iroh endpoint: {}", e);
                    io::Error::other(format!("Failed to bind endpoint: {e}"))
                })?;

            tracing::debug!("Iroh endpoint bound successfully");

            // Wait for the endpoint to be fully ready before creating ticket
            endpoint.home_relay().initialized().await;
            let node_addr = endpoint.node_addr().initialized().await;

            // Create a proper NodeTicket
            let ticket = NodeTicket::new(node_addr.clone()).to_string();

            tracing::info!("Iroh endpoint ready with node ID: {}", node_addr.node_id);
            tracing::info!("Iroh ticket: {}", ticket);

            Ok(Listener::Iroh(endpoint, ticket))
        } else if addr.starts_with('/') || addr.starts_with('.') {
            // attempt to remove the socket unconditionally
            let _ = std::fs::remove_file(addr);
            let listener = UnixListener::bind(addr)?;
            Ok(Listener::Unix(listener))
        } else {
            let mut addr = addr.to_owned();
            if addr.starts_with(':') {
                addr = format!("127.0.0.1{addr}");
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
                #[cfg(unix)]
                {
                    let stream =
                        UnixStream::connect(listener.local_addr()?.as_pathname().unwrap()).await?;
                    Ok(Box::new(stream))
                }
                #[cfg(windows)]
                {
                    let path = listener.local_addr()?;
                    let stream = WinUnixStream::connect(&path).await?;
                    Ok(Box::new(stream))
                }
            }
            Listener::Iroh(_, ticket) => {
                let secret_key = get_or_create_secret()?;

                // Create a client endpoint
                let client_endpoint = Endpoint::builder()
                    .alpns(vec![])
                    .relay_mode(RelayMode::Default)
                    .secret_key(secret_key)
                    .bind()
                    .await
                    .map_err(io::Error::other)?;

                // Parse ticket to get node address
                let node_ticket: NodeTicket = ticket
                    .parse()
                    .map_err(|e| io::Error::other(format!("Invalid ticket: {}", e)))?;
                let node_addr = node_ticket.node_addr().clone();

                // Connect to the server
                let conn = client_endpoint
                    .connect(node_addr, ALPN)
                    .await
                    .map_err(io::Error::other)?;

                // Open bidirectional stream
                let (mut send_stream, recv_stream) =
                    conn.open_bi().await.map_err(io::Error::other)?;

                // Send handshake
                #[allow(unused_imports)]
                use tokio::io::AsyncWriteExt;
                send_stream
                    .write_all(&HANDSHAKE)
                    .await
                    .map_err(io::Error::other)?;

                let stream = IrohStream::new(send_stream, recv_stream);
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
                #[cfg(unix)]
                {
                    let addr = listener.local_addr().unwrap();
                    let path = addr.as_pathname().unwrap();
                    write!(f, "{}", path.display())
                }
                #[cfg(windows)]
                {
                    let path = listener.local_addr().unwrap();
                    write!(f, "{}", path.display())
                }
            }
            Listener::Iroh(_, ticket) => {
                write!(f, "iroh://{ticket}")
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

    #[tokio::test]
    #[ignore] // Skip by default due to network requirements
    async fn test_bind_iroh() {
        // This test may take longer due to network setup
        exercise_listener("iroh://").await;
    }
}
