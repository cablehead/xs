use crate::listener::AsyncReadWriteBox;
use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use rustls::RootCertStore;
use std::sync::Arc;
use tokio::net::{TcpStream, UnixStream};
use tokio_rustls::TlsConnector;

use super::types::{BoxError, ConnectionKind, RequestParts};

async fn create_tls_connector() -> Result<TlsConnector, BoxError> {
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

pub async fn connect(parts: &RequestParts) -> Result<AsyncReadWriteBox, BoxError> {
    match &parts.connection {
        ConnectionKind::Unix(path) => {
            let stream = UnixStream::connect(path).await?;
            Ok(Box::new(stream))
        }
        ConnectionKind::Tcp { host, port } => {
            let stream = TcpStream::connect((host.as_str(), *port)).await?;
            Ok(Box::new(stream))
        }
        ConnectionKind::Tls { host, port } => {
            let tcp_stream = TcpStream::connect((host.as_str(), *port)).await?;
            let connector = create_tls_connector().await?;
            let server_name = ServerName::try_from(host.clone())?; // Clone the host string
            let tls_stream = connector.connect(server_name, tcp_stream).await?;
            Ok(Box::new(tls_stream))
        }
    }
}
