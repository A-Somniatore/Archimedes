//! Error types for Archimedes.
//!
//! This module provides the [`ThemisError`] type, which is the standard error
//! type used throughout the Archimedes framework.
//!
//! # `ErrorCategory` vs `ErrorCode`
//!
//! This crate currently uses [`ErrorCategory`] for error classification. This is being
//! unified with `ErrorCode` from `themis-platform-types` in V1.1. The mapping is:
//!
//! | `ErrorCategory` | `ErrorCode` (Target) |
//! |---|---|
//! | `Validation` | `ValidationError` |
//! | `Authentication` | `AuthenticationError` |
//! | `Authorization` | `AuthorizationDenied` |
//! | `NotFound` | `ResourceNotFound` |
//! | `RateLimited` | `RateLimitExceeded` |
//! | `Internal` | `InternalServerError` |
//! | `External` | `ExternalServiceError` |
//! | `Timeout` | `TimeoutError` |
//! | `Conflict` | `ConflictError` |
//!
//! Until V1.1, `ErrorCategory` is serialized with `snake_case` names that match the
//! JSON envelope format.

use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Result type alias using [`ThemisError`].
pub type ThemisResult<T> = Result<T, ThemisError>;

/// Categories of errors for classification and handling.
///
/// # Note: Unification with `ErrorCode`
///
/// This enum is being unified with `ErrorCode` from `themis-platform-types` in V1.1.
/// See module-level documentation for the mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Request validation errors (invalid input, schema mismatch).
    Validation,
    /// Authentication errors (invalid/missing credentials).
    Authentication,
    /// Authorization errors (permission denied).
    Authorization,
    /// Resource not found.
    NotFound,
    /// Rate limiting.
    RateLimited,
    /// Internal server errors.
    Internal,
    /// External service errors (downstream failures).
    External,
    /// Request timeout.
    Timeout,
    /// Conflict (e.g., concurrent modification).
    Conflict,
}

impl ErrorCategory {
    /// Returns the default HTTP status code for this error category.
    #[must_use]
    pub const fn default_status_code(&self) -> StatusCode {
        match self {
            Self::Validation => StatusCode::BAD_REQUEST,
            Self::Authentication => StatusCode::UNAUTHORIZED,
            Self::Authorization => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::External => StatusCode::BAD_GATEWAY,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Conflict => StatusCode::CONFLICT,
        }
    }
}

/// Standard error type for Archimedes.
///
/// `ThemisError` provides structured errors with:
/// - Error categorization
/// - HTTP status code mapping
/// - Serializable error envelope for responses
/// - Error chaining support
///
/// # Example
///
/// ```
/// use archimedes_core::{ThemisError, ErrorCategory};
///
/// fn validate_request(data: &str) -> Result<(), ThemisError> {
///     if data.is_empty() {
///         return Err(ThemisError::validation("Data cannot be empty"));
///     }
///     Ok(())
/// }
/// ```
#[derive(Error, Debug)]
pub enum ThemisError {
    /// Request validation failed.
    #[error("Validation error: {message}")]
    Validation {
        /// Human-readable error message.
        message: String,
        /// Field-specific validation errors.
        #[source]
        field_errors: Option<FieldErrors>,
    },

    /// Authentication failed.
    #[error("Authentication error: {message}")]
    Authentication {
        /// Human-readable error message.
        message: String,
    },

    /// Authorization denied.
    #[error("Authorization denied: {message}")]
    Authorization {
        /// Human-readable error message.
        message: String,
        /// The operation that was denied.
        operation_id: Option<String>,
    },

    /// Resource not found.
    #[error("Not found: {message}")]
    NotFound {
        /// Human-readable error message.
        message: String,
        /// The type of resource that was not found.
        resource_type: Option<String>,
        /// The identifier of the resource.
        resource_id: Option<String>,
    },

    /// Rate limit exceeded.
    #[error("Rate limited: {message}")]
    RateLimited {
        /// Human-readable error message.
        message: String,
        /// Seconds until the rate limit resets.
        retry_after_seconds: Option<u64>,
    },

    /// Internal server error.
    #[error("Internal error: {message}")]
    Internal {
        /// Human-readable error message.
        message: String,
        /// The underlying error (not exposed to clients).
        #[source]
        source: Option<anyhow::Error>,
    },

    /// External service error.
    #[error("External service error: {message}")]
    External {
        /// Human-readable error message.
        message: String,
        /// The name of the external service.
        service: Option<String>,
    },

    /// Request timeout.
    #[error("Timeout: {message}")]
    Timeout {
        /// Human-readable error message.
        message: String,
    },

    /// Conflict error (e.g., concurrent modification).
    #[error("Conflict: {message}")]
    Conflict {
        /// Human-readable error message.
        message: String,
    },
}

