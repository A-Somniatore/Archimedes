//! Response builders for common HTTP response types.
//!
//! This module provides convenient builders for creating HTTP responses
//! with proper content types and status codes.
//!
//! # Response Types
//!
//! | Builder | Content-Type | Description |
//! |---------|--------------|-------------|
//! | [`JsonResponse`] | `application/json` | JSON serialized response |
//! | [`HtmlResponse`] | `text/html` | HTML content |
//! | [`TextResponse`] | `text/plain` | Plain text |
//! | [`Redirect`] | N/A | HTTP redirect (301, 302, etc.) |
//! | [`NoContent`] | N/A | 204 No Content |
//!
//! # Example
//!
//! ```rust
//! use archimedes_extract::response::{JsonResponse, HtmlResponse, Redirect};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! // JSON response
//! let json = JsonResponse::new(User { id: 1, name: "Alice".into() });
//!
//! // HTML response
//! let html = HtmlResponse::new("<h1>Hello</h1>");
//!
//! // Redirect
//! let redirect = Redirect::to("/dashboard");
//! ```

use bytes::Bytes;
use http::{header, Response, StatusCode};
use serde::Serialize;

/// JSON response builder.
///
/// Creates an HTTP response with `Content-Type: application/json` and
/// the body serialized as JSON.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::response::JsonResponse;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct ApiResponse {
///     success: bool,
///     message: String,
/// }
///
/// let response = JsonResponse::new(ApiResponse {
///     success: true,
///     message: "Operation completed".into(),
/// });
///
/// assert_eq!(response.status(), http::StatusCode::OK);
/// ```
#[derive(Debug)]
pub struct JsonResponse<T> {
    data: T,
    status: StatusCode,
}

impl<T: Serialize> JsonResponse<T> {
    /// Creates a new JSON response with status 200 OK.
    #[must_use]
    pub fn new(data: T) -> Self {
        Self {
            data,
            status: StatusCode::OK,
        }
    }

    /// Creates a JSON response with status 201 Created.
    #[must_use]
    pub fn created(data: T) -> Self {
        Self {
            data,
            status: StatusCode::CREATED,
        }
    }

    /// Sets a custom status code.
    #[must_use]
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Returns the status code.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns a reference to the data.
    #[must_use]
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Builds the HTTP response.
    ///
    /// # Panics
    ///
    /// Panics if JSON serialization fails.
    #[must_use]
    pub fn into_response(self) -> Response<Bytes> {
        let body = serde_json::to_vec(&self.data).expect("JSON serialization failed");

        Response::builder()
            .status(self.status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Bytes::from(body))
            .expect("Failed to build response")
    }
}

/// HTML response builder.
///
/// Creates an HTTP response with `Content-Type: text/html; charset=utf-8`.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::response::HtmlResponse;
///
/// let response = HtmlResponse::new("<h1>Hello, World!</h1>");
/// assert_eq!(response.status(), http::StatusCode::OK);
/// ```
#[derive(Debug, Clone)]
pub struct HtmlResponse {
    body: String,
    status: StatusCode,
}

impl HtmlResponse {
    /// Creates a new HTML response with status 200 OK.
    #[must_use]
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            status: StatusCode::OK,
        }
    }

    /// Sets a custom status code.
    #[must_use]
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Returns the status code.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the body content.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Builds the HTTP response.
    #[must_use]
    pub fn into_response(self) -> Response<Bytes> {
        Response::builder()
            .status(self.status)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Bytes::from(self.body))
            .expect("Failed to build response")
    }
}

/// Plain text response builder.
///
/// Creates an HTTP response with `Content-Type: text/plain; charset=utf-8`.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::response::TextResponse;
///
/// let response = TextResponse::new("Hello, World!");
/// assert_eq!(response.status(), http::StatusCode::OK);
/// ```
#[derive(Debug, Clone)]
pub struct TextResponse {
    body: String,
    status: StatusCode,
}

impl TextResponse {
    /// Creates a new text response with status 200 OK.
    #[must_use]
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            status: StatusCode::OK,
        }
    }

    /// Sets a custom status code.
    #[must_use]
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Returns the status code.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the body content.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Builds the HTTP response.
    #[must_use]
    pub fn into_response(self) -> Response<Bytes> {
        Response::builder()
            .status(self.status)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Bytes::from(self.body))
            .expect("Failed to build response")
    }
}

/// HTTP redirect response builder.
///
/// Creates redirect responses with various status codes.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::response::Redirect;
///
/// // Temporary redirect (302)
/// let redirect = Redirect::to("/dashboard");
///
/// // Permanent redirect (301)
/// let permanent = Redirect::permanent("/new-location");
///
/// // See Other (303) - after POST
/// let see_other = Redirect::see_other("/result");
/// ```
#[derive(Debug, Clone)]
pub struct Redirect {
    location: String,
    status: StatusCode,
}

impl Redirect {
    /// Creates a temporary redirect (302 Found).
    #[must_use]
    pub fn to(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            status: StatusCode::FOUND,
        }
    }

    /// Creates a permanent redirect (301 Moved Permanently).
    #[must_use]
    pub fn permanent(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            status: StatusCode::MOVED_PERMANENTLY,
        }
    }

    /// Creates a See Other redirect (303).
    /// Typically used after POST to redirect to GET.
    #[must_use]
    pub fn see_other(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            status: StatusCode::SEE_OTHER,
        }
    }

    /// Creates a Temporary Redirect (307).
    /// Preserves the HTTP method.
    #[must_use]
    pub fn temporary(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            status: StatusCode::TEMPORARY_REDIRECT,
        }
    }

    /// Creates a Permanent Redirect (308).
    /// Preserves the HTTP method.
    #[must_use]
    pub fn permanent_redirect(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            status: StatusCode::PERMANENT_REDIRECT,
        }
    }

    /// Returns the status code.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the redirect location.
    #[must_use]
    pub fn location(&self) -> &str {
        &self.location
    }

    /// Builds the HTTP response.
    #[must_use]
    pub fn into_response(self) -> Response<Bytes> {
        Response::builder()
            .status(self.status)
            .header(header::LOCATION, self.location)
            .body(Bytes::new())
            .expect("Failed to build response")
    }
}

