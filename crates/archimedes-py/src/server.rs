//! HTTP server implementation for Python bindings
//!
//! This module provides a simple HTTP server that routes requests
//! to Python handlers registered via the `@app.handler` decorator.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use pyo3::prelude::*;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::context::PyRequestContext;
use crate::handlers::HandlerRegistry;
use crate::middleware;
use crate::response::PyResponse;

/// HTTP response body type
pub type ResponseBody = Full<Bytes>;

/// HTTP response type
pub type HttpResponse = Response<ResponseBody>;

/// Server state shared across connections
pub struct ServerState {
    /// Handler registry containing Python handlers
    pub handlers: Arc<HandlerRegistry>,

    /// Contract path for validation (future use)
    pub contract_path: Option<String>,
}

/// HTTP server for Python handlers
pub struct PyServer {
    addr: SocketAddr,
    state: Arc<ServerState>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl PyServer {
    /// Create a new server
    pub fn new(
        addr: SocketAddr,
        handlers: Arc<HandlerRegistry>,
        contract_path: Option<String>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            addr,
            state: Arc::new(ServerState {
                handlers,
                contract_path,
            }),
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Run the server until shutdown
    pub async fn run(&self) -> Result<(), ServerError> {
        println!("[archimedes] Binding to {}...", self.addr);

        let listener = TcpListener::bind(self.addr).await.map_err(|e| {
            ServerError::BindError(format!("Failed to bind to {}: {}", self.addr, e))
        })?;

        println!(
            "[archimedes] Archimedes Python server listening on http://{}",
            self.addr
        );

        let state = Arc::clone(&self.state);
        let mut shutdown_rx = self.shutdown_rx.clone();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, remote_addr)) => {
                            println!("[archimedes] Accepted connection from {}", remote_addr);
                            let state = Arc::clone(&state);
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, remote_addr, state).await {
                                    eprintln!("[archimedes] Connection error from {}: {}", remote_addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("[archimedes] Failed to accept connection: {}", e);
                        }
                    }
                }

                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        println!("[archimedes] Shutdown signal received");
                        break;
                    }
                }
            }
        }

        println!("[archimedes] Server stopped");
        Ok(())
    }

    /// Signal the server to stop
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Handle a single TCP connection
async fn handle_connection(
    stream: tokio::net::TcpStream,
    remote_addr: SocketAddr,
    state: Arc<ServerState>,
) -> Result<(), hyper::Error> {
    let io = TokioIo::new(stream);

    let service = service_fn(move |req: Request<Incoming>| {
        let state = Arc::clone(&state);
        async move { handle_request(req, state).await }
    });

    http1::Builder::new().serve_connection(io, service).await
}

/// Handle a single HTTP request
async fn handle_request(
    req: Request<Incoming>,
    state: Arc<ServerState>,
) -> Result<HttpResponse, Infallible> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let headers = req.headers().clone();

    println!("[archimedes] {} {}", method, path);

    // Handle built-in endpoints
    match (method.as_str(), path.as_str()) {
        ("GET", "/health") | ("GET", "/_archimedes/health") => {
            return Ok(health_response());
        }
        ("GET", "/ready") | ("GET", "/_archimedes/ready") => {
            return Ok(ready_response());
        }
        _ => {}
    }

    // Try to match operation from path
    // For now, use a simple path-to-operation mapping
    // In the future, this will use the contract router
    let operation_id = path_to_operation(&path, &method);

    // Check if we have a handler for this operation
    if let Some(operation_id) = operation_id {
        if state.handlers.has(&operation_id) {
            // Read the request body
            let body = match read_body(req).await {
                Ok(body) => body,
                Err(e) => {
                    eprintln!("[archimedes] Failed to read request body: {}", e);
                    return Ok(error_response(
                        StatusCode::BAD_REQUEST,
                        "Failed to read request body",
                    ));
                }
            };

            // Parse the body as JSON if present
            let body_json: Option<serde_json::Value> = if body.is_empty() {
                None
            } else {
                match serde_json::from_slice(&body) {
                    Ok(json) => Some(json),
                    Err(e) => {
                        println!("[archimedes] Failed to parse body as JSON: {}", e);
                        // Try to create a raw string value
                        Some(serde_json::Value::String(
                            String::from_utf8_lossy(&body).to_string(),
                        ))
                    }
                }
            };

            // Invoke the Python handler (with middleware)
            let result =
                invoke_python_handler(&state, &operation_id, &path, &method, &headers, body_json);

            return Ok(result);
        }
    }

    // No handler found
    Ok(error_response(StatusCode::NOT_FOUND, "Not found"))
}

/// Read the entire request body
async fn read_body(req: Request<Incoming>) -> Result<Bytes, hyper::Error> {
    let body = req.into_body();
    let collected = body.collect().await?;
    Ok(collected.to_bytes())
}

