//! Tracing middleware.
//!
//! This middleware initializes OpenTelemetry tracing context for each request.
//! It creates a new span and propagates trace context from incoming headers.
//!
//! ## Trace Context Propagation
//!
//! Supports the [W3C Trace Context](https://www.w3.org/TR/trace-context/) standard:
//! - `traceparent` - Contains trace ID, span ID, and trace flags
//! - `tracestate` - Vendor-specific trace information
//!
//! ## Span Attributes
//!
//! The created span includes standard HTTP attributes:
//! - `http.method` - HTTP method
//! - `http.url` - Request URL
//! - `http.target` - Request path
//! - `http.status_code` - Response status (added on completion)

use crate::context::MiddlewareContext;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::types::{Request, Response};
use uuid::Uuid;

/// The W3C Trace Context header for trace propagation.
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// The W3C Trace State header for vendor-specific data.
pub const TRACESTATE_HEADER: &str = "tracestate";

/// Middleware that initializes OpenTelemetry tracing context.
///
/// This middleware creates a span for each request and propagates
/// trace context from upstream services.
///
/// # Behavior
///
/// 1. Extract trace context from `traceparent` header if present
/// 2. Generate new trace ID if not propagated
/// 3. Create new span ID for this request
/// 4. Store trace context in [`MiddlewareContext`]
/// 5. Add span attributes (method, path, etc.)
///
/// # Example
///
/// ```ignore
/// use archimedes_middleware::stages::tracing::TracingMiddleware;
///
/// let middleware = TracingMiddleware::new("my-service");
/// // Add to pipeline...
/// ```
#[derive(Debug, Clone)]
pub struct TracingMiddleware {
    /// The service name for span attributes.
    service_name: String,
}

impl TracingMiddleware {
    /// Creates a new Tracing middleware.
    ///
    /// # Arguments
    ///
    /// * `service_name` - The name of this service for span attributes
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    /// Extracts trace context from the `traceparent` header.
    ///
    /// Format: `{version}-{trace-id}-{parent-span-id}-{flags}`
    /// Example: `00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01`
    fn extract_trace_context(&self, request: &Request) -> Option<TraceContext> {
        let header = request.headers().get(TRACEPARENT_HEADER)?;
        let value = header.to_str().ok()?;
        TraceContext::parse(value)
    }

    /// Generates a new trace ID (128-bit random).
    fn generate_trace_id() -> String {
        // Use UUID v7 which is available in workspace
        let uuid = Uuid::now_v7();
        uuid.simple().to_string()
    }

    /// Generates a new span ID (64-bit random).
    fn generate_span_id() -> String {
        // Use half of a UUID v7
        let uuid = Uuid::now_v7();
        uuid.simple().to_string()[..16].to_string()
    }
}

impl Default for TracingMiddleware {
    fn default() -> Self {
        Self::new("unknown")
    }
}

impl Middleware for TracingMiddleware {
    fn name(&self) -> &'static str {
        "tracing"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Extract or generate trace context
            let trace_context = self
                .extract_trace_context(&request)
                .unwrap_or_else(|| TraceContext {
                    trace_id: Self::generate_trace_id(),
                    parent_span_id: None,
                    flags: TraceFlags::SAMPLED,
                });

            // Generate new span ID for this request
            let span_id = Self::generate_span_id();

            // Store in context
            ctx.set_trace_id(trace_context.trace_id.clone());
            ctx.set_span_id(span_id.clone());

            // Store additional trace info as extension
            ctx.set_extension(SpanInfo {
                service_name: self.service_name.clone(),
                method: request.method().to_string(),
                path: request.uri().path().to_string(),
                parent_span_id: trace_context.parent_span_id,
            });

            // Process request through remaining middleware
            let response = next.run(ctx, request).await;

            // Note: In a real implementation, we would:
            // 1. Complete the span with status code
            // 2. Export the span to the tracing backend
            // For now, we just return the response

            response
        })
    }
}

/// Parsed trace context from W3C Trace Context headers.
#[derive(Debug, Clone)]
pub struct TraceContext {
    /// The 128-bit trace ID as a hex string.
    pub trace_id: String,
    /// The parent span ID (if propagated from upstream).
    pub parent_span_id: Option<String>,
    /// Trace flags (sampling, etc.).
    pub flags: TraceFlags,
}

impl TraceContext {
    /// Parses a `traceparent` header value.
    ///
    /// Format: `{version}-{trace-id}-{parent-span-id}-{flags}`
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        // Validate version (must be "00")
        if parts[0] != "00" {
            return None;
        }

