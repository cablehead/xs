use base64::prelude::*;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Check if a string looks like a Windows absolute path (e.g., "C:\..." or "D:\...")
fn is_windows_path(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

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
    /// The core selected by a trailing `/<core>` segment on `addr`, if any.
    /// Sent as the `xs-core` header so the server resolves `store.core(name)`
    /// for this request. See ADR 0008.
    pub core: Option<String>,
}

/// An xs store directory always contains a `fjall/` subdirectory (see the
/// [`store`](crate::store) module docs). Used to tell "this path is a store"
/// from "this path is a store's parent, with a core name as the last
/// segment" without requiring a new separator in the address syntax.
fn looks_like_store_dir(p: &std::path::Path) -> bool {
    p.join("fjall").is_dir()
}

/// Split a trailing `/<core>` segment off a unix-socket-style address.
///
/// `addr` itself wins whenever it already looks like a store (or is an
/// explicit path to a `sock` file): that keeps `xs cat ./not-yet-started`
/// reporting "no store at" instead of misreading the last path segment as a
/// core name. Only when `addr` doesn't look like a store, but its parent
/// does, is the last segment treated as a core.
fn split_unix_core(addr: &str) -> (String, Option<String>) {
    let path = std::path::Path::new(addr);
    if looks_like_store_dir(path) || path.is_file() {
        return (addr.to_string(), None);
    }
    if let Some(pos) = addr.rfind('/') {
        let base = if pos == 0 { "/" } else { &addr[..pos] };
        let tail = &addr[pos + 1..];
        if !tail.is_empty() && looks_like_store_dir(std::path::Path::new(base)) {
            return (base.to_string(), Some(tail.to_string()));
        }
    }
    (addr.to_string(), None)
}

/// Split a trailing `/<core>` segment off a ticket-style address (iroh).
fn split_trailing_segment(s: &str) -> (String, Option<String>) {
    match s.split_once('/') {
        Some((head, tail)) if !tail.is_empty() => (head.to_string(), Some(tail.to_string())),
        _ => (s.to_string(), None),
    }
}

impl RequestParts {
    pub fn parse(
        addr: &str,
        path: &str,
        query: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Iroh case
        if addr.starts_with("iroh://") {
            let rest = addr.strip_prefix("iroh://").unwrap_or(addr);
            let (ticket, core) = split_trailing_segment(rest);
            return Ok(RequestParts {
                uri: if let Some(q) = query {
                    format!("http://localhost/{path}?{q}")
                } else {
                    format!("http://localhost/{path}")
                },
                host: None,
                authorization: None,
                connection: ConnectionKind::Iroh { ticket },
                core,
            });
        }

        // Unix socket case (also handles Windows paths like "C:\...")
        if addr.starts_with('/') || addr.starts_with('.') || is_windows_path(addr) {
            let (base, core) = split_unix_core(addr);
            let base_path = std::path::Path::new(&base);
            let socket_path = if base_path.is_dir() {
                base_path.join("sock")
            } else {
                base_path.to_path_buf()
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
                core,
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
        // A trailing path on the address itself (not the route `path` param
        // passed in above) selects a core, e.g. "host:port/vm".
        let core = {
            let p = url.path().trim_matches('/');
            if p.is_empty() {
                None
            } else {
                Some(p.split('/').next().unwrap_or(p).to_string())
            }
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
            core,
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
