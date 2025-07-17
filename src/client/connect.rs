use crate::listener::{AsyncReadWriteBox, IrohStream};
use iroh::{Endpoint, RelayMode};
use iroh_base::ticket::NodeTicket;
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
        ConnectionKind::Iroh { ticket } => {
            // Create an iroh endpoint for connecting
            let endpoint = Endpoint::builder()
                .alpns(vec![b"xs".to_vec()])
                .relay_mode(RelayMode::Default)
                .bind()
                .await
                .map_err(|e| {
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as BoxError
                })?;

            // Parse the ticket string to get the NodeTicket, then extract NodeAddr
            let node_ticket: NodeTicket = ticket.parse().map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid ticket format: {}", e),
                )) as BoxError
            })?;
            let node_addr = node_ticket.node_addr().clone();

            // Connect to the node
            let conn = endpoint.connect(node_addr, b"xs").await.map_err(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as BoxError
            })?;

            // Create a bidirectional stream
            let stream = IrohStream::from_connection(conn).await.map_err(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as BoxError
            })?;

            Ok(Box::new(stream))
        }
    }
}