impl ThemisError {
    /// Creates a validation error with a message.
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field_errors: None,
        }
    }

    /// Creates a validation error with field-specific errors.
    #[must_use]
    pub fn validation_with_fields(message: impl Into<String>, field_errors: FieldErrors) -> Self {
        Self::Validation {
            message: message.into(),
            field_errors: Some(field_errors),
        }
    }

    /// Creates an authentication error.
    #[must_use]
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Creates an authorization error.
    #[must_use]
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
            operation_id: None,
        }
    }

    /// Creates an authorization error with operation context.
    #[must_use]
    pub fn authorization_for_operation(
        message: impl Into<String>,
        operation_id: impl Into<String>,
    ) -> Self {
        Self::Authorization {
            message: message.into(),
            operation_id: Some(operation_id.into()),
        }
    }

    /// Creates a not found error.
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            resource_type: None,
            resource_id: None,
        }
    }

    /// Creates a not found error with resource context.
    #[must_use]
    pub fn not_found_resource(
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        let resource_type = resource_type.into();
        let resource_id = resource_id.into();
        Self::NotFound {
            message: format!("{resource_type} with ID '{resource_id}' not found"),
            resource_type: Some(resource_type),
            resource_id: Some(resource_id),
        }
    }

    /// Creates a rate limited error.
    #[must_use]
    pub fn rate_limited(message: impl Into<String>, retry_after_seconds: Option<u64>) -> Self {
        Self::RateLimited {
            message: message.into(),
            retry_after_seconds,
        }
    }

    /// Creates an internal error.
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Creates an internal error with a source error.
    pub fn internal_with_source(
        message: impl Into<String>,
        source: impl Into<anyhow::Error>,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Creates an external service error.
    #[must_use]
    pub fn external(message: impl Into<String>, service: Option<impl Into<String>>) -> Self {
        Self::External {
            message: message.into(),
            service: service.map(Into::into),
        }
    }

    /// Creates a timeout error.
    #[must_use]
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    /// Creates a conflict error.
    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// Returns the error category.
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::Validation { .. } => ErrorCategory::Validation,
            Self::Authentication { .. } => ErrorCategory::Authentication,
            Self::Authorization { .. } => ErrorCategory::Authorization,
            Self::NotFound { .. } => ErrorCategory::NotFound,
            Self::RateLimited { .. } => ErrorCategory::RateLimited,
            Self::Internal { .. } => ErrorCategory::Internal,
            Self::External { .. } => ErrorCategory::External,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::Conflict { .. } => ErrorCategory::Conflict,
        }
    }

    /// Returns the HTTP status code for this error.
    #[must_use]
    pub const fn status_code(&self) -> StatusCode {
        self.category().default_status_code()
    }

    /// Converts this error to a serializable error envelope.
    #[must_use]
    pub fn to_envelope(&self, request_id: Option<&str>) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorDetail {
                code: self.error_code(),
                message: self.to_string(),
                category: self.category(),
                details: self.error_details(),
            },
            request_id: request_id.map(ToString::to_string),
        }
    }

    /// Returns a machine-readable error code.
    #[must_use]
    fn error_code(&self) -> String {
        match self {
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::Authentication { .. } => "AUTHENTICATION_ERROR",
            Self::Authorization { .. } => "AUTHORIZATION_DENIED",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::Internal { .. } => "INTERNAL_ERROR",
            Self::External { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::Timeout { .. } => "TIMEOUT",
            Self::Conflict { .. } => "CONFLICT",
        }
        .to_string()
    }

    /// Returns additional error details for the envelope.
    #[must_use]
    fn error_details(&self) -> Option<serde_json::Value> {
        match self {
            Self::Validation {
                field_errors: Some(errors),
                ..
            } => serde_json::to_value(errors).ok(),
            Self::NotFound {
                resource_type: Some(rt),
                resource_id: Some(rid),
                ..
            } => Some(serde_json::json!({
                "resource_type": rt,
                "resource_id": rid
            })),
            Self::RateLimited {
                retry_after_seconds: Some(seconds),
                ..
            } => Some(serde_json::json!({
                "retry_after_seconds": seconds
            })),
            Self::Authorization {
                operation_id: Some(op),
                ..
            } => Some(serde_json::json!({
                "operation_id": op
            })),
            Self::External {
                service: Some(svc), ..
            } => Some(serde_json::json!({
                "service": svc
            })),
            _ => None,
        }
    }
}

/// Field-specific validation errors.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Error)]
#[error("Field validation errors")]
pub struct FieldErrors {
    /// Map of field path to list of error messages.
    pub fields: HashMap<String, Vec<String>>,
}

impl FieldErrors {
    /// Creates a new empty `FieldErrors`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an error for a field.
    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.fields
            .entry(field.into())
            .or_default()
            .push(message.into());
    }

    /// Returns `true` if there are no field errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns the number of fields with errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }
}

