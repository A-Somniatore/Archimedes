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
//! | [`FileResponse`] | Auto-detected | File download response |
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
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
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

/// File download response builder.
///
/// Creates an HTTP response for file downloads with proper
/// `Content-Disposition` and `Content-Type` headers.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::response::FileResponse;
///
/// // Simple file download
/// let response = FileResponse::new(b"file content".to_vec())
///     .filename("document.txt");
///
/// // With explicit content type
/// let response = FileResponse::new(vec![0x89, 0x50, 0x4E, 0x47])
///     .filename("image.png")
///     .content_type("image/png");
///
/// // Inline display (browser shows instead of downloads)
/// let response = FileResponse::new(b"PDF content".to_vec())
///     .filename("report.pdf")
///     .inline();
/// ```
#[derive(Debug, Clone)]
pub struct FileResponse {
    data: Vec<u8>,
    filename: Option<String>,
    content_type: Option<String>,
    disposition: ContentDisposition,
    status: StatusCode,
}

/// Content-Disposition type for file responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentDisposition {
    /// Browser should download the file.
    #[default]
    Attachment,
    /// Browser should display the file inline if possible.
    Inline,
}

impl FileResponse {
    /// Creates a new file response from bytes.
    #[must_use]
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self {
            data: data.into(),
            filename: None,
            content_type: None,
            disposition: ContentDisposition::default(),
            status: StatusCode::OK,
        }
    }

    /// Creates a file response from a string.
    #[must_use]
    pub fn from_string(content: impl Into<String>) -> Self {
        Self::new(content.into().into_bytes())
    }

    /// Sets the filename for the `Content-Disposition` header.
    ///
    /// This also attempts to auto-detect the content type from the extension
    /// if not explicitly set.
    #[must_use]
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Sets the content type explicitly.
    #[must_use]
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Sets the disposition to inline (browser displays instead of downloads).
    #[must_use]
    pub fn inline(mut self) -> Self {
        self.disposition = ContentDisposition::Inline;
        self
    }

    /// Sets the disposition to attachment (browser downloads).
    #[must_use]
    pub fn attachment(mut self) -> Self {
        self.disposition = ContentDisposition::Attachment;
        self
    }

    /// Sets a custom status code (default is 200 OK).
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

    /// Returns the filename if set.
    #[must_use]
    pub fn get_filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Returns the content type, auto-detecting from filename if not set.
    #[must_use]
    pub fn get_content_type(&self) -> &str {
        if let Some(ref ct) = self.content_type {
            return ct;
        }

        // Auto-detect from filename extension
        if let Some(ref filename) = self.filename {
            return Self::mime_from_extension(filename);
        }

        "application/octet-stream"
    }

    /// Returns the content disposition.
    #[must_use]
    pub fn get_disposition(&self) -> ContentDisposition {
        self.disposition
    }

    /// Returns the data length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the data is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Guesses MIME type from file extension.
    fn mime_from_extension(filename: &str) -> &'static str {
        let ext = filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            // Text
            "txt" => "text/plain",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "csv" => "text/csv",
            "xml" => "text/xml",
            "md" => "text/markdown",

            // JavaScript/JSON
            "js" | "mjs" => "application/javascript",
            "json" => "application/json",

            // Images
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "ico" => "image/x-icon",
            "bmp" => "image/bmp",

            // Audio
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "flac" => "audio/flac",

            // Video
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",

            // Documents
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

            // Archives
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" | "gzip" => "application/gzip",
            "rar" => "application/vnd.rar",
            "7z" => "application/x-7z-compressed",

            // Fonts
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "eot" => "application/vnd.ms-fontobject",

            // Other
            "wasm" => "application/wasm",
            "yaml" | "yml" => "application/x-yaml",
            "toml" => "application/toml",

            // Default
            _ => "application/octet-stream",
        }
    }

    /// Builds the `Content-Disposition` header value.
    fn build_content_disposition(&self) -> String {
        let disposition_type = match self.disposition {
            ContentDisposition::Attachment => "attachment",
            ContentDisposition::Inline => "inline",
        };

        match &self.filename {
            Some(filename) => {
                // RFC 5987 encoding for non-ASCII filenames
                let needs_encoding = filename.bytes().any(|b| b > 127 || b == b'"' || b == b'\\');

                if needs_encoding {
                    // Use UTF-8 encoded filename* parameter
                    let encoded: String = filename
                        .bytes()
                        .map(|b| {
                            if b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_' {
                                format!("{}", b as char)
                            } else {
                                format!("%{:02X}", b)
                            }
                        })
                        .collect();
                    format!("{}; filename*=UTF-8''{}", disposition_type, encoded)
                } else {
                    format!("{}; filename=\"{}\"", disposition_type, filename)
                }
            }
            None => disposition_type.to_string(),
        }
    }

    /// Builds the HTTP response.
    #[must_use]
    pub fn into_response(self) -> Response<Bytes> {
        let content_type = self.get_content_type().to_string();
        let content_disposition = self.build_content_disposition();
        let content_length = self.data.len();

        Response::builder()
            .status(self.status)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_DISPOSITION, content_disposition)
            .header(header::CONTENT_LENGTH, content_length)
            .body(Bytes::from(self.data))
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
        assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/target");
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
        let response = ErrorResponse::not_found("User not found").with_request_id("req-123");

        let http_response = response.into_response();
        let body: serde_json::Value = serde_json::from_slice(http_response.body()).unwrap();

        assert_eq!(body["request_id"], "req-123");
    }

    #[test]
    fn test_file_response_basic() {
        let data = b"Hello, World!".to_vec();
        let response = FileResponse::new(data.clone());

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.len(), 13);
        assert!(!response.is_empty());
        assert_eq!(response.get_content_type(), "application/octet-stream");
    }

    #[test]
    fn test_file_response_with_filename() {
        let response = FileResponse::new(b"content".to_vec())
            .filename("document.pdf");

        assert_eq!(response.get_filename(), Some("document.pdf"));
        assert_eq!(response.get_content_type(), "application/pdf");
    }

    #[test]
    fn test_file_response_explicit_content_type() {
        let response = FileResponse::new(b"content".to_vec())
            .filename("file.txt")
            .content_type("application/custom");

        // Explicit content type takes precedence
        assert_eq!(response.get_content_type(), "application/custom");
    }

    #[test]
    fn test_file_response_inline() {
        let response = FileResponse::new(b"content".to_vec())
            .filename("image.png")
            .inline();

        assert_eq!(response.get_disposition(), ContentDisposition::Inline);

        let http_response = response.into_response();
        let disposition = http_response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap();

        assert!(disposition.starts_with("inline"));
        assert!(disposition.contains("filename=\"image.png\""));
    }

    #[test]
    fn test_file_response_attachment() {
        let response = FileResponse::new(b"content".to_vec())
            .filename("doc.pdf")
            .attachment();

        let http_response = response.into_response();
        let disposition = http_response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap();

        assert!(disposition.starts_with("attachment"));
    }

    #[test]
    fn test_file_response_into_response() {
        let data = b"file content".to_vec();
        let response = FileResponse::new(data.clone())
            .filename("test.txt")
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain"
        );
        assert_eq!(
            response.headers().get(header::CONTENT_LENGTH).unwrap(),
            "12"
        );
        assert_eq!(response.body().as_ref(), b"file content");
    }

    #[test]
    fn test_file_response_from_string() {
        let response = FileResponse::from_string("Hello!")
            .filename("greeting.txt");

        assert_eq!(response.len(), 6);
        assert_eq!(response.get_content_type(), "text/plain");
    }

    #[test]
    fn test_file_response_custom_status() {
        let response = FileResponse::new(b"data".to_vec())
            .with_status(StatusCode::CREATED);

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[test]
    fn test_file_response_mime_detection() {
        // Images
        assert_eq!(
            FileResponse::new(vec![]).filename("photo.jpg").get_content_type(),
            "image/jpeg"
        );
        assert_eq!(
            FileResponse::new(vec![]).filename("image.png").get_content_type(),
            "image/png"
        );
        assert_eq!(
            FileResponse::new(vec![]).filename("icon.svg").get_content_type(),
            "image/svg+xml"
        );

        // Documents
        assert_eq!(
            FileResponse::new(vec![]).filename("doc.pdf").get_content_type(),
            "application/pdf"
        );
        assert_eq!(
            FileResponse::new(vec![]).filename("data.json").get_content_type(),
            "application/json"
        );

        // Archives
        assert_eq!(
            FileResponse::new(vec![]).filename("archive.zip").get_content_type(),
            "application/zip"
        );

        // Unknown extension
        assert_eq!(
            FileResponse::new(vec![]).filename("file.xyz").get_content_type(),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_file_response_unicode_filename() {
        let response = FileResponse::new(b"data".to_vec())
            .filename("документ.pdf");

        let http_response = response.into_response();
        let disposition = http_response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap();

        // Should use UTF-8 encoding
        assert!(disposition.contains("filename*=UTF-8''"));
    }

    #[test]
    fn test_file_response_no_filename() {
        let response = FileResponse::new(b"data".to_vec());

        let http_response = response.into_response();
        let disposition = http_response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(disposition, "attachment");
    }

    #[test]
    fn test_file_response_empty() {
        let response = FileResponse::new(Vec::new());
        assert!(response.is_empty());
        assert_eq!(response.len(), 0);
    }

    #[test]
    fn test_content_disposition_default() {
        let disposition = ContentDisposition::default();
        assert_eq!(disposition, ContentDisposition::Attachment);
    }
}