/// Invoke a Python handler
fn invoke_python_handler(
    state: &ServerState,
    operation_id: &str,
    path: &str,
    method: &Method,
    headers: &http::HeaderMap,
    body: Option<serde_json::Value>,
) -> HttpResponse {
    // Process request through middleware
    let mw_result = middleware::process_request(
        method,
        path,
        headers,
        operation_id,
        extract_path_params(path, operation_id),
        std::collections::HashMap::new(), // TODO: parse query params
    );

    // Acquire the GIL and invoke the handler
    let (response_json, handler_error) = Python::with_gil(|py| {
        match state
            .handlers
            .invoke(py, operation_id, mw_result.context, body)
        {
            Ok(json) => (Some(json), None),
            Err(e) => (None, Some(format!("{}", e))),
        }
    });

    // Build response
    let mut response = if let Some(json) = response_json {
        json_to_response(&json)
    } else {
        let error_msg = handler_error.unwrap_or_else(|| "Unknown error".to_string());
        eprintln!("[archimedes] Handler error: {}", error_msg);
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Handler error: {}", error_msg),
        )
    };

    // Add middleware headers to response
    middleware::add_response_headers(
        response.headers_mut(),
        &mw_result.request_id,
        &mw_result.trace_id,
        &mw_result.span_id,
    );

    // Log request completion
    let duration_ms = middleware::request_duration_ms(mw_result.started_at);
    println!(
        "[archimedes] {} {} -> {} ({:.2}ms) request_id={}",
        method,
        path,
        response.status().as_u16(),
        duration_ms,
        mw_result.request_id
    );

    response
}

/// Convert a JSON value to an HTTP response
fn json_to_response(json: &serde_json::Value) -> HttpResponse {
    // Check if the response is a PyResponse-style dict
    if let Some(obj) = json.as_object() {
        if let Some(status) = obj.get("status_code").and_then(|v| v.as_u64()) {
            let body = obj.get("body").cloned().unwrap_or(serde_json::Value::Null);
            let body_bytes = serde_json::to_vec(&body).unwrap_or_default();

            return Response::builder()
                .status(status as u16)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(body_bytes)))
                .unwrap();
        }
    }

    // Default: wrap in JSON response
    let body_bytes = serde_json::to_vec(json).unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .unwrap()
}

/// Health check response
fn health_response() -> HttpResponse {
    let body = serde_json::json!({
        "status": "healthy",
        "service": "archimedes-python"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(serde_json::to_vec(&body).unwrap())))
        .unwrap()
}

/// Readiness response
fn ready_response() -> HttpResponse {
    let body = serde_json::json!({
        "status": "ready",
        "service": "archimedes-python"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(serde_json::to_vec(&body).unwrap())))
        .unwrap()
}

/// Error response
fn error_response(status: StatusCode, message: &str) -> HttpResponse {
    let body = serde_json::json!({
        "error": {
            "code": status.as_u16(),
            "message": message
        }
    });

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(serde_json::to_vec(&body).unwrap())))
        .unwrap()
}

/// Map a path and method to an operation ID
///
/// This is a simple implementation that uses path patterns.
/// In the future, this will use the contract router.
fn path_to_operation(path: &str, method: &Method) -> Option<String> {
    // Common REST patterns
    let patterns = [
        // Health endpoints
        (Method::GET, "/health", "healthCheck"),
        (Method::GET, "/_archimedes/health", "healthCheck"),
        // User endpoints (example)
        (Method::GET, "/users", "listUsers"),
        (Method::POST, "/users", "createUser"),
        (Method::GET, "/users/", "getUser"),       // with ID
        (Method::PUT, "/users/", "updateUser"),    // with ID
        (Method::DELETE, "/users/", "deleteUser"), // with ID
    ];

    for (m, pattern, operation) in &patterns {
        if method == m {
            if pattern.ends_with('/') {
                if path.starts_with(pattern) {
                    return Some(operation.to_string());
                }
            } else if path == *pattern {
                return Some(operation.to_string());
            }
        }
    }

    // Fallback: use the path as operation ID (for custom routes)
    // Remove leading slash and replace remaining slashes with underscores
    let operation = path.trim_start_matches('/').replace('/', "_");
    if !operation.is_empty() {
        Some(operation)
    } else {
        None
    }
}

/// Extract path parameters from a URL path
fn extract_path_params(
    path: &str,
    operation_id: &str,
) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();

    // Simple extraction for /resource/{id} patterns
    if operation_id == "getUser" || operation_id == "updateUser" || operation_id == "deleteUser" {
        if let Some(id) = path.strip_prefix("/users/") {
            if !id.is_empty() {
                params.insert("userId".to_string(), id.to_string());
            }
        }
    }

    params
}

