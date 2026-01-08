//! Middleware integration for Python bindings
//!
//! This module bridges the Rust middleware pipeline to Python handlers.
//! It ensures Python handlers get the same middleware guarantees as
//! native Rust handlers.
//!
//! ## Middleware Stages
//!
//! 1. **Request ID** - Generate/propagate X-Request-Id (UUID v7)
//! 2. **Tracing** - Initialize trace/span IDs for observability
//! 3. **Identity** - Extract caller identity from headers
//! 4. **Authorization** - (Future: OPA policy evaluation)
//! 5. **Request Validation** - (Future: Themis contract validation)
//! 6. **Response Validation** - (Future: Themis contract validation)
//! 7. **Telemetry** - Emit metrics and logs
//! 8. **Error Normalization** - Consistent error format

use std::collections::HashMap;
use std::time::Instant;

use archimedes_core::RequestId;
// Note: MiddlewareContext is available but we implement our own lightweight version
// for Python bindings to avoid the full middleware pipeline overhead initially.
// This will be upgraded to use the full archimedes-middleware in Phase 2.
use http::{HeaderMap, Method};
use uuid::Uuid;

use crate::context::{PyIdentity, PyRequestContext};

/// Header names used by middleware
pub mod headers {
    /// Request ID header (standard)
    pub const REQUEST_ID: &str = "x-request-id";
    /// W3C Trace Context parent header
    pub const TRACEPARENT: &str = "traceparent";
    /// W3C Trace Context state header
    pub const TRACESTATE: &str = "tracestate";
    /// Caller identity header (JSON-encoded)
    pub const CALLER_IDENTITY: &str = "x-caller-identity";
    /// Operation ID header
    pub const OPERATION_ID: &str = "x-operation-id";
}

/// Middleware processing result
pub struct MiddlewareResult {
    /// The enriched request context for the handler
    pub context: PyRequestContext,
    /// Request ID to include in response
    pub request_id: String,
    /// Trace ID for logging
    pub trace_id: String,
    /// Span ID for logging
    pub span_id: String,
    /// When processing started (for latency metrics)
    pub started_at: Instant,
}

/// Process incoming request through middleware stages
///
/// This function applies all pre-handler middleware stages:
/// 1. Request ID extraction/generation
/// 2. Trace context extraction/generation
/// 3. Identity extraction from headers
///
/// # Arguments
///
/// * `method` - HTTP method
/// * `path` - Request path
/// * `headers` - Request headers
/// * `operation_id` - Resolved operation ID
/// * `path_params` - Extracted path parameters
/// * `query_params` - Parsed query parameters
///
/// # Returns
///
/// A `MiddlewareResult` containing the enriched context
pub fn process_request(
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    operation_id: &str,
    path_params: HashMap<String, String>,
    query_params: HashMap<String, Vec<String>>,
) -> MiddlewareResult {
    let started_at = Instant::now();

    // Stage 1: Request ID extraction/generation
    let request_id = extract_or_generate_request_id(headers);

    // Stage 2: Trace context extraction/generation
    let (trace_id, span_id) = extract_or_generate_trace_context(headers);

    // Stage 3: Identity extraction
    let identity = extract_identity(headers);

    // Convert headers to HashMap for Python
    let headers_map = headers_to_map(headers);

    // Build the Python request context
    let context = PyRequestContext::new(
        operation_id.to_string(),
        method.to_string(),
        path.to_string(),
        path_params,
        query_params,
        headers_map,
        trace_id.clone(),
        span_id.clone(),
        identity,
    );

    MiddlewareResult {
        context,
        request_id: request_id.to_string(),
        trace_id,
        span_id,
        started_at,
    }
}

/// Extract request ID from headers or generate a new UUID v7
fn extract_or_generate_request_id(headers: &HeaderMap) -> RequestId {
    headers
        .get(headers::REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(RequestId::from_uuid)
        .unwrap_or_else(RequestId::new)
}

/// Extract W3C Trace Context or generate new IDs
fn extract_or_generate_trace_context(headers: &HeaderMap) -> (String, String) {
    // Try to extract from traceparent header
    // Format: version-trace_id-parent_id-flags (e.g., "00-abc123...-def456...-01")
    if let Some(traceparent) = headers
        .get(headers::TRACEPARENT)
        .and_then(|v| v.to_str().ok())
    {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() >= 3 {
            let trace_id = parts[1].to_string();
            let span_id = generate_span_id(); // Always generate new span for this request
            return (trace_id, span_id);
        }
    }

    // Generate new trace context
    (generate_trace_id(), generate_span_id())
}

/// Extract caller identity from X-Caller-Identity header
fn extract_identity(headers: &HeaderMap) -> Option<PyIdentity> {
    let identity_json = headers
        .get(headers::CALLER_IDENTITY)
        .and_then(|v| v.to_str().ok())?;

    // Try to parse as JSON
    let json: serde_json::Value = serde_json::from_str(identity_json).ok()?;

    let identity_type = json.get("type").and_then(|v| v.as_str())?;

    match identity_type {
        "spiffe" => {
            let subject = json.get("id").and_then(|v| v.as_str())?.to_string();
            let issuer = json
                .get("trust_domain")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Some(PyIdentity::new(
                subject,
                issuer,
                None,
                None,
                HashMap::new(),
                Vec::new(),
                Vec::new(),
            ))
        }
        "user" => {
            let subject = json.get("user_id").and_then(|v| v.as_str())?.to_string();
            let roles = json
                .get("roles")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            Some(PyIdentity::new(
                subject,
                None,
                None,
                None,
                HashMap::new(),
                roles,
                Vec::new(),
            ))
        }
        "api_key" => {
            let subject = json.get("key_id").and_then(|v| v.as_str())?.to_string();
            Some(PyIdentity::new(
                subject,
                None,
                None,
                None,
                HashMap::new(),
                Vec::new(),
                Vec::new(),
            ))
        }
        _ => None,
    }
}

/// Convert HeaderMap to HashMap for Python
fn headers_to_map(headers: &HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            map.insert(name.to_string(), v.to_string());
        }
    }
    map
}

