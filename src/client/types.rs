use base64::prelude::*;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, PartialEq)]
pub enum ConnectionKind {
    Unix(std::path::PathBuf),
    Tcp { host: String, port: u16 },
    Tls { host: String, port: u16 },
    Iroh { ticket: String },
}

#[derive(Debug, PartialEq)]
pub struct RequestParts {
    pub uri: String,
    pub host: Option<String>,
    pub authorization: Option<String>,
    pub connection: ConnectionKind,
}

impl RequestParts {
    pub fn parse(
        addr: &str,
        path: &str,
        query: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Iroh case
        if addr.starts_with("iroh://") {
            let ticket = addr.strip_prefix("iroh://").unwrap_or(addr);
            return Ok(RequestParts {
                uri: if let Some(q) = query {
                    format!("http://localhost/{path}?{q}")
                } else {
                    format!("http://localhost/{path}")
                },
                host: None,
                authorization: None,
                connection: ConnectionKind::Iroh {
                    ticket: ticket.to_string(),
                },
            });
        }

        // Unix socket case
        if addr.starts_with('/') || addr.starts_with('.') {
            let socket_path = if std::path::Path::new(addr).is_dir() {
                std::path::Path::new(addr).join("sock")
            } else {
                std::path::Path::new(addr).to_path_buf()
            };

            return Ok(RequestParts {
                uri: if let Some(q) = query {
                    format!("http://localhost/{path}?{q}")
                } else {
                    format!("http://localhost/{path}")
                },
                host: None,
                authorization: None,
                connection: ConnectionKind::Unix(socket_path),
            });
        }

        // Normalize URL
        let addr = if addr.starts_with(':') {
            format!("http://127.0.0.1{addr}")
        } else if !addr.contains("://") {
            format!("http://{addr}")
        } else {
            addr.to_string()
        };

        let url = url::Url::parse(&addr)?;
        let scheme = url.scheme();
        let host = url.host_str().ok_or("Missing host")?.to_string();
        let port = url
            .port()
            .unwrap_or(if scheme == "https" { 443 } else { 80 });
        let port_str = if (scheme == "http" && port == 80) || (scheme == "https" && port == 443) {
            "".to_string()
        } else {
            format!(":{port}")
        };

        // Build clean request URI (no auth)
        let uri = if let Some(q) = query {
            format!("{scheme}://{host}{port_str}/{path}?{q}")
        } else {
            format!("{scheme}://{host}{port_str}/{path}")
        };

        // Set auth if present
        let authorization = if let Some(password) = url.password() {
            let credentials = format!("{}:{}", url.username(), password);
            Some(format!(
                "Basic {}",
                base64::prelude::BASE64_STANDARD.encode(credentials)
            ))
        } else if !url.username().is_empty() {
            let credentials = format!("{}:", url.username());
            Some(format!(
                "Basic {}",
                base64::prelude::BASE64_STANDARD.encode(credentials)
            ))
        } else {
            None
        };

        Ok(RequestParts {
            uri,
            host: Some(format!("{host}{port_str}")),
            authorization,
            connection: if scheme == "https" {
                ConnectionKind::Tls { host, port }
            } else {
                ConnectionKind::Tcp { host, port }
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_socket() {
        let parts = RequestParts::parse("./store", "foo", None).unwrap();
        assert_eq!(parts.uri, "http://localhost/foo");
        assert_eq!(parts.host, None);
        assert_eq!(parts.authorization, None);
    }

    #[test]
    fn test_port_only() {
        let parts = RequestParts::parse(":8080", "bar", Some("q=1")).unwrap();
        assert_eq!(parts.uri, "http://127.0.0.1:8080/bar?q=1");
        assert_eq!(parts.host, Some("127.0.0.1:8080".to_string()));
        assert_eq!(parts.authorization, None);
    }

    #[test]
    fn test_https_url_with_auth() {
        let parts = RequestParts::parse("https://user:pass@example.com:400", "", None).unwrap();
        assert_eq!(parts.uri, "https://example.com:400/");
        assert_eq!(parts.host, Some("example.com:400".to_string()));
        assert_eq!(parts.authorization, Some("Basic dXNlcjpwYXNz".to_string()));
    }
}
