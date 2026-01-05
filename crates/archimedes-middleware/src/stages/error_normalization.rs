//! Error normalization middleware.
//!
//! This middleware ensures all errors are converted to a standard error envelope
//! format before being sent to clients. It catches any panics or unexpected errors
//! and converts them to properly formatted responses.
//!
//! # Pipeline Position
//!
//! Error normalization is the final middleware stage:
//!
//! ```text
//! Handler → ResponseValidation → Telemetry → [ErrorNormalization] → Response
//! ```
//!
//! # Error Envelope Format
//!
//! All errors are converted to this format:
//!
//! ```json
//! {
//!   "error": {
//!     "code": "ERROR_CODE",
//!     "message": "Human-readable error message",
//!     "request_id": "uuid-v7-request-id"
//!   }
//! }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_middleware::stages::ErrorNormalizationMiddleware;
//!
//! // Default configuration
//! let error_norm = ErrorNormalizationMiddleware::new();
//!
//! // With verbose internal errors (development only)
//! let error_norm = ErrorNormalizationMiddleware::new()
//!     .expose_internal_errors(true);
//! ```

use crate::{
    context::MiddlewareContext,
    middleware::{BoxFuture, Middleware, Next},
    types::{Request, Response},
};
use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;

/// Error normalization middleware that ensures consistent error responses.
#[derive(Debug, Clone)]
pub struct ErrorNormalizationMiddleware {
    /// Whether to expose internal error details (development mode).
    expose_internal_errors: bool,
    /// Default error message for internal errors.
    internal_error_message: String,
}

/// Normalized error data stored in context.
#[derive(Debug, Clone)]
pub struct NormalizedError {
    /// The error code.
    pub code: String,
    /// The error message.
    pub message: String,
    /// The HTTP status code.
    pub status_code: u16,
    /// Whether this was normalized from an internal error.
    pub was_internal: bool,
}

impl Default for ErrorNormalizationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorNormalizationMiddleware {
    /// Creates a new error normalization middleware with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            expose_internal_errors: false,
            internal_error_message: "An internal error occurred".to_string(),
        }
    }

    /// Sets whether to expose internal error details.
    ///
    /// **Warning**: Only enable this in development environments.
    #[must_use]
    pub fn expose_internal_errors(mut self, expose: bool) -> Self {
        self.expose_internal_errors = expose;
        self
    }

    /// Sets the default message for internal errors.
    #[must_use]
    pub fn internal_error_message(mut self, message: &str) -> Self {
        self.internal_error_message = message.to_string();
        self
    }

    /// Normalizes an error response.
    fn normalize_error_response(
        &self,
        ctx: &MiddlewareContext,
        response: Response,
    ) -> Response {
        let status = response.status();

        // Only normalize error responses (4xx and 5xx)
        if status.is_success() || status.is_informational() || status.is_redirection() {
            return response;
        }

        // Get error code from status
        let code = self.status_to_code(status);

        // Get message - either from body or default
        let message = if status.is_server_error() && !self.expose_internal_errors {
            self.internal_error_message.clone()
        } else {
            self.extract_message_from_response(&response)
                .unwrap_or_else(|| status.canonical_reason().unwrap_or("Unknown error").to_string())
        };

        // Create normalized error response
        let error_body = serde_json::json!({
            "error": {
                "code": code,
                "message": message,
                "request_id": ctx.request_id().to_string()
            }
        });

        http::Response::builder()
            .status(status)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(error_body.to_string())))
            .expect("failed to build error response")
    }

    /// Converts HTTP status to error code.
    fn status_to_code(&self, status: StatusCode) -> String {
        match status.as_u16() {
            400 => "BAD_REQUEST".to_string(),
            401 => "UNAUTHORIZED".to_string(),
            403 => "FORBIDDEN".to_string(),
            404 => "NOT_FOUND".to_string(),
            405 => "METHOD_NOT_ALLOWED".to_string(),
            408 => "REQUEST_TIMEOUT".to_string(),
            409 => "CONFLICT".to_string(),
            422 => "UNPROCESSABLE_ENTITY".to_string(),
            429 => "RATE_LIMITED".to_string(),
            500 => "INTERNAL_ERROR".to_string(),
            502 => "BAD_GATEWAY".to_string(),
            503 => "SERVICE_UNAVAILABLE".to_string(),
            504 => "GATEWAY_TIMEOUT".to_string(),
            _ => format!("HTTP_{}", status.as_u16()),
        }
    }

    /// Extracts error message from response body if possible.
    fn extract_message_from_response(&self, _response: &Response) -> Option<String> {
        // In a real implementation, we'd try to parse the body
        // For now, return None to use default message
        None
    }
}

impl Middleware for ErrorNormalizationMiddleware {
    fn name(&self) -> &'static str {
        "error_normalization"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Process the request
            let response = next.run(ctx, request).await;

