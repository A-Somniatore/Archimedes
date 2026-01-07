//! Sidecar HTTP server implementation.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn, Instrument};

use crate::config::SidecarConfig;
use crate::error::{ErrorResponse, SidecarError, SidecarResult};
use crate::health::HealthChecker;
use crate::headers::PropagatedHeaders;
use crate::proxy::{ProxyClient, ProxyRequest};

/// Sidecar server.
pub struct SidecarServer {
    /// Configuration.
    config: Arc<SidecarConfig>,
    /// Proxy client.
    proxy: Arc<ProxyClient>,
    /// Health checker.
    health: Arc<HealthChecker>,
}

impl SidecarServer {
    /// Create a new sidecar server.
    pub fn new(config: SidecarConfig) -> SidecarResult<Self> {
        let config = Arc::new(config);
        let proxy = Arc::new(ProxyClient::new(&config)?);
        let health = Arc::new(HealthChecker::new(config.clone()));

        Ok(Self {
            config,
            proxy,
            health,
        })
    }

    /// Run the sidecar server.
    pub async fn run(self) -> SidecarResult<()> {
        let addr = SocketAddr::new(
            self.config
                .sidecar
                .listen_addr
                .parse()
                .map_err(|e| SidecarError::config(format!("invalid listen address: {e}")))?,
            self.config.sidecar.listen_port,
        );

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| SidecarError::server(format!("failed to bind: {e}")))?;

        info!("Archimedes sidecar listening on {}", addr);
        info!(
            "Proxying to upstream: {}",
            self.config.sidecar.upstream_url
        );

        // Mark as ready
        self.health.set_ready(true);

        // Accept connections
        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            let config = self.config.clone();
            let proxy = self.proxy.clone();
            let health = self.health.clone();

            // Spawn handler for this connection
            tokio::spawn(async move {
                let io = TokioIo::new(stream);

                let service = service_fn(move |req| {
                    let config = config.clone();
                    let proxy = proxy.clone();
                    let health = health.clone();
                    async move {
                        handle_request(req, config, proxy, health, peer_addr)
                            .await
                            .map_err(|_| -> Infallible { unreachable!() })
                    }
                });

                if let Err(e) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    debug!("Connection error: {}", e);
                }
            });
        }
    }
}

/// Handle an incoming request.
async fn handle_request(
    req: Request<Incoming>,
    _config: Arc<SidecarConfig>,
    proxy: Arc<ProxyClient>,
    health: Arc<HealthChecker>,
    peer_addr: SocketAddr,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path_and_query().map(ToString::to_string).unwrap_or_else(|| "/".to_string());

    // Generate request ID
    let propagated = PropagatedHeaders::new();
    let request_id = propagated.request_id.clone();

    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %method,
        path = %path,
        peer = %peer_addr,
    );

    async move {
        // Handle internal endpoints
        if path.starts_with("/_archimedes/") {
            return handle_internal_endpoint(&path, &health).await;
        }

        // Extract request body
        let (parts, body) = req.into_parts();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                warn!("Failed to read request body: {}", e);
                return Ok(error_response(
                    StatusCode::BAD_REQUEST,
                    "failed to read request body",
                    &request_id,
                ));
            }
        };

        // Create proxy request
        let proxy_req = ProxyRequest::new(method.clone(), &path)
            .with_headers(parts.headers.clone())
            .with_body(body_bytes.clone())
            .with_propagated(propagated);

        // Forward to upstream
        match proxy.forward(proxy_req).await {
            Ok(response) => {
                let duration = start.elapsed();
                info!(
                    status = %response.status,
                    duration_ms = %duration.as_millis(),
                    "request completed"
                );

                // Build response
                let mut builder = Response::builder().status(response.status);

                // Copy headers (excluding hop-by-hop)
                for (name, value) in &response.headers {
                    if !is_hop_by_hop_header(name.as_str()) {
                        builder = builder.header(name, value);
                    }
                }

                // Add request ID header
                builder = builder.header("x-request-id", &request_id);

                Ok(builder
                    .body(Full::new(response.body))
                    .unwrap_or_else(|_| {
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Full::new(Bytes::from("internal error")))
                            .unwrap()
                    }))
            }
            Err(e) => {
                let duration = start.elapsed();
                error!(
                    error = %e,
                    duration_ms = %duration.as_millis(),
                    "proxy error"
                );

                Ok(error_response(
                    StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_GATEWAY),
                    &e.to_string(),
                    &request_id,
                ))
            }
        }
    }
    .instrument(span)
    .await
}

/// Handle internal sidecar endpoints.
async fn handle_internal_endpoint(
    path: &str,
    health: &HealthChecker,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match path {
        "/_archimedes/health" => {
            let response = health.liveness();
            let status = if response.status.is_operational() {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };

            Ok(json_response(status, &response))
        }
        "/_archimedes/ready" => {
            let response = health.readiness().await;
            let status = if response.status.is_ready() {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };

            Ok(json_response(status, &response))
        }
        "/_archimedes/metrics" => {
            // TODO: Return Prometheus metrics
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/plain")
                .body(Full::new(Bytes::from("# TODO: metrics")))
                .unwrap())
        }
        "/_archimedes/version" => {
            let version = serde_json::json!({
                "version": crate::VERSION,
                "build": env!("CARGO_PKG_VERSION"),
            });

            Ok(json_response(StatusCode::OK, &version))
        }
        _ => Ok(error_response(
            StatusCode::NOT_FOUND,
            &format!("unknown internal endpoint: {path}"),
            "internal",
        )),
    }
}

/// Create a JSON response.
fn json_response<T: serde::Serialize>(status: StatusCode, body: &T) -> Response<Full<Bytes>> {
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("{}")))
                .unwrap()
        })
}

/// Create an error response.
fn error_response(status: StatusCode, message: &str, request_id: &str) -> Response<Full<Bytes>> {
    let error = ErrorResponse::new(status.canonical_reason().unwrap_or("error"), message)
        .with_request_id(request_id);

    json_response(status, &error)
}

/// Check if a header is hop-by-hop (should not be forwarded).
fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_hop_by_hop_header() {
        assert!(is_hop_by_hop_header("connection"));
        assert!(is_hop_by_hop_header("Connection"));
        assert!(is_hop_by_hop_header("transfer-encoding"));
        assert!(!is_hop_by_hop_header("content-type"));
        assert!(!is_hop_by_hop_header("accept"));
    }

    #[test]
    fn test_error_response() {
        let response = error_response(StatusCode::BAD_REQUEST, "test error", "req-123");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let _body = response.into_body();
        // Body contains the error message (we'd need to collect it in async context)
    }

    #[test]
    fn test_json_response() {
        let data = serde_json::json!({"key": "value"});
        let response = json_response(StatusCode::OK, &data);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }
}
