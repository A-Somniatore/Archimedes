//! Extraction error types.
//!
//! This module provides error types for extraction failures,
//! including information about the source of the error.

use http::StatusCode;
use std::fmt;

/// Source of extraction (where data was being extracted from).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionSource {
    /// Path parameters (e.g., `/users/{id}`)
    Path,
    /// Query string parameters
    Query,
    /// Request body (JSON, form, etc.)
    Body,
    /// HTTP headers
    Header,
    /// Content-Type header specifically
    ContentType,
    /// Other sources (e.g., DI container)
    Other,
}

impl fmt::Display for ExtractionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path => write!(f, "path"),
            Self::Query => write!(f, "query"),
            Self::Body => write!(f, "body"),
            Self::Header => write!(f, "header"),
            Self::ContentType => write!(f, "content-type"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// Error that occurs during extraction.
///
/// Contains information about the source of the error and what went wrong.
/// Can be converted to an appropriate HTTP status code for error responses.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{ExtractionError, ExtractionSource};
/// use http::StatusCode;
///
/// let err = ExtractionError::missing(ExtractionSource::Path, "user_id");
/// assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
/// assert_eq!(err.extraction_source(), ExtractionSource::Path);
/// assert!(err.to_string().contains("user_id"));
/// ```
#[derive(Debug)]
pub struct ExtractionError {
    extraction_source: ExtractionSource,
    kind: ExtractionErrorKind,
    field: Option<String>,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtractionErrorKind {
    /// Required field or parameter is missing
    Missing,
    /// Value has invalid type or format
    InvalidType,
    /// Value failed validation
    ValidationFailed,
    /// Deserialization failed
    DeserializationFailed,
    /// Body is too large
    PayloadTooLarge,
    /// Content-Type is unsupported
    UnsupportedMediaType,
    /// Custom error (e.g., DI failure)
    Custom,
}

impl ExtractionError {
    /// Creates an error for a missing field or parameter.
    #[must_use]
    pub fn missing(source: ExtractionSource, field: impl Into<String>) -> Self {
        let field = field.into();
        Self {
            extraction_source: source,
            kind: ExtractionErrorKind::Missing,
            message: format!("missing required {source} parameter: {field}"),
            field: Some(field),
        }
    }

    /// Creates an error for an invalid type or format.
    #[must_use]
    pub fn invalid_type(
        source: ExtractionSource,
        field: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        let field = field.into();
        let details = details.into();
        Self {
            extraction_source: source,
            kind: ExtractionErrorKind::InvalidType,
            message: format!("invalid {source} parameter '{field}': {details}"),
            field: Some(field),
        }
    }

    /// Creates an error for a validation failure.
    #[must_use]
    pub fn validation_failed(
        source: ExtractionSource,
        field: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        let field = field.into();
        let details = details.into();
        Self {
            extraction_source: source,
            kind: ExtractionErrorKind::ValidationFailed,
            message: format!("validation failed for {source} parameter '{field}': {details}"),
            field: Some(field),
        }
    }

    /// Creates an error for deserialization failure.
    #[must_use]
    pub fn deserialization_failed(source: ExtractionSource, error: impl Into<String>) -> Self {
        let error = error.into();
        Self {
            extraction_source: source,
            kind: ExtractionErrorKind::DeserializationFailed,
            message: format!("failed to deserialize {source}: {error}"),
            field: None,
        }
    }

    /// Creates an error for a payload that's too large.
    #[must_use]
    pub fn payload_too_large(max_size: usize, actual_size: usize) -> Self {
        Self {
            extraction_source: ExtractionSource::Body,
            kind: ExtractionErrorKind::PayloadTooLarge,
            message: format!(
                "payload too large: max {max_size} bytes, got {actual_size} bytes"
            ),
            field: None,
        }
    }

    /// Creates an error for unsupported content type.
    #[must_use]
    pub fn unsupported_media_type(expected: &str, actual: Option<&str>) -> Self {
        let actual_str = actual.unwrap_or("none");
        Self {
            extraction_source: ExtractionSource::ContentType,
            kind: ExtractionErrorKind::UnsupportedMediaType,
            message: format!(
                "unsupported content type: expected '{expected}', got '{actual_str}'"
            ),
            field: None,
        }
    }

    /// Creates a custom error.
    ///
    /// Use this for errors that don't fit the other categories,
    /// such as dependency injection failures.
    #[must_use]
    pub fn custom(
        source: ExtractionSource,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let field = field.into();
        Self {
            extraction_source: source,
            kind: ExtractionErrorKind::Custom,
            message: message.into(),
            field: Some(field),
        }
    }

    /// Returns the extraction source.
    #[must_use]
    pub fn extraction_source(&self) -> ExtractionSource {
        self.extraction_source
    }

    /// Alias for `extraction_source` for backwards compatibility.
    #[must_use]
    pub fn source(&self) -> ExtractionSource {
        self.extraction_source
    }

    /// Returns the field name if applicable.
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
    }

    /// Returns the appropriate HTTP status code for this error.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self.kind {
            ExtractionErrorKind::Missing => StatusCode::BAD_REQUEST,
            ExtractionErrorKind::InvalidType => StatusCode::BAD_REQUEST,
            ExtractionErrorKind::ValidationFailed => StatusCode::UNPROCESSABLE_ENTITY,
            ExtractionErrorKind::DeserializationFailed => StatusCode::BAD_REQUEST,
            ExtractionErrorKind::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ExtractionErrorKind::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ExtractionErrorKind::Custom => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Returns the error code suitable for error envelopes.
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self.kind {
            ExtractionErrorKind::Missing => "MISSING_PARAMETER",
            ExtractionErrorKind::InvalidType => "INVALID_PARAMETER",
            ExtractionErrorKind::ValidationFailed => "VALIDATION_FAILED",
            ExtractionErrorKind::DeserializationFailed => "DESERIALIZATION_FAILED",
            ExtractionErrorKind::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            ExtractionErrorKind::UnsupportedMediaType => "UNSUPPORTED_MEDIA_TYPE",
            ExtractionErrorKind::Custom => "EXTRACTION_FAILED",
        }
    }
}

