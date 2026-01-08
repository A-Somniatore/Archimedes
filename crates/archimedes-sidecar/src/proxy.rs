//! HTTP proxy client for forwarding requests to upstream services.

use std::time::Duration;

use bytes::Bytes;
use http::{header::HeaderMap, Method, StatusCode};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::SidecarConfig;
use crate::error::{SidecarError, SidecarResult};
use crate::headers::{filter_headers_for_upstream, PropagatedHeaders};

/// HTTP proxy client for forwarding requests to upstream.
#[derive(Debug, Clone)]
pub struct ProxyClient {
    /// HTTP client.
    client: Client,
    /// Upstream base URL.
    upstream_url: String,
    /// Request timeout.
    timeout: Duration,
}

impl ProxyClient {
    /// Create a new proxy client.
    pub fn new(config: &SidecarConfig) -> SidecarResult<Self> {
        let client = Client::builder()
            .timeout(config.sidecar.upstream_timeout)
            .pool_max_idle_per_host(100)
            .build()
            .map_err(|e| SidecarError::proxy(format!("failed to create client: {e}")))?;

        Ok(Self {
            client,
            upstream_url: config.sidecar.upstream_url.clone(),
            timeout: config.sidecar.upstream_timeout,
        })
    }

    /// Forward a request to the upstream service.
    pub async fn forward(&self, request: ProxyRequest) -> SidecarResult<ProxyResponse> {
        let url = format!("{}{}", self.upstream_url, request.path);

        let mut req_builder = match request.method {
            Method::GET => self.client.get(&url),
            Method::POST => self.client.post(&url),
            Method::PUT => self.client.put(&url),
            Method::DELETE => self.client.delete(&url),
            Method::PATCH => self.client.patch(&url),
            Method::HEAD => self.client.head(&url),
            Method::OPTIONS => self.client.request(Method::OPTIONS, &url),
            _ => {
                return Err(SidecarError::proxy(format!(
                    "unsupported method: {}",
                    request.method
                )));
            }
        };

        // Add filtered headers
        let mut headers = filter_headers_for_upstream(&request.headers);

        // Add propagated headers
        request.propagated.add_to_headers(&mut headers);

        // Set headers on request
        for (name, value) in headers {
            if let Some(name) = name {
                req_builder = req_builder.header(name, value);
            }
        }

        // Add body if present
        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }

        // Send request
        let response = req_builder
            .send()
            .await
            .map_err(|e| SidecarError::upstream(format!("request failed: {e}")))?;

        // Extract response details
        let status = response.status();
        let response_headers = response.headers().clone();

        // Read body
        let body = response
            .bytes()
            .await
            .map_err(|e| SidecarError::upstream(format!("failed to read body: {e}")))?;

        Ok(ProxyResponse {
            status,
            headers: response_headers,
            body,
        })
    }

    /// Get the upstream URL.
    pub fn upstream_url(&self) -> &str {
        &self.upstream_url
    }

    /// Get the timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// Request to be forwarded to upstream.
#[derive(Debug)]
pub struct ProxyRequest {
    /// HTTP method.
    pub method: Method,
    /// Request path (including query string).
    pub path: String,
    /// Request headers.
    pub headers: HeaderMap,
    /// Request body.
    pub body: Option<Bytes>,
    /// Headers to propagate.
    pub propagated: PropagatedHeaders,
}

impl ProxyRequest {
    /// Create a new proxy request.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: HeaderMap::new(),
            body: None,
            propagated: PropagatedHeaders::new(),
        }
    }

    /// Set the request headers.
    #[must_use]
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Set the request body.
    #[must_use]
    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the propagated headers.
    #[must_use]
    pub fn with_propagated(mut self, propagated: PropagatedHeaders) -> Self {
        self.propagated = propagated;
        self
    }

    /// Get the request ID.
    pub fn request_id(&self) -> &str {
        &self.propagated.request_id
    }
}

/// Response from upstream.
#[derive(Debug)]
pub struct ProxyResponse {
    /// HTTP status code.
    pub status: StatusCode,
    /// Response headers.
    pub headers: HeaderMap,
    /// Response body.
    pub body: Bytes,
}

impl ProxyResponse {
    /// Check if the response indicates success.
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Check if the response indicates a client error.
    pub fn is_client_error(&self) -> bool {
        self.status.is_client_error()
    }

    /// Check if the response indicates a server error.
    pub fn is_server_error(&self) -> bool {
        self.status.is_server_error()
    }

    /// Get the response body as a string.
    pub fn body_string(&self) -> Option<String> {
        String::from_utf8(self.body.to_vec()).ok()
    }

    /// Get the response body as JSON.
    pub fn body_json<T: for<'de> Deserialize<'de>>(&self) -> SidecarResult<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| SidecarError::upstream(format!("invalid JSON response: {e}")))
    }

    /// Get a header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Get the content type.
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Get the content length.
    pub fn content_length(&self) -> Option<usize> {
        self.header("content-length").and_then(|v| v.parse().ok())
    }
}

/// Metrics for proxy operations.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProxyMetrics {
    /// Total requests forwarded.
    pub total_requests: u64,
    /// Successful requests (2xx).
    pub successful_requests: u64,
    /// Client errors (4xx).
    pub client_errors: u64,
    /// Server errors (5xx).
    pub server_errors: u64,
    /// Connection errors.
    pub connection_errors: u64,
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Total bytes received.
    pub bytes_received: u64,
}

impl ProxyMetrics {
    /// Record a request result.
    pub fn record(&mut self, response: &ProxyResponse, request_body_len: usize) {
        self.total_requests += 1;
        self.bytes_sent += request_body_len as u64;
        self.bytes_received += response.body.len() as u64;

        if response.is_success() {
            self.successful_requests += 1;
        } else if response.is_client_error() {
            self.client_errors += 1;
        } else if response.is_server_error() {
            self.server_errors += 1;
        }
    }

    /// Record a connection error.
    pub fn record_error(&mut self) {
        self.total_requests += 1;
        self.connection_errors += 1;
    }

    /// Get the success rate (0.0 - 1.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_request() {
        let request = ProxyRequest::new(Method::GET, "/api/users").with_body("test body");

        assert_eq!(request.method, Method::GET);
        assert_eq!(request.path, "/api/users");
        assert!(request.body.is_some());
        assert!(!request.request_id().is_empty());
    }

    #[test]
    fn test_proxy_response_is_success() {
        let response = ProxyResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        };
        assert!(response.is_success());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());
    }

    #[test]
    fn test_proxy_response_is_client_error() {
        let response = ProxyResponse {
            status: StatusCode::BAD_REQUEST,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        };
        assert!(!response.is_success());
        assert!(response.is_client_error());
    }

    #[test]
    fn test_proxy_response_body_string() {
        let response = ProxyResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::from("hello world"),
        };
        assert_eq!(response.body_string(), Some("hello world".to_string()));
    }

    #[test]
    fn test_proxy_metrics() {
        let mut metrics = ProxyMetrics::default();

        let success_response = ProxyResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::from("response"),
        };
        metrics.record(&success_response, 10);

        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.bytes_sent, 10);
        assert_eq!(metrics.bytes_received, 8);

        let error_response = ProxyResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        };
        metrics.record(&error_response, 5);

        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.server_errors, 1);
        assert_eq!(metrics.success_rate(), 0.5);
    }

    #[test]
    fn test_proxy_metrics_error() {
        let mut metrics = ProxyMetrics::default();
        metrics.record_error();

        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.connection_errors, 1);
        assert_eq!(metrics.success_rate(), 0.0);
    }
}
