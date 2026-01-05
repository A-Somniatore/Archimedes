//! Telemetry emission middleware.
//!
//! This middleware collects and emits telemetry data for every request,
//! including metrics, timing information, and structured logging.
//!
//! # Pipeline Position
//!
//! Telemetry runs after Response Validation and before Error Normalization:
//!
//! ```text
//! Handler → ResponseValidation → [Telemetry] → ErrorNormalization → Response
//! ```
//!
//! # Metrics Emitted
//!
//! - `archimedes_requests_total` - Counter of total requests by operation and status
//! - `archimedes_request_duration_seconds` - Histogram of request latency
//! - `archimedes_in_flight_requests` - Gauge of currently processing requests
//!
//! # Log Format
//!
//! Structured JSON logs include:
//! - `request_id` - Unique request identifier
//! - `trace_id` - Distributed trace ID
//! - `span_id` - Current span ID
//! - `operation_id` - Contract operation being called
//! - `status_code` - HTTP response status
//! - `duration_ms` - Request duration in milliseconds
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_middleware::stages::TelemetryMiddleware;
//!
//! // Default configuration
//! let telemetry = TelemetryMiddleware::new("my-service");
//!
//! // With custom configuration
//! let telemetry = TelemetryMiddleware::builder("my-service")
//!     .version("1.0.0")
//!     .environment("production")
//!     .build();
//! ```

use crate::{
    context::MiddlewareContext,
    middleware::{BoxFuture, Middleware, Next},
    types::{Request, Response},
};
use std::time::Instant;

/// Telemetry middleware that emits metrics and logs for every request.
#[derive(Debug, Clone)]
pub struct TelemetryMiddleware {
    /// Service name for telemetry labels.
    service_name: String,
    /// Service version.
    version: String,
    /// Environment (e.g., production, staging).
    environment: String,
    /// Whether to emit detailed logs.
    verbose: bool,
}

/// Telemetry data collected during request processing.
#[derive(Debug, Clone)]
pub struct TelemetryData {
    /// The service name.
    pub service_name: String,
    /// The service version.
    pub version: String,
    /// The environment.
    pub environment: String,
    /// The operation ID.
    pub operation_id: String,
    /// The HTTP method.
    pub method: String,
    /// The request path.
    pub path: String,
    /// The HTTP status code.
    pub status_code: u16,
    /// Request duration in milliseconds.
    pub duration_ms: f64,
    /// The request ID.
    pub request_id: String,
    /// The trace ID (if available).
    pub trace_id: Option<String>,
    /// The span ID (if available).
    pub span_id: Option<String>,
}

impl TelemetryMiddleware {
    /// Creates a new telemetry middleware with the given service name.
    #[must_use]
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            version: "unknown".to_string(),
            environment: "unknown".to_string(),
            verbose: false,
        }
    }

    /// Creates a builder for more detailed configuration.
    #[must_use]
    pub fn builder(service_name: &str) -> TelemetryBuilder {
        TelemetryBuilder {
            service_name: service_name.to_string(),
            version: "unknown".to_string(),
            environment: "unknown".to_string(),
            verbose: false,
        }
    }

    /// Collects telemetry data from the context and response.
    ///
    /// Note: This method provides a reusable way to collect telemetry data.
    /// Currently telemetry collection is inlined in the process method for
    /// ownership reasons, but this method is kept for future use.
    #[allow(dead_code)]
    fn collect_telemetry(
        &self,
        ctx: &MiddlewareContext,
        request: &Request,
        response: &Response,
        duration: std::time::Duration,
    ) -> TelemetryData {
        TelemetryData {
            service_name: self.service_name.clone(),
            version: self.version.clone(),
            environment: self.environment.clone(),
            operation_id: ctx.operation_id().unwrap_or("unknown").to_string(),
            method: request.method().to_string(),
            path: request.uri().path().to_string(),
            status_code: response.status().as_u16(),
            duration_ms: duration.as_secs_f64() * 1000.0,
            request_id: ctx.request_id().to_string(),
            trace_id: ctx.trace_id().map(ToString::to_string),
            span_id: ctx.span_id().map(ToString::to_string),
        }
    }

    /// Emits telemetry (metrics and logs).
    ///
    /// In production, this would send to OpenTelemetry collectors.
    /// For now, this is a mock that stores data in context.
    fn emit_telemetry(&self, ctx: &mut MiddlewareContext, data: TelemetryData) {
        // Store telemetry data in context for testing/inspection
        ctx.set_extension(data.clone());

        // In production, this would:
        // 1. Increment request counter metric
        // 2. Record duration histogram
        // 3. Emit structured log

        if self.verbose {
            // Would emit detailed log in production
            // For now, data is just stored in context
        }
    }
}

impl Middleware for TelemetryMiddleware {
    fn name(&self) -> &'static str {
        "telemetry"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Record start time
            let start = Instant::now();

            // Clone request info before passing ownership
            let method = request.method().to_string();
            let path = request.uri().path().to_string();

            // Process the request
            let response = next.run(ctx, request).await;

            // Calculate duration
            let duration = start.elapsed();

