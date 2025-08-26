use crate::listener::{AsyncReadWriteBox, IrohStream, ALPN, HANDSHAKE};
use iroh::{Endpoint, RelayMode, SecretKey};
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

/// Get the secret key or generate a new one.
/// Uses IROH_SECRET environment variable if available, otherwise generates a new one.
fn get_or_create_secret() -> Result<SecretKey, BoxError> {
    match std::env::var("IROH_SECRET") {
        Ok(secret) => {
            use std::str::FromStr;
            SecretKey::from_str(&secret).map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid secret key: {}", e),
                )) as BoxError
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
            let secret_key = get_or_create_secret()?;

            // Create an iroh endpoint for connecting
            let endpoint = Endpoint::builder()
                .alpns(vec![])
                .relay_mode(RelayMode::Default)
                .secret_key(secret_key)
                .bind()
                .await
                .map_err(|e| Box::new(std::io::Error::other(e)) as BoxError)?;

            // Parse the ticket string to get the NodeTicket, then extract NodeAddr
            let node_ticket: NodeTicket = ticket.parse().map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "Invalid ticket format: {}",
                    e
                ))) as BoxError
            })?;
            let node_addr = node_ticket.node_addr().clone();

            tracing::info!("Connecting to iroh node: {}", node_addr.node_id);

            // Connect to the node using the proper ALPN
            let conn = endpoint
                .connect(node_addr, ALPN)
                .await
                .map_err(|e| Box::new(std::io::Error::other(e)) as BoxError)?;

            tracing::info!("Successfully connected to iroh node");

            // Create a bidirectional stream
            let (mut send_stream, recv_stream) = conn
                .open_bi()
                .await
                .map_err(|e| Box::new(std::io::Error::other(e)) as BoxError)?;

            // Send the handshake (connecting side sends first)
            #[allow(unused_imports)]
            use tokio::io::AsyncWriteExt;
            send_stream
                .write_all(&HANDSHAKE)
                .await
                .map_err(|e| Box::new(std::io::Error::other(e)) as BoxError)?;

            tracing::info!("Handshake sent successfully");

            let stream = IrohStream::new(send_stream, recv_stream);
            Ok(Box::new(stream))
        }
    }
}
