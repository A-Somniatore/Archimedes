//! Request ID middleware.
//!
//! This middleware is responsible for generating or extracting a unique
//! request ID for each incoming request. The request ID is used for:
//!
//! - Log correlation across services
//! - Distributed tracing
//! - Support ticket references
//!
//! ## Request ID Sources
//!
//! 1. **X-Request-ID header**: If present, the existing ID is used
//! 2. **Generated UUID v7**: If no header is present, a new ID is generated
//!
//! UUID v7 is used because it is:
//! - Time-ordered (naturally sortable)
//! - Contains embedded timestamp
//! - Globally unique without coordination
//!
//! ## Response Header
//!
//! The middleware always sets the `X-Request-ID` header on the response,
//! allowing clients to correlate their requests with server logs.

use crate::context::MiddlewareContext;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::types::{Request, Response};
use archimedes_core::RequestId;
use uuid::Uuid;

/// The header name for request ID propagation.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Middleware that generates or extracts request IDs.
///
/// This middleware ensures every request has a unique identifier that
/// can be used for logging, tracing, and debugging.
///
/// # Behavior
///
/// 1. Check for `X-Request-ID` header
/// 2. If present, use existing ID (with validation)
/// 3. If absent, generate new UUID v7
/// 4. Store ID in [`MiddlewareContext`]
/// 5. Add ID to response headers
///
/// # Example
///
/// ```ignore
/// use archimedes_middleware::stages::request_id::RequestIdMiddleware;
///
/// let middleware = RequestIdMiddleware::new();
/// // Add to pipeline...
/// ```
#[derive(Debug, Clone, Default)]
pub struct RequestIdMiddleware {
    /// Whether to trust incoming request ID headers.
    ///
    /// In production, this should typically be `false` for external traffic
    /// and `true` for internal service-to-service calls.
    trust_incoming: bool,
}

impl RequestIdMiddleware {
    /// Creates a new Request ID middleware.
    ///
    /// By default, incoming request IDs are not trusted and new IDs
    /// are always generated.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a middleware that trusts incoming `X-Request-ID` headers.
    ///
    /// Use this for internal services that receive requests from other
    /// trusted services that have already assigned request IDs.
    #[must_use]
    pub fn trust_incoming() -> Self {
        Self { trust_incoming: true }
    }

    /// Extracts request ID from headers if present and valid.
    fn extract_request_id(&self, request: &Request) -> Option<RequestId> {
        if !self.trust_incoming {
            return None;
        }

        request
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(RequestId::from_uuid)
    }
}

impl Middleware for RequestIdMiddleware {
    fn name(&self) -> &'static str {
        "request_id"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Extract or generate request ID
            let request_id = self
                .extract_request_id(&request)
                .unwrap_or_else(RequestId::new);

            // Store in context
            ctx.set_request_id(request_id);

            // Process request through remaining middleware
            let mut response = next.run(ctx, request).await;

            // Add request ID to response headers
            response.headers_mut().insert(
                REQUEST_ID_HEADER,
                request_id.to_string().parse().expect("valid header value"),
            );

            response
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;

    fn create_test_request() -> Request {
        HttpRequest::builder()
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_request_with_id(request_id: &str) -> Request {
        HttpRequest::builder()
            .uri("/test")
            .header(REQUEST_ID_HEADER, request_id)
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
    async fn test_generates_request_id_when_missing() {
        let middleware = RequestIdMiddleware::new();
        let mut ctx = MiddlewareContext::new();
        let original_id = ctx.request_id();
        let request = create_test_request();

        let next = Next::handler(create_handler());
        let response = middleware.process(&mut ctx, request, next).await;

        // Context should have a request ID (may be different from original)
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
        let header_id = response.headers().get(REQUEST_ID_HEADER).unwrap().to_str().unwrap();
        assert!(!header_id.is_empty());
        
        // The ID in context should match response header
        assert_eq!(ctx.request_id().to_string(), header_id);
    }

    #[tokio::test]
    async fn test_ignores_incoming_id_when_not_trusted() {
        let middleware = RequestIdMiddleware::new();
        let mut ctx = MiddlewareContext::new();
        let incoming_id = "12345678-1234-7234-1234-123456789abc";
        let request = create_request_with_id(incoming_id);

        let next = Next::handler(create_handler());
        let response = middleware.process(&mut ctx, request, next).await;

        // Should have generated a new ID, not used the incoming one
        let header_id = response.headers().get(REQUEST_ID_HEADER).unwrap().to_str().unwrap();
        assert_ne!(header_id, incoming_id);
    }

    #[tokio::test]
    async fn test_uses_incoming_id_when_trusted() {
        let middleware = RequestIdMiddleware::trust_incoming();
        let mut ctx = MiddlewareContext::new();
        // Use valid UUID v7 format
        let incoming_id = "01234567-89ab-7def-8123-456789abcdef";
        let request = create_request_with_id(incoming_id);

        let next = Next::handler(create_handler());
        let response = middleware.process(&mut ctx, request, next).await;

        // Should use the incoming ID
        let header_id = response.headers().get(REQUEST_ID_HEADER).unwrap().to_str().unwrap();
        assert_eq!(header_id, incoming_id);
        assert_eq!(ctx.request_id().to_string(), incoming_id);
    }

    #[tokio::test]
    async fn test_ignores_invalid_incoming_id() {
        let middleware = RequestIdMiddleware::trust_incoming();
        let mut ctx = MiddlewareContext::new();
        let invalid_id = "not-a-valid-uuid";
        let request = create_request_with_id(invalid_id);

        let next = Next::handler(create_handler());
        let response = middleware.process(&mut ctx, request, next).await;

        // Should generate new ID since incoming is invalid
        let header_id = response.headers().get(REQUEST_ID_HEADER).unwrap().to_str().unwrap();
        assert_ne!(header_id, invalid_id);
        // Should be a valid UUID
        assert!(Uuid::parse_str(header_id).is_ok());
    }

    #[test]
    fn test_middleware_name() {
        let middleware = RequestIdMiddleware::new();
        assert_eq!(middleware.name(), "request_id");
    }

    #[test]
    fn test_trust_incoming_config() {
        let default = RequestIdMiddleware::new();
        let trusting = RequestIdMiddleware::trust_incoming();

        assert!(!default.trust_incoming);
        assert!(trusting.trust_incoming);
    }
}