/// No Content response (204).
///
/// Creates an empty response with status 204 No Content.
/// Typically used for successful DELETE requests or updates that
/// don't return data.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::response::NoContent;
///
/// let response = NoContent::new();
/// assert_eq!(response.status(), http::StatusCode::NO_CONTENT);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NoContent;

impl NoContent {
    /// Creates a new 204 No Content response.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Returns the status code (always 204).
    #[must_use]
    pub fn status(&self) -> StatusCode {
        StatusCode::NO_CONTENT
    }

    /// Builds the HTTP response.
    #[must_use]
    pub fn into_response(self) -> Response<Bytes> {
        Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Bytes::new())
            .expect("Failed to build response")
    }
}

/// Error response builder.
///
/// Creates standardized error responses matching the Themis error envelope format.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::response::ErrorResponse;
/// use http::StatusCode;
///
/// let response = ErrorResponse::new(
///     StatusCode::NOT_FOUND,
///     "NOT_FOUND",
///     "User not found",
/// );
/// ```
#[derive(Debug, Clone)]
pub struct ErrorResponse {
    status: StatusCode,
    code: String,
    message: String,
    request_id: Option<String>,
}

impl ErrorResponse {
    /// Creates a new error response.
    #[must_use]
    pub fn new(
        status: StatusCode,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            request_id: None,
        }
    }

    /// Creates a 400 Bad Request error.
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", message)
    }

    /// Creates a 401 Unauthorized error.
    #[must_use]
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message)
    }

    /// Creates a 403 Forbidden error.
    #[must_use]
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "FORBIDDEN", message)
    }

    /// Creates a 404 Not Found error.
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", message)
    }

    /// Creates a 500 Internal Server Error.
    #[must_use]
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message)
    }

    /// Sets the request ID for error tracking.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Returns the status code.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Builds the HTTP response with JSON error envelope.
    #[must_use]
    pub fn into_response(self) -> Response<Bytes> {
        #[derive(serde::Serialize)]
        struct ErrorEnvelope {
            code: String,
            message: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            request_id: Option<String>,
        }

        let envelope = ErrorEnvelope {
            code: self.code,
            message: self.message,
            request_id: self.request_id,
        };

        let body = serde_json::to_vec(&envelope).expect("JSON serialization failed");

        Response::builder()
            .status(self.status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Bytes::from(body))
            .expect("Failed to build response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestData {
        id: u64,
        name: String,
    }

    #[test]
    fn test_json_response() {
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
        };

        let response = JsonResponse::new(data);
        assert_eq!(response.status(), StatusCode::OK);

        let http_response = response.into_response();
        assert_eq!(http_response.status(), StatusCode::OK);
        assert_eq!(
            http_response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_json_response_created() {
        let data = TestData {
            id: 1,
            name: "New".to_string(),
        };

        let response = JsonResponse::created(data);
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[test]
    fn test_json_response_custom_status() {
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
        };

        let response = JsonResponse::new(data).with_status(StatusCode::ACCEPTED);
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn test_html_response() {
        let response = HtmlResponse::new("<h1>Hello</h1>");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body(), "<h1>Hello</h1>");

        let http_response = response.into_response();
        assert_eq!(
            http_response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[test]
    fn test_text_response() {
        let response = TextResponse::new("Hello, World!");
        assert_eq!(response.status(), StatusCode::OK);

        let http_response = response.into_response();
        assert_eq!(
            http_response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8"
        );
    }

    #[test]
    fn test_redirect_to() {
        let redirect = Redirect::to("/dashboard");
        assert_eq!(redirect.status(), StatusCode::FOUND);
        assert_eq!(redirect.location(), "/dashboard");
    }

    #[test]
    fn test_redirect_permanent() {
        let redirect = Redirect::permanent("/new-url");
        assert_eq!(redirect.status(), StatusCode::MOVED_PERMANENTLY);
    }

    #[test]
    fn test_redirect_see_other() {
        let redirect = Redirect::see_other("/result");
        assert_eq!(redirect.status(), StatusCode::SEE_OTHER);
    }

    #[test]
    fn test_redirect_response() {
        let redirect = Redirect::to("/target");
        let response = redirect.into_response();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "/target"
        );
    }

    #[test]
    fn test_no_content() {
        let response = NoContent::new();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let http_response = response.into_response();
        assert_eq!(http_response.status(), StatusCode::NO_CONTENT);
        assert!(http_response.body().is_empty());
    }

    #[test]
    fn test_error_response() {
        let response = ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INPUT",
            "Invalid email format",
        );

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_response_shortcuts() {
        assert_eq!(
            ErrorResponse::bad_request("test").status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorResponse::unauthorized("test").status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorResponse::forbidden("test").status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorResponse::not_found("test").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorResponse::internal_error("test").status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_response_with_request_id() {
        let response = ErrorResponse::not_found("User not found")
            .with_request_id("req-123");

        let http_response = response.into_response();
        let body: serde_json::Value =
            serde_json::from_slice(http_response.body()).unwrap();

        assert_eq!(body["request_id"], "req-123");
    }
}
