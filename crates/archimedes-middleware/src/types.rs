//! Common types used throughout the middleware pipeline.
//!
//! This module re-exports HTTP request and response types used by middleware.

use bytes::Bytes;
use http_body_util::Full;

/// The HTTP request type used in the middleware pipeline.
///
/// This is a standard `http::Request` with a `Full<Bytes>` body.
pub type Request = http::Request<Full<Bytes>>;

/// The HTTP response type used in the middleware pipeline.
///
/// This is a standard `http::Response` with a `Full<Bytes>` body.
pub type Response = http::Response<Full<Bytes>>;

/// A boxed HTTP body for streaming responses.
pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, std::convert::Infallible>;

/// Extension trait for building error responses.
pub trait ResponseExt {
    /// Creates an error response with the given status code and message.
    fn error(status: http::StatusCode, message: &str) -> Response;

    /// Creates a JSON error response.
    fn json_error(status: http::StatusCode, code: &str, message: &str) -> Response;
}

impl ResponseExt for Response {
    fn error(status: http::StatusCode, message: &str) -> Response {
        http::Response::builder()
            .status(status)
            .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Full::new(Bytes::from(message.to_string())))
            .expect("failed to build error response")
    }

    fn json_error(status: http::StatusCode, code: &str, message: &str) -> Response {
        let body = serde_json::json!({
            "error": {
                "code": code,
                "message": message
            }
        });

        http::Response::builder()
            .status(status)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .expect("failed to build JSON error response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;

    #[test]
    fn test_error_response() {
        let response = Response::error(StatusCode::BAD_REQUEST, "Invalid input");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8"
        );
    }

    #[test]
    fn test_json_error_response() {
        let response = Response::json_error(
            StatusCode::UNAUTHORIZED,
            "AUTH_REQUIRED",
            "Authentication required",
        );
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }
}
