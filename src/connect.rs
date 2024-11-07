use rustls::ClientConfig;
use std::sync::Arc;
use tokio::net::{TcpStream, UnixStream};
use tokio_rustls::TlsConnector;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

async fn create_tls_connector() -> Result<TlsConnector, BoxError> {
    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_native_roots()? // Use native root certificates
        .with_no_client_auth();

    Ok(TlsConnector::from(Arc::new(config)))
}

async fn connect(addr: &str) -> Result<AsyncReadWriteBox, BoxError> {
    if addr.starts_with('/') || addr.starts_with('.') {
        // Unix socket handling remains unchanged
        let path = std::path::Path::new(addr);
        let addr = if path.is_dir() {
            path.join("sock")
        } else {
            path.to_path_buf()
        };
        let stream = UnixStream::connect(addr).await?;
        Ok(Box::new(stream))
    } else if addr.starts_with("https://") {
        // Parse HTTPS URL
        let url = url::Url::parse(addr)?;
        let host = url.host_str().ok_or("Missing host")?;
        let port = url.port().unwrap_or(443);

        // Connect to TCP first
        let tcp_stream = TcpStream::connect((host, port)).await?;

        // Create TLS connector and connect
        let connector = create_tls_connector().await?;
        let tls_stream = connector.connect(host.try_into()?, tcp_stream).await?;

        Ok(Box::new(tls_stream))
    } else {
        // Handle plain HTTP or shortened form
        let addr = if addr.starts_with(':') {
            format!("127.0.0.1{}", addr)
        } else if !addr.contains("://") {
            format!("http://{}", addr)
        } else {
            addr.to_string()
        };

        let url = url::Url::parse(&addr)?;
        let host = url.host_str().ok_or("Missing host")?;
        let port = url.port().unwrap_or(80);

        let stream = TcpStream::connect((host, port)).await?;
        Ok(Box::new(stream))
    }
}

// Update the trait bounds for AsyncReadWriteBox to include TLS streams
pub type AsyncReadWriteBox =
    Box<dyn tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static>;

// Rest of the client.rs implementation remains the same...