impl fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExtractionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_error() {
        let err = ExtractionError::missing(ExtractionSource::Path, "user_id");

        assert_eq!(err.source(), ExtractionSource::Path);
        assert_eq!(err.field(), Some("user_id"));
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code(), "MISSING_PARAMETER");
        assert!(err.to_string().contains("user_id"));
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn test_invalid_type_error() {
        let err = ExtractionError::invalid_type(
            ExtractionSource::Query,
            "limit",
            "expected integer",
        );

        assert_eq!(err.source(), ExtractionSource::Query);
        assert_eq!(err.field(), Some("limit"));
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code(), "INVALID_PARAMETER");
        assert!(err.to_string().contains("limit"));
        assert!(err.to_string().contains("expected integer"));
    }

    #[test]
    fn test_validation_failed_error() {
        let err = ExtractionError::validation_failed(
            ExtractionSource::Body,
            "email",
            "invalid email format",
        );

        assert_eq!(err.source(), ExtractionSource::Body);
        assert_eq!(err.field(), Some("email"));
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.error_code(), "VALIDATION_FAILED");
    }

    #[test]
    fn test_deserialization_failed_error() {
        let err = ExtractionError::deserialization_failed(
            ExtractionSource::Body,
            "unexpected token at position 5",
        );

        assert_eq!(err.source(), ExtractionSource::Body);
        assert_eq!(err.field(), None);
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code(), "DESERIALIZATION_FAILED");
    }

    #[test]
    fn test_payload_too_large_error() {
        let err = ExtractionError::payload_too_large(1024, 2048);

        assert_eq!(err.source(), ExtractionSource::Body);
        assert_eq!(err.status_code(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(err.error_code(), "PAYLOAD_TOO_LARGE");
        assert!(err.to_string().contains("1024"));
        assert!(err.to_string().contains("2048"));
    }

    #[test]
    fn test_unsupported_media_type_error() {
        let err = ExtractionError::unsupported_media_type(
            "application/json",
            Some("text/plain"),
        );

        assert_eq!(err.source(), ExtractionSource::ContentType);
        assert_eq!(err.status_code(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(err.error_code(), "UNSUPPORTED_MEDIA_TYPE");
        assert!(err.to_string().contains("application/json"));
        assert!(err.to_string().contains("text/plain"));
    }

    #[test]
    fn test_extraction_source_display() {
        assert_eq!(ExtractionSource::Path.to_string(), "path");
        assert_eq!(ExtractionSource::Query.to_string(), "query");
        assert_eq!(ExtractionSource::Body.to_string(), "body");
        assert_eq!(ExtractionSource::Header.to_string(), "header");
        assert_eq!(ExtractionSource::ContentType.to_string(), "content-type");
    }
}
