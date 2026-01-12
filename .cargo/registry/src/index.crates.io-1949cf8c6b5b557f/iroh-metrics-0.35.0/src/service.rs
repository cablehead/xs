//! Functions to start services that deal with metrics exposed by this crate.

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use hyper::{service::service_fn, Request, Response};
use tokio::{io::AsyncWriteExt as _, net::TcpListener};
use tracing::{debug, error, info, warn};

use crate::{parse_prometheus_metrics, Error, MetricsSource};

type BytesBody = http_body_util::Full<hyper::body::Bytes>;

/// Start a HTTP server to expose metrics .
pub async fn start_metrics_server(
    addr: SocketAddr,
    registry: impl MetricsSource + Clone,
) -> std::io::Result<()> {
    info!("Starting metrics server on {addr}");
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _addr) = listener.accept().await?;
        let io = hyper_util::rt::TokioIo::new(stream);
        let registry = registry.clone();
        tokio::spawn(async move {
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handler(req, registry.clone())))
                .await
            {
                error!("Error serving metrics connection: {err:#}");
            }
        });
    }
}

/// Start a metrics dumper service.
pub async fn start_metrics_dumper(
    path: std::path::PathBuf,
    interval: std::time::Duration,
    registry: impl MetricsSource,
) -> Result<(), crate::Error> {
    info!(file = %path.display(), ?interval, "running metrics dumper");

    let start = Instant::now();

    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .await?;

    let mut file = tokio::io::BufWriter::new(file);

    // Dump metrics once with a header
    dump_metrics(&mut file, &start, &registry, true).await?;
    loop {
        dump_metrics(&mut file, &start, &registry, false).await?;
        tokio::time::sleep(interval).await;
    }
}

/// Start a metrics exporter service.
pub async fn start_metrics_exporter(
    cfg: MetricsExporterConfig,
    registry: impl MetricsSource,
) -> Result<(), Error> {
    let MetricsExporterConfig {
        interval,
        endpoint,
        service_name,
        instance_name,
        username,
        password,
    } = cfg;
    let push_client = reqwest::Client::new();
    let prom_gateway_uri = format!(
        "{}/metrics/job/{}/instance/{}",
        endpoint, service_name, instance_name
    );
    loop {
        tokio::time::sleep(interval).await;
        let buf = registry.encode_openmetrics_to_string()?;
        let mut req = push_client.post(&prom_gateway_uri);
        if let Some(username) = username.clone() {
            req = req.basic_auth(username, Some(password.clone()));
        }
        let res = match req.body(buf).send().await {
            Ok(res) => res,
            Err(e) => {
                warn!("failed to push metrics: {}", e);
                continue;
            }
        };
        match res.status() {
            reqwest::StatusCode::OK => {
                debug!("pushed metrics to gateway");
            }
            _ => {
                warn!("failed to push metrics to gateway: {:?}", res);
                let body = res.text().await.unwrap();
                warn!("error body: {}", body);
            }
        }
    }
}

/// HTTP handler that will respond with the OpenMetrics encoding of our metrics.
#[allow(clippy::unused_async)]
async fn handler(
    _req: Request<hyper::body::Incoming>,
    registry: impl MetricsSource,
) -> Result<Response<BytesBody>, Error> {
    let content = registry.encode_openmetrics_to_string()?;
    let response = Response::builder()
        .header(hyper::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(body_full(content))
        .expect("Failed to build response");

    Ok(response)
}

/// Creates a new [`BytesBody`] with given content.
fn body_full(content: impl Into<hyper::body::Bytes>) -> BytesBody {
    http_body_util::Full::new(content.into())
}

/// Dump metrics to a file.
async fn dump_metrics(
    file: &mut tokio::io::BufWriter<tokio::fs::File>,
    start: &Instant,
    registry: &impl MetricsSource,
    write_header: bool,
) -> Result<(), Error> {
    let m = registry.encode_openmetrics_to_string()?;
    let m = parse_prometheus_metrics(&m);
    let time_since_start = start.elapsed().as_millis() as f64;

    // take the keys from m and sort them
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();

    let mut metrics = String::new();
    if write_header {
        metrics.push_str("time");
        for key in keys.iter() {
            metrics.push(',');
            metrics.push_str(key);
        }
        metrics.push('\n');
    }

    metrics.push_str(&format!("{}", time_since_start));
    for key in keys.iter() {
        let value = m[*key];
        let formatted_value = format!("{:.3}", value);
        metrics.push(',');
        metrics.push_str(&formatted_value);
    }
    metrics.push('\n');

    file.write_all(metrics.as_bytes()).await?;
    file.flush().await?;
    Ok(())
}

/// Configuration for pushing metrics to a remote endpoint.
#[derive(PartialEq, Eq, Debug, Default, serde::Deserialize, Clone)]
pub struct MetricsExporterConfig {
    /// The push interval.
    pub interval: Duration,
    /// The endpoint url for the push metrics collector.
    pub endpoint: String,
    /// The name of the service you're exporting metrics for.
    ///
    /// Generally, `metrics_exporter` is good enough for use
    /// outside of production deployments.
    pub service_name: String,
    /// The name of the instance you're exporting metrics for.
    ///
    /// This should be device-unique. If not, this will sum up
    /// metrics from different devices.
    ///
    /// E.g. `username-laptop`, `username-phone`, etc.
    ///
    /// Another potential scheme with good privacy would be a
    /// keyed blake3 hash of the secret key. (This gives you
    /// an identifier that is as unique as a `NodeID`, but
    /// can't be correlated to `NodeID`s.)
    pub instance_name: String,
    /// The username for basic auth for the push metrics collector.
    pub username: Option<String>,
    /// The password for basic auth for the push metrics collector.
    pub password: String,
}