/// Generate a trace ID (32 hex characters)
fn generate_trace_id() -> String {
    let uuid = Uuid::now_v7();
    // Use both halves of UUID for 128-bit trace ID
    format!("{:032x}", uuid.as_u128())
}

/// Generate a span ID (16 hex characters)
fn generate_span_id() -> String {
    let uuid = Uuid::now_v7();
    // Use lower 64 bits for span ID
    format!("{:016x}", uuid.as_u128() as u64)
}

/// Add middleware headers to response
pub fn add_response_headers(
    response_headers: &mut http::HeaderMap,
    request_id: &str,
    trace_id: &str,
    span_id: &str,
) {
    // Always include request ID in response
    if let Ok(value) = request_id.parse() {
        response_headers.insert(headers::REQUEST_ID, value);
    }

    // Include trace context for observability
    let traceparent = format!("00-{}-{}-01", trace_id, span_id);
    if let Ok(value) = traceparent.parse() {
        response_headers.insert(headers::TRACEPARENT, value);
    }
}

/// Calculate request duration in milliseconds
pub fn request_duration_ms(started_at: Instant) -> f64 {
    started_at.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::HeaderValue;

    // =========================================================================
    // Request ID Tests
    // =========================================================================

    #[test]
    fn test_generate_request_id_when_missing() {
        let headers = HeaderMap::new();
        let id = extract_or_generate_request_id(&headers);
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn test_extract_request_id_from_header() {
        let mut headers = HeaderMap::new();
        let expected_id = "550e8400-e29b-41d4-a716-446655440000";
        headers.insert(headers::REQUEST_ID, HeaderValue::from_static(expected_id));

        let id = extract_or_generate_request_id(&headers);
        assert_eq!(id.to_string(), expected_id);
    }

    #[test]
    fn test_generate_request_id_for_invalid_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            headers::REQUEST_ID,
            HeaderValue::from_static("invalid-uuid"),
        );

        let id = extract_or_generate_request_id(&headers);
        // Should generate new ID, not use invalid one
        assert_ne!(id.to_string(), "invalid-uuid");
    }

    // =========================================================================
    // Trace Context Tests
    // =========================================================================

    #[test]
    fn test_generate_trace_context_when_missing() {
        let headers = HeaderMap::new();
        let (trace_id, span_id) = extract_or_generate_trace_context(&headers);

        assert_eq!(trace_id.len(), 32);
        assert_eq!(span_id.len(), 16);
    }

    #[test]
    fn test_extract_trace_id_from_traceparent() {
        let mut headers = HeaderMap::new();
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        headers.insert(headers::TRACEPARENT, HeaderValue::from_static(traceparent));

        let (trace_id, span_id) = extract_or_generate_trace_context(&headers);

        // Should extract trace ID from header
        assert_eq!(trace_id, "0af7651916cd43dd8448eb211c80319c");
        // Span ID is always generated fresh
        assert_eq!(span_id.len(), 16);
    }

    #[test]
    fn test_generate_trace_context_for_invalid_traceparent() {
        let mut headers = HeaderMap::new();
        headers.insert(headers::TRACEPARENT, HeaderValue::from_static("invalid"));

        let (trace_id, span_id) = extract_or_generate_trace_context(&headers);

        // Should generate new IDs
        assert_eq!(trace_id.len(), 32);
        assert_eq!(span_id.len(), 16);
    }

    // =========================================================================
    // Identity Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_spiffe_identity() {
        let mut headers = HeaderMap::new();
        let identity_json =
            r#"{"type":"spiffe","id":"spiffe://example.com/service","trust_domain":"example.com"}"#;
        headers.insert(
            headers::CALLER_IDENTITY,
            HeaderValue::from_str(identity_json).unwrap(),
        );

        let identity = extract_identity(&headers).unwrap();
        assert_eq!(identity.subject, "spiffe://example.com/service");
        assert_eq!(identity.issuer, Some("example.com".to_string()));
    }

    #[test]
    fn test_extract_user_identity() {
        let mut headers = HeaderMap::new();
        let identity_json = r#"{"type":"user","user_id":"user-123","roles":["admin","developer"]}"#;
        headers.insert(
            headers::CALLER_IDENTITY,
            HeaderValue::from_str(identity_json).unwrap(),
        );

        let identity = extract_identity(&headers).unwrap();
        assert_eq!(identity.subject, "user-123");
        assert!(identity.has_role_rs("admin"));
        assert!(identity.has_role_rs("developer"));
    }

    #[test]
    fn test_extract_api_key_identity() {
        let mut headers = HeaderMap::new();
        let identity_json = r#"{"type":"api_key","key_id":"key-abc123"}"#;
        headers.insert(
            headers::CALLER_IDENTITY,
            HeaderValue::from_str(identity_json).unwrap(),
        );

        let identity = extract_identity(&headers).unwrap();
        assert_eq!(identity.subject, "key-abc123");
    }

    #[test]
    fn test_no_identity_when_header_missing() {
        let headers = HeaderMap::new();
        let identity = extract_identity(&headers);
        assert!(identity.is_none());
    }

    #[test]
    fn test_no_identity_for_invalid_json() {
        let mut headers = HeaderMap::new();
        headers.insert(
            headers::CALLER_IDENTITY,
            HeaderValue::from_static("not json"),
        );

        let identity = extract_identity(&headers);
        assert!(identity.is_none());
    }

    #[test]
    fn test_no_identity_for_unknown_type() {
        let mut headers = HeaderMap::new();
        let identity_json = r#"{"type":"unknown","id":"test"}"#;
        headers.insert(
            headers::CALLER_IDENTITY,
            HeaderValue::from_str(identity_json).unwrap(),
        );

        let identity = extract_identity(&headers);
        assert!(identity.is_none());
    }

    // =========================================================================
    // Full Request Processing Tests
    // =========================================================================

    #[test]
    fn test_process_request_generates_context() {
        let headers = HeaderMap::new();
        let path_params = HashMap::new();
        let query_params = HashMap::new();

        let result = process_request(
            &Method::GET,
            "/users",
            &headers,
            "listUsers",
            path_params,
            query_params,
        );

        assert_eq!(result.context.operation_id, "listUsers");
        assert_eq!(result.context.method, "GET");
        assert_eq!(result.context.path, "/users");
        assert_eq!(result.request_id.len(), 36); // UUID format
        assert_eq!(result.trace_id.len(), 32);
        assert_eq!(result.span_id.len(), 16);
    }

    #[test]
    fn test_process_request_with_identity() {
        let mut headers = HeaderMap::new();
        let identity_json = r#"{"type":"user","user_id":"alice","roles":["admin"]}"#;
        headers.insert(
            headers::CALLER_IDENTITY,
            HeaderValue::from_str(identity_json).unwrap(),
        );

        let result = process_request(
            &Method::POST,
            "/users",
            &headers,
            "createUser",
            HashMap::new(),
            HashMap::new(),
        );

        assert!(result.context.is_authenticated_rs());
        assert_eq!(result.context.subject_rs(), Some("alice".to_string()));
    }

    #[test]
    fn test_process_request_preserves_trace_context() {
        let mut headers = HeaderMap::new();
        let traceparent = "00-abcdef1234567890abcdef1234567890-1234567890abcdef-01";
        headers.insert(headers::TRACEPARENT, HeaderValue::from_static(traceparent));

        let result = process_request(
            &Method::GET,
            "/health",
            &headers,
            "healthCheck",
            HashMap::new(),
            HashMap::new(),
        );

        assert_eq!(result.trace_id, "abcdef1234567890abcdef1234567890");
    }

    // =========================================================================
    // Response Header Tests
    // =========================================================================

    #[test]
    fn test_add_response_headers() {
        let mut headers = HeaderMap::new();
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let trace_id = "abcdef1234567890abcdef1234567890";
        let span_id = "1234567890abcdef";

        add_response_headers(&mut headers, request_id, trace_id, span_id);

        assert_eq!(
            headers.get(headers::REQUEST_ID).unwrap().to_str().unwrap(),
            request_id
        );
        assert!(headers.get(headers::TRACEPARENT).is_some());
    }

    // =========================================================================
    // Duration Tests
    // =========================================================================

    #[test]
    fn test_request_duration_ms() {
        let started_at = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = request_duration_ms(started_at);

        // Should be at least 10ms but not more than 50ms (allowing for test overhead)
        assert!(duration >= 10.0);
        assert!(duration < 50.0);
    }

    // =========================================================================
    // Headers Conversion Tests
    // =========================================================================

    #[test]
    fn test_headers_to_map() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("x-custom", HeaderValue::from_static("custom-value"));

        let map = headers_to_map(&headers);

        assert_eq!(
            map.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(map.get("x-custom"), Some(&"custom-value".to_string()));
    }

    #[test]
    fn test_headers_to_map_empty() {
        let headers = HeaderMap::new();
        let map = headers_to_map(&headers);
        assert!(map.is_empty());
    }
}
