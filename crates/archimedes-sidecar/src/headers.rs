//! Header management for the sidecar proxy.
//!
//! This module handles the propagation of specific headers between the sidecar
//! and the upstream application service.

use http::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use themis_platform_types::CallerIdentity;
use uuid::Uuid;

/// Headers propagated to the upstream application.
#[derive(Debug, Clone)]
pub struct PropagatedHeaders {
    /// Request ID for correlation.
    pub request_id: String,
    /// Trace ID for distributed tracing.
    pub trace_id: Option<String>,
    /// Span ID for current span.
    pub span_id: Option<String>,
    /// Caller identity (JSON encoded).
    pub caller_identity: Option<String>,
    /// Matched operation ID from contract.
    pub operation_id: Option<String>,
}

impl PropagatedHeaders {
    /// Create new propagated headers with a request ID.
    pub fn new() -> Self {
        Self {
            request_id: Uuid::now_v7().to_string(),
            trace_id: None,
            span_id: None,
            caller_identity: None,
            operation_id: None,
        }
    }

    /// Set the trace ID.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Set the span ID.
    #[must_use]
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }

    /// Set the caller identity.
    #[must_use]
    pub fn with_caller_identity(mut self, identity: &CallerIdentity) -> Self {
        if let Ok(json) = serde_json::to_string(identity) {
            self.caller_identity = Some(json);
        }
        self
    }

    /// Set the operation ID.
    #[must_use]
    pub fn with_operation_id(mut self, operation_id: impl Into<String>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    /// Add the propagated headers to a header map.
    pub fn add_to_headers(&self, headers: &mut HeaderMap) {
        // Always add request ID
        if let Ok(value) = HeaderValue::from_str(&self.request_id) {
            headers.insert(HEADER_REQUEST_ID.clone(), value);
        }

        // Add trace ID if present
        if let Some(ref trace_id) = self.trace_id {
            if let Ok(value) = HeaderValue::from_str(trace_id) {
                headers.insert(HEADER_TRACE_ID.clone(), value);
            }
        }

        // Add span ID if present
        if let Some(ref span_id) = self.span_id {
            if let Ok(value) = HeaderValue::from_str(span_id) {
                headers.insert(HEADER_SPAN_ID.clone(), value);
            }
        }

        // Add caller identity if present
        if let Some(ref identity) = self.caller_identity {
            if let Ok(value) = HeaderValue::from_str(identity) {
                headers.insert(HEADER_CALLER_IDENTITY.clone(), value);
            }
        }

        // Add operation ID if present
        if let Some(ref operation_id) = self.operation_id {
            if let Ok(value) = HeaderValue::from_str(operation_id) {
                headers.insert(HEADER_OPERATION_ID.clone(), value);
            }
        }
    }
}

impl Default for PropagatedHeaders {
    fn default() -> Self {
        Self::new()
    }
}

/// Header name for request ID.
pub static HEADER_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Header name for trace ID.
pub static HEADER_TRACE_ID: HeaderName = HeaderName::from_static("x-trace-id");

/// Header name for span ID.
pub static HEADER_SPAN_ID: HeaderName = HeaderName::from_static("x-span-id");

/// Header name for caller identity.
pub static HEADER_CALLER_IDENTITY: HeaderName = HeaderName::from_static("x-caller-identity");

/// Header name for operation ID.
pub static HEADER_OPERATION_ID: HeaderName = HeaderName::from_static("x-operation-id");

/// Headers that should NOT be forwarded to upstream.
pub static FILTERED_HEADERS: &[&str] = &[
    // Security-sensitive headers
    "authorization",
    "cookie",
    "set-cookie",
    // Hop-by-hop headers (HTTP/1.1)
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    // Internal headers
    "x-forwarded-for",
    "x-real-ip",
];

/// Check if a header should be filtered (not forwarded).
pub fn should_filter_header(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    FILTERED_HEADERS.contains(&name_lower.as_str())
}

/// Filter headers for forwarding to upstream.
pub fn filter_headers_for_upstream(headers: &HeaderMap) -> HeaderMap {
    let mut filtered = HeaderMap::new();

    for (name, value) in headers {
        if !should_filter_header(name.as_str()) {
            filtered.insert(name.clone(), value.clone());
        }
    }

    filtered
}

/// Extract trace context from incoming headers.
pub fn extract_trace_context(headers: &HeaderMap) -> Option<TraceContext> {
    // Try W3C Trace Context format first
    if let Some(traceparent) = headers.get("traceparent") {
        if let Ok(value) = traceparent.to_str() {
            return TraceContext::from_traceparent(value);
        }
    }

    // Fallback to custom headers
    let trace_id = headers
        .get(&HEADER_TRACE_ID)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let span_id = headers
        .get(&HEADER_SPAN_ID)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    if trace_id.is_some() || span_id.is_some() {
        Some(TraceContext {
            trace_id: trace_id.unwrap_or_else(|| Uuid::now_v7().to_string()),
            parent_span_id: span_id,
            span_id: Uuid::now_v7().to_string(),
            sampled: true,
        })
    } else {
        None
    }
}