/// Serializable error envelope for HTTP responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    /// The error details.
    pub error: ErrorDetail,
    /// The request ID for correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Error detail within an envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Error category.
    pub category: ErrorCategory,
    /// Additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let error = ThemisError::validation("Invalid email format");
        assert_eq!(error.category(), ErrorCategory::Validation);
        assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        assert!(error.to_string().contains("Invalid email format"));
    }

    #[test]
    fn test_validation_error_with_fields() {
        let mut field_errors = FieldErrors::new();
        field_errors.add("email", "Invalid format");
        field_errors.add("email", "Must not be empty");
        field_errors.add("name", "Too long");

        let error = ThemisError::validation_with_fields("Validation failed", field_errors);
        assert_eq!(error.category(), ErrorCategory::Validation);

        let envelope = error.to_envelope(Some("req-123"));
        assert!(envelope.error.details.is_some());
    }

    #[test]
    fn test_authorization_error() {
        let error = ThemisError::authorization_for_operation("Access denied", "deleteUser");
        assert_eq!(error.category(), ErrorCategory::Authorization);
        assert_eq!(error.status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_not_found_resource() {
        let error = ThemisError::not_found_resource("User", "user-123");
        assert_eq!(error.category(), ErrorCategory::NotFound);
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        assert!(error.to_string().contains("user-123"));
    }

    #[test]
    fn test_rate_limited() {
        let error = ThemisError::rate_limited("Too many requests", Some(60));
        assert_eq!(error.category(), ErrorCategory::RateLimited);
        assert_eq!(error.status_code(), StatusCode::TOO_MANY_REQUESTS);

        let envelope = error.to_envelope(None);
        let details = envelope.error.details.unwrap();
        assert_eq!(details["retry_after_seconds"], 60);
    }

    #[test]
    fn test_internal_error() {
        let error = ThemisError::internal("Something went wrong");
        assert_eq!(error.category(), ErrorCategory::Internal);
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_envelope_serialization() {
        let error = ThemisError::not_found("Resource not found");
        let envelope = error.to_envelope(Some("req-456"));

        let json = serde_json::to_string(&envelope).expect("serialization should work");
        assert!(json.contains("\"code\":\"NOT_FOUND\""));
        assert!(json.contains("\"request_id\":\"req-456\""));
        assert!(json.contains("\"category\":\"not_found\""));
    }

    #[test]
    fn test_field_errors() {
        let mut errors = FieldErrors::new();
        assert!(errors.is_empty());

        errors.add("email", "Invalid format");
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 1);

        errors.add("email", "Required");
        assert_eq!(errors.fields["email"].len(), 2);
    }

    #[test]
    fn test_all_error_categories_have_status_codes() {
        let categories = [
            ErrorCategory::Validation,
            ErrorCategory::Authentication,
            ErrorCategory::Authorization,
            ErrorCategory::NotFound,
            ErrorCategory::RateLimited,
            ErrorCategory::Internal,
            ErrorCategory::External,
            ErrorCategory::Timeout,
            ErrorCategory::Conflict,
        ];

        for category in categories {
            let status = category.default_status_code();
            assert!(
                status.is_client_error() || status.is_server_error(),
                "Category {:?} should map to error status code, got {}",
                category,
                status
            );
        }
    }

    /// Test documenting the expected ErrorCode mapping (V1.1).
    ///
    /// This test documents how `ErrorCategory` will be unified with `ErrorCode`
    /// from `themis-platform-types` in V1.1.
    #[test]
    #[allow(deprecated)]
    fn test_error_category_to_error_code_mapping_v1_1() {
        // Once themis-platform-types::ErrorCode is available, this mapping should be used:
        let mappings = [
            ("Validation", "ValidationError"),
            ("Authentication", "AuthenticationError"),
            ("Authorization", "AuthorizationDenied"),
            ("NotFound", "ResourceNotFound"),
            ("RateLimited", "RateLimitExceeded"),
            ("Internal", "InternalServerError"),
            ("External", "ExternalServiceError"),
            ("Timeout", "TimeoutError"),
            ("Conflict", "ConflictError"),
        ];

        // Verify all categories have mappings
        assert_eq!(mappings.len(), 9);

        // Current state: ErrorCategory has 9 variants
        let categories = [
            ErrorCategory::Validation,
            ErrorCategory::Authentication,
            ErrorCategory::Authorization,
            ErrorCategory::NotFound,
            ErrorCategory::RateLimited,
            ErrorCategory::Internal,
            ErrorCategory::External,
            ErrorCategory::Timeout,
            ErrorCategory::Conflict,
        ];
        assert_eq!(categories.len(), mappings.len(), "Mapping incomplete");
    }
}