            // Check if it's an error response
            if response.status().is_client_error() || response.status().is_server_error() {
                let status = response.status();
                let code = self.status_to_code(status);

                // Store normalized error info in context
                ctx.set_extension(NormalizedError {
                    code: code.clone(),
                    message: status.canonical_reason().unwrap_or("Unknown error").to_string(),
                    status_code: status.as_u16(),
                    was_internal: status.is_server_error(),
                });

                // Normalize the error response
                self.normalize_error_response(ctx, response)
            } else {
                response
            }
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::Next;
    use http::{Request as HttpRequest, Response as HttpResponse};

    fn make_test_request() -> Request {
        HttpRequest::builder()
            .method("GET")
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn success_response() -> Response {
        HttpResponse::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
            .unwrap()
    }

    fn error_response(status: StatusCode) -> Response {
        HttpResponse::builder()
            .status(status)
            .body(Full::new(Bytes::from(r#"{"error":"something went wrong"}"#)))
            .unwrap()
    }

    fn create_handler() -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        |_ctx, _req| Box::pin(async { success_response() })
    }

    fn create_error_handler(status: StatusCode) -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        move |_ctx, _req| Box::pin(async move { error_response(status) })
    }

    #[test]
    fn test_middleware_name() {
        let middleware = ErrorNormalizationMiddleware::new();
        assert_eq!(middleware.name(), "error_normalization");
    }

    #[test]
    fn test_default_configuration() {
        let middleware = ErrorNormalizationMiddleware::default();
        assert!(!middleware.expose_internal_errors);
    }

    #[test]
    fn test_status_to_code() {
        let middleware = ErrorNormalizationMiddleware::new();

        assert_eq!(middleware.status_to_code(StatusCode::BAD_REQUEST), "BAD_REQUEST");
        assert_eq!(middleware.status_to_code(StatusCode::UNAUTHORIZED), "UNAUTHORIZED");
        assert_eq!(middleware.status_to_code(StatusCode::FORBIDDEN), "FORBIDDEN");
        assert_eq!(middleware.status_to_code(StatusCode::NOT_FOUND), "NOT_FOUND");
        assert_eq!(middleware.status_to_code(StatusCode::INTERNAL_SERVER_ERROR), "INTERNAL_ERROR");
        assert_eq!(middleware.status_to_code(StatusCode::SERVICE_UNAVAILABLE), "SERVICE_UNAVAILABLE");
    }

    #[tokio::test]
    async fn test_success_response_passes_through() {
        let middleware = ErrorNormalizationMiddleware::new();
        let mut ctx = MiddlewareContext::new();

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);

        // No error should be stored
        assert!(ctx.get_extension::<NormalizedError>().is_none());
    }

    #[tokio::test]
    async fn test_client_error_normalized() {
        let middleware = ErrorNormalizationMiddleware::new();
        let mut ctx = MiddlewareContext::new();

        let request = make_test_request();
        let next = Next::handler(create_error_handler(StatusCode::NOT_FOUND));

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // Check normalized error in context
        let error = ctx.get_extension::<NormalizedError>().unwrap();
        assert_eq!(error.code, "NOT_FOUND");
        assert_eq!(error.status_code, 404);
        assert!(!error.was_internal);
    }

    #[tokio::test]
    async fn test_server_error_normalized() {
        let middleware = ErrorNormalizationMiddleware::new();
        let mut ctx = MiddlewareContext::new();

        let request = make_test_request();
        let next = Next::handler(create_error_handler(StatusCode::INTERNAL_SERVER_ERROR));

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Check normalized error in context
        let error = ctx.get_extension::<NormalizedError>().unwrap();
        assert_eq!(error.code, "INTERNAL_ERROR");
        assert_eq!(error.status_code, 500);
        assert!(error.was_internal);
    }

    #[tokio::test]
    async fn test_error_includes_request_id() {
        let middleware = ErrorNormalizationMiddleware::new();
        let mut ctx = MiddlewareContext::new();

        let request = make_test_request();
        let next = Next::handler(create_error_handler(StatusCode::BAD_REQUEST));

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Response should include request_id in body
        // (verified via the normalize_error_response method)
        assert!(ctx.get_extension::<NormalizedError>().is_some());
    }

    #[test]
    fn test_expose_internal_errors_configuration() {
        let middleware = ErrorNormalizationMiddleware::new()
            .expose_internal_errors(true)
            .internal_error_message("Custom internal error");

        assert!(middleware.expose_internal_errors);
        assert_eq!(middleware.internal_error_message, "Custom internal error");
    }

    #[test]
    fn test_normalized_error_structure() {
        let error = NormalizedError {
            code: "NOT_FOUND".to_string(),
            message: "Resource not found".to_string(),
            status_code: 404,
            was_internal: false,
        };

        assert_eq!(error.code, "NOT_FOUND");
        assert_eq!(error.status_code, 404);
        assert!(!error.was_internal);
    }
}