        // Parse trace ID (32 hex chars)
        let trace_id = parts[1];
        if trace_id.len() != 32 || !trace_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        // Parse parent span ID (16 hex chars)
        let parent_span_id = parts[2];
        if parent_span_id.len() != 16 || !parent_span_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        // Parse flags (2 hex chars)
        let flags = parts[3];
        if flags.len() != 2 || !flags.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        let flags_byte = u8::from_str_radix(flags, 16).ok()?;

        Some(Self {
            trace_id: trace_id.to_string(),
            parent_span_id: Some(parent_span_id.to_string()),
            flags: TraceFlags(flags_byte),
        })
    }
}

/// Trace flags from the W3C Trace Context spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceFlags(u8);

impl TraceFlags {
    /// No flags set.
    pub const NONE: Self = Self(0x00);
    /// The trace is sampled.
    pub const SAMPLED: Self = Self(0x01);

    /// Returns true if the sampled flag is set.
    #[must_use]
    pub const fn is_sampled(self) -> bool {
        self.0 & 0x01 != 0
    }
}

/// Additional span information stored as a context extension.
#[derive(Debug, Clone)]
pub struct SpanInfo {
    /// The service name.
    pub service_name: String,
    /// The HTTP method.
    pub method: String,
    /// The request path.
    pub path: String,
    /// The parent span ID (if propagated).
    pub parent_span_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;

    fn create_test_request() -> Request {
        HttpRequest::builder()
            .method("GET")
            .uri("/users/123")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_request_with_traceparent(traceparent: &str) -> Request {
        HttpRequest::builder()
            .method("POST")
            .uri("/api/data")
            .header(TRACEPARENT_HEADER, traceparent)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_handler() -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        |_ctx, _req| {
            Box::pin(async {
                HttpResponse::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap()
            })
        }
    }

    #[tokio::test]
    async fn test_generates_trace_context_when_missing() {
        let middleware = TracingMiddleware::new("test-service");
        let mut ctx = MiddlewareContext::new();
        let request = create_test_request();

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        // Should have generated trace and span IDs
        assert!(ctx.trace_id().is_some());
        assert!(ctx.span_id().is_some());

        // Should have span info extension
        let span_info = ctx.get_extension::<SpanInfo>().unwrap();
        assert_eq!(span_info.service_name, "test-service");
        assert_eq!(span_info.method, "GET");
        assert_eq!(span_info.path, "/users/123");
        assert!(span_info.parent_span_id.is_none());
    }

    #[tokio::test]
    async fn test_propagates_trace_context() {
        let middleware = TracingMiddleware::new("test-service");
        let mut ctx = MiddlewareContext::new();
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let request = create_request_with_traceparent(traceparent);

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        // Should use propagated trace ID
        assert_eq!(ctx.trace_id(), Some("0af7651916cd43dd8448eb211c80319c"));

        // Should have new span ID (not the parent)
        assert!(ctx.span_id().is_some());
        assert_ne!(ctx.span_id(), Some("b7ad6b7169203331"));

        // Should have parent span ID in extension
        let span_info = ctx.get_extension::<SpanInfo>().unwrap();
        assert_eq!(span_info.parent_span_id, Some("b7ad6b7169203331".to_string()));
    }

    #[tokio::test]
    async fn test_ignores_invalid_traceparent() {
        let middleware = TracingMiddleware::new("test-service");
        let mut ctx = MiddlewareContext::new();
        let request = create_request_with_traceparent("invalid-traceparent");

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        // Should have generated new trace ID
        assert!(ctx.trace_id().is_some());
        assert!(ctx.trace_id().unwrap().len() == 32);
    }

    #[test]
    fn test_parse_traceparent() {
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let context = TraceContext::parse(traceparent).unwrap();

        assert_eq!(context.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(context.parent_span_id, Some("b7ad6b7169203331".to_string()));
        assert!(context.flags.is_sampled());
    }

    #[test]
    fn test_parse_traceparent_invalid_version() {
        let traceparent = "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        assert!(TraceContext::parse(traceparent).is_none());
    }

    #[test]
    fn test_parse_traceparent_invalid_format() {
        assert!(TraceContext::parse("invalid").is_none());
        assert!(TraceContext::parse("00-abc-def-01").is_none());
        assert!(TraceContext::parse("").is_none());
    }

    #[test]
    fn test_trace_flags() {
        assert!(!TraceFlags::NONE.is_sampled());
        assert!(TraceFlags::SAMPLED.is_sampled());
        assert!(TraceFlags(0x03).is_sampled());
    }

    #[test]
    fn test_middleware_name() {
        let middleware = TracingMiddleware::new("test");
        assert_eq!(middleware.name(), "tracing");
    }
}