            // Create telemetry data
            let data = TelemetryData {
                service_name: self.service_name.clone(),
                version: self.version.clone(),
                environment: self.environment.clone(),
                operation_id: ctx.operation_id().unwrap_or("unknown").to_string(),
                method,
                path,
                status_code: response.status().as_u16(),
                duration_ms: duration.as_secs_f64() * 1000.0,
                request_id: ctx.request_id().to_string(),
                trace_id: ctx.trace_id().map(ToString::to_string),
                span_id: ctx.span_id().map(ToString::to_string),
            };

            // Emit telemetry
            self.emit_telemetry(ctx, data);

            response
        })
    }
}

/// Builder for `TelemetryMiddleware`.
#[derive(Debug)]
pub struct TelemetryBuilder {
    service_name: String,
    version: String,
    environment: String,
    verbose: bool,
}

impl TelemetryBuilder {
    /// Sets the service version.
    #[must_use]
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Sets the environment.
    #[must_use]
    pub fn environment(mut self, environment: &str) -> Self {
        self.environment = environment.to_string();
        self
    }

    /// Enables verbose logging.
    #[must_use]
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Builds the telemetry middleware.
    #[must_use]
    pub fn build(self) -> TelemetryMiddleware {
        TelemetryMiddleware {
            service_name: self.service_name,
            version: self.version,
            environment: self.environment,
            verbose: self.verbose,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::Next;
    use bytes::Bytes;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;

    fn make_test_request() -> Request {
        HttpRequest::builder()
            .method("GET")
            .uri("/users/123")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn success_response() -> Response {
        HttpResponse::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from(r#"{"id":"123"}"#)))
            .unwrap()
    }

    fn create_handler() -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        |_ctx, _req| Box::pin(async { success_response() })
    }

    #[test]
    fn test_middleware_name() {
        let middleware = TelemetryMiddleware::new("test-service");
        assert_eq!(middleware.name(), "telemetry");
    }

    #[test]
    fn test_builder_configuration() {
        let middleware = TelemetryMiddleware::builder("my-service")
            .version("1.0.0")
            .environment("production")
            .verbose(true)
            .build();

        assert_eq!(middleware.service_name, "my-service");
        assert_eq!(middleware.version, "1.0.0");
        assert_eq!(middleware.environment, "production");
        assert!(middleware.verbose);
    }

    #[tokio::test]
    async fn test_telemetry_collects_data() {
        let middleware = TelemetryMiddleware::builder("test-service")
            .version("1.0.0")
            .environment("test")
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("getUser".to_string());

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);

        // Check telemetry was collected
        let telemetry = ctx.get_extension::<TelemetryData>().unwrap();
        assert_eq!(telemetry.service_name, "test-service");
        assert_eq!(telemetry.version, "1.0.0");
        assert_eq!(telemetry.environment, "test");
        assert_eq!(telemetry.operation_id, "getUser");
        assert_eq!(telemetry.method, "GET");
        assert_eq!(telemetry.path, "/users/123");
        assert_eq!(telemetry.status_code, 200);
        assert!(telemetry.duration_ms >= 0.0);
    }

    #[tokio::test]
    async fn test_telemetry_includes_request_id() {
        let middleware = TelemetryMiddleware::new("test-service");

        let mut ctx = MiddlewareContext::new();
        // Request ID should already be set from RequestIdMiddleware

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);

        let telemetry = ctx.get_extension::<TelemetryData>().unwrap();
        assert!(!telemetry.request_id.is_empty());
    }

    #[tokio::test]
    async fn test_telemetry_captures_trace_context() {
        let middleware = TelemetryMiddleware::new("test-service");

        let mut ctx = MiddlewareContext::new();
        ctx.set_trace_id("abc123".to_string());
        ctx.set_span_id("span456".to_string());

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);

        let telemetry = ctx.get_extension::<TelemetryData>().unwrap();
        assert_eq!(telemetry.trace_id, Some("abc123".to_string()));
        assert_eq!(telemetry.span_id, Some("span456".to_string()));
    }

    #[tokio::test]
    async fn test_telemetry_captures_error_responses() {
        let middleware = TelemetryMiddleware::new("test-service");

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("getUser".to_string());

        let request = make_test_request();
        let next = Next::handler(|_ctx, _req| {
            Box::pin(async {
                HttpResponse::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::new(Bytes::from(r#"{"error":"not found"}"#)))
                    .unwrap()
            })
        });

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let telemetry = ctx.get_extension::<TelemetryData>().unwrap();
        assert_eq!(telemetry.status_code, 404);
    }

    #[test]
    fn test_telemetry_data_structure() {
        let data = TelemetryData {
            service_name: "test".to_string(),
            version: "1.0.0".to_string(),
            environment: "production".to_string(),
            operation_id: "getUser".to_string(),
            method: "GET".to_string(),
            path: "/users/123".to_string(),
            status_code: 200,
            duration_ms: 45.5,
            request_id: "req-123".to_string(),
            trace_id: Some("trace-abc".to_string()),
            span_id: Some("span-xyz".to_string()),
        };

        assert_eq!(data.service_name, "test");
        assert_eq!(data.status_code, 200);
        assert_eq!(data.duration_ms, 45.5);
    }
}
