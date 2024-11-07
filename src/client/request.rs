use base64::prelude::*;

#[derive(Default, Debug, PartialEq)]
pub struct RequestParts {
    pub uri: String,
    pub host: Option<String>,
    pub authorization: Option<String>,
}

pub fn parse_request_parts(
    addr: &str,
    path: &str,
    query: Option<&str>,
) -> Result<RequestParts, Box<dyn std::error::Error + Send + Sync>> {
    let mut parts = RequestParts::default();

    // Unix socket case
    if addr.starts_with('/') || addr.starts_with('.') {
        parts.uri = if let Some(q) = query {
            format!("http://localhost/{}?{}", path, q)
        } else {
            format!("http://localhost/{}", path)
        };
        return Ok(parts);
    }

    // Convert port-only or bare host to URL
    let addr = if addr.starts_with(':') {
        format!("http://127.0.0.1{}", addr)
    } else if !addr.contains("://") {
        format!("http://{}", addr)
    } else {
        addr.to_string()
    };

    let url = url::Url::parse(&addr)?;

    // Build the clean request URI (no auth)
    let scheme = url.scheme();
    let host = url.host_str().ok_or("Missing host")?.to_string(); // Convert to owned String
    let port = url.port().map(|p| format!(":{}", p)).unwrap_or_default();

    parts.uri = if let Some(q) = query {
        format!("{}://{}{}/{}?{}", scheme, host, port, path, q)
    } else {
        format!("{}://{}{}/{}", scheme, host, port, path)
    };

    // Set host header
    parts.host = Some(format!("{}{}", host, port));

    // Set auth if present
    if let Some(password) = url.password() {
        let credentials = format!("{}:{}", url.username(), password);
        parts.authorization = Some(format!("Basic {}", BASE64_STANDARD.encode(credentials)));
    } else if !url.username().is_empty() {
        let credentials = format!("{}:", url.username());
        parts.authorization = Some(format!("Basic {}", BASE64_STANDARD.encode(credentials)));
    }

    Ok(parts)
}

use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::{Method, Request};
use hyper_util::rt::TokioIo;

pub async fn request(
    addr: &str,
    method: Method,
    path: &str,
    query: Option<&str>,
    body: BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>,
    headers: Option<Vec<(String, String)>>,
) -> Result<hyper::Response<hyper::body::Incoming>, Box<dyn std::error::Error + Send + Sync>> {
    let stream = super::connect(addr).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let parts = parse_request_parts(addr, path, query)?;

    let mut builder = Request::builder()
        .method(method)
        .uri(parts.uri)
        .header(hyper::header::USER_AGENT, "xs/0.1")
        .header(hyper::header::ACCEPT, "*/*");

    if let Some(host) = parts.host {
        builder = builder.header(hyper::header::HOST, host);
    }
    if let Some(auth) = parts.authorization {
        builder = builder.header(hyper::header::AUTHORIZATION, auth);
    }

    if let Some(extra_headers) = headers {
        for (name, value) in extra_headers {
            builder = builder.header(name, value);
        }
    }

    let req = builder.body(body)?;
    sender.send_request(req).await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_socket() {
        let parts = parse_request_parts("./store", "foo", None).unwrap();
        assert_eq!(parts.uri, "http://localhost/foo");
        assert_eq!(parts.host, None);
        assert_eq!(parts.authorization, None);
    }

    #[test]
    fn test_port_only() {
        let parts = parse_request_parts(":8080", "bar", Some("q=1")).unwrap();
        assert_eq!(parts.uri, "http://127.0.0.1:8080/bar?q=1");
        assert_eq!(parts.host, Some("127.0.0.1:8080".to_string()));
        assert_eq!(parts.authorization, None);
    }

    #[test]
    fn test_https_url_with_auth() {
        let parts = parse_request_parts("https://user:pass@example.com:400", "", None).unwrap();
        assert_eq!(parts.uri, "https://example.com:400/");
        assert_eq!(parts.host, Some("example.com:400".to_string()));
        assert_eq!(parts.authorization, Some("Basic dXNlcjpwYXNz".to_string()));
    }
}