/// Server error types
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Failed to bind to address
    #[error("Bind error: {0}")]
    BindError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Path to Operation Mapping Tests
    // =========================================================================

    #[test]
    fn test_path_to_operation() {
        assert_eq!(
            path_to_operation("/health", &Method::GET),
            Some("healthCheck".to_string())
        );
        assert_eq!(
            path_to_operation("/users", &Method::GET),
            Some("listUsers".to_string())
        );
        assert_eq!(
            path_to_operation("/users", &Method::POST),
            Some("createUser".to_string())
        );
        assert_eq!(
            path_to_operation("/users/123", &Method::GET),
            Some("getUser".to_string())
        );
        assert_eq!(
            path_to_operation("/users/abc", &Method::PUT),
            Some("updateUser".to_string())
        );
        assert_eq!(
            path_to_operation("/users/xyz", &Method::DELETE),
            Some("deleteUser".to_string())
        );
    }

    #[test]
    fn test_path_to_operation_internal_health() {
        assert_eq!(
            path_to_operation("/_archimedes/health", &Method::GET),
            Some("healthCheck".to_string())
        );
    }

    #[test]
    fn test_path_to_operation_fallback() {
        // Unknown paths fall back to path-based operation ID
        assert_eq!(
            path_to_operation("/custom/endpoint", &Method::GET),
            Some("custom_endpoint".to_string())
        );
        assert_eq!(
            path_to_operation("/api/v1/resources", &Method::POST),
            Some("api_v1_resources".to_string())
        );
    }

    #[test]
    fn test_path_to_operation_wrong_method() {
        // POST to /health shouldn't match healthCheck (GET only)
        let result = path_to_operation("/health", &Method::POST);
        // Falls back to path-based operation
        assert_eq!(result, Some("health".to_string()));
    }

    // =========================================================================
    // Path Parameter Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_path_params() {
        let params = extract_path_params("/users/123", "getUser");
        assert_eq!(params.get("userId"), Some(&"123".to_string()));

        let params = extract_path_params("/users/abc-def", "updateUser");
        assert_eq!(params.get("userId"), Some(&"abc-def".to_string()));
    }

    #[test]
    fn test_extract_path_params_uuid() {
        let params = extract_path_params("/users/550e8400-e29b-41d4-a716-446655440000", "getUser");
        assert_eq!(
            params.get("userId"),
            Some(&"550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_extract_path_params_empty_for_list() {
        let params = extract_path_params("/users", "listUsers");
        assert!(params.is_empty());
    }

    #[test]
    fn test_extract_path_params_delete() {
        let params = extract_path_params("/users/user-to-delete", "deleteUser");
        assert_eq!(params.get("userId"), Some(&"user-to-delete".to_string()));
    }

    // =========================================================================
    // Response Generation Tests
    // =========================================================================

    #[test]
    fn test_health_response() {
        let response = health_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Check content type header
        let content_type = response.headers().get("content-type").unwrap();
        assert_eq!(content_type, "application/json");
    }

    #[test]
    fn test_ready_response() {
        let response = ready_response();
        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response.headers().get("content-type").unwrap();
        assert_eq!(content_type, "application/json");
    }

    #[test]
    fn test_error_response() {
        let response = error_response(StatusCode::NOT_FOUND, "Not found");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_response_bad_request() {
        let response = error_response(StatusCode::BAD_REQUEST, "Invalid input");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_response_internal_error() {
        let response = error_response(StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong");
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_response_unauthorized() {
        let response = error_response(StatusCode::UNAUTHORIZED, "Authentication required");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_error_response_forbidden() {
        let response = error_response(StatusCode::FORBIDDEN, "Access denied");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    // =========================================================================
    // JSON Response Conversion Tests
    // =========================================================================

    #[test]
    fn test_json_to_response_simple() {
        let json = serde_json::json!({"status": "ok"});
        let response = json_to_response(&json);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_json_to_response_with_status_code() {
        let json = serde_json::json!({
            "status_code": 201,
            "body": {"id": "new-resource"}
        });
        let response = json_to_response(&json);
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[test]
    fn test_json_to_response_not_found() {
        let json = serde_json::json!({
            "status_code": 404,
            "body": {"error": "Resource not found"}
        });
        let response = json_to_response(&json);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_json_to_response_no_content() {
        let json = serde_json::json!({
            "status_code": 204,
            "body": null
        });
        let response = json_to_response(&json);
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    // =========================================================================
    // Server Error Tests
    // =========================================================================

    #[test]
    fn test_server_error_display() {
        let bind_error = ServerError::BindError("Address already in use".to_string());
        assert!(bind_error.to_string().contains("Address already in use"));

        let io_error = ServerError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Connection reset",
        ));
        assert!(io_error.to_string().contains("Connection reset"));
    }

    #[test]
    fn test_server_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let server_err: ServerError = io_err.into();
        assert!(matches!(server_err, ServerError::IoError(_)));
    }
}