/// W3C Trace Context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Trace ID (32 hex characters).
    pub trace_id: String,
    /// Parent span ID (16 hex characters).
    pub parent_span_id: Option<String>,
    /// Current span ID (16 hex characters).
    pub span_id: String,
    /// Whether this trace is sampled.
    pub sampled: bool,
}

impl TraceContext {
    /// Create a new trace context.
    pub fn new() -> Self {
        Self {
            trace_id: format!("{:032x}", Uuid::now_v7().as_u128()),
            parent_span_id: None,
            span_id: format!("{:016x}", rand_u64()),
            sampled: true,
        }
    }

    /// Parse from W3C traceparent header.
    ///
    /// Format: `{version}-{trace-id}-{parent-id}-{flags}`
    /// Example: `00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01`
    pub fn from_traceparent(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = parts[0];
        if version != "00" {
            return None; // Only support version 00
        }

        let trace_id = parts[1].to_string();
        let parent_span_id = parts[2].to_string();
        let flags = u8::from_str_radix(parts[3], 16).unwrap_or(0);
        let sampled = flags & 0x01 != 0;

        Some(Self {
            trace_id,
            parent_span_id: Some(parent_span_id),
            span_id: format!("{:016x}", rand_u64()),
            sampled,
        })
    }

    /// Format as W3C traceparent header.
    pub fn to_traceparent(&self) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", self.trace_id, self.span_id, flags)
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a random u64 for span IDs.
#[allow(clippy::cast_possible_truncation)]
fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) as u64;
    // Mix in process ID for uniqueness across processes
    nanos ^ u64::from(std::process::id()).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propagated_headers() {
        let headers = PropagatedHeaders::new()
            .with_trace_id("trace-123")
            .with_span_id("span-456")
            .with_operation_id("getUser");

        assert!(!headers.request_id.is_empty());
        assert_eq!(headers.trace_id, Some("trace-123".to_string()));
        assert_eq!(headers.span_id, Some("span-456".to_string()));
        assert_eq!(headers.operation_id, Some("getUser".to_string()));
    }

    #[test]
    fn test_add_to_headers() {
        let propagated = PropagatedHeaders::new()
            .with_trace_id("trace-123")
            .with_operation_id("getUser");

        let mut headers = HeaderMap::new();
        propagated.add_to_headers(&mut headers);

        assert!(headers.contains_key(&HEADER_REQUEST_ID));
        assert!(headers.contains_key(&HEADER_TRACE_ID));
        assert!(headers.contains_key(&HEADER_OPERATION_ID));
    }

    #[test]
    fn test_should_filter_header() {
        assert!(should_filter_header("authorization"));
        assert!(should_filter_header("Authorization"));
        assert!(should_filter_header("cookie"));
        assert!(should_filter_header("transfer-encoding"));
        assert!(!should_filter_header("content-type"));
        assert!(!should_filter_header("accept"));
    }

    #[test]
    fn test_filter_headers_for_upstream() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("authorization", HeaderValue::from_static("Bearer token"));
        headers.insert("accept", HeaderValue::from_static("*/*"));

        let filtered = filter_headers_for_upstream(&headers);
        assert!(filtered.contains_key("content-type"));
        assert!(filtered.contains_key("accept"));
        assert!(!filtered.contains_key("authorization"));
    }

    #[test]
    fn test_trace_context_new() {
        let ctx = TraceContext::new();
        assert_eq!(ctx.trace_id.len(), 32);
        assert_eq!(ctx.span_id.len(), 16);
        assert!(ctx.sampled);
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_trace_context_from_traceparent() {
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = TraceContext::from_traceparent(traceparent).unwrap();

        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_span_id, Some("b7ad6b7169203331".to_string()));
        assert!(ctx.sampled);
    }

    #[test]
    fn test_trace_context_to_traceparent() {
        let ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            parent_span_id: None,
            span_id: "b7ad6b7169203331".to_string(),
            sampled: true,
        };

        let traceparent = ctx.to_traceparent();
        assert!(traceparent.starts_with("00-"));
        assert!(traceparent.ends_with("-01"));
    }

    #[test]
    fn test_extract_trace_context_w3c() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            HeaderValue::from_static("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );

        let ctx = extract_trace_context(&headers).unwrap();
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_extract_trace_context_custom() {
        let mut headers = HeaderMap::new();
        headers.insert(&HEADER_TRACE_ID, HeaderValue::from_static("custom-trace-id"));
        headers.insert(&HEADER_SPAN_ID, HeaderValue::from_static("custom-span-id"));

        let ctx = extract_trace_context(&headers).unwrap();
        assert_eq!(ctx.trace_id, "custom-trace-id");
        assert_eq!(ctx.parent_span_id, Some("custom-span-id".to_string()));
    }
}
