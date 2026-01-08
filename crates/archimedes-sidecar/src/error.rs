//! Error types for the Archimedes sidecar.

use std::fmt;

use thiserror::Error;

/// Sidecar-specific errors.
#[derive(Debug, Error)]
pub enum SidecarError {
    /// Configuration error.
    #[error("Configuration error: {message}")]
    Config {
        /// Error message.
        message: String,
    },

    /// Upstream connection error.
    #[error("Upstream error: {message}")]
    Upstream {
        /// Error message.
        message: String,
        /// Optional HTTP status code from upstream.
        status: Option<u16>,
    },

    /// Proxy error during request forwarding.
    #[error("Proxy error: {message}")]
    Proxy {
        /// Error message.
        message: String,
    },

    /// Contract validation error.
    #[error("Validation error: {message}")]
    Validation {
        /// Error message.
        message: String,
        /// Field that failed validation.
        field: Option<String>,
    },

    /// Authorization denied.
    #[error("Authorization denied: {reason}")]
    AuthorizationDenied {
        /// Reason for denial.
        reason: String,
    },

    /// Health check failure.
    #[error("Health check failed: {message}")]
    HealthCheck {
        /// Error message.
        message: String,
    },

    /// Server startup error.
    #[error("Server error: {message}")]
    Server {
        /// Error message.
        message: String,
    },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP error.
    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Request client error.
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    /// Internal error.
    #[error("Internal error: {message}")]
    Internal {
        /// Error message.
        message: String,
    },
}

impl SidecarError {
    /// Create a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create an upstream error.
    pub fn upstream(message: impl Into<String>) -> Self {
        Self::Upstream {
            message: message.into(),
            status: None,
        }
    }

    /// Create an upstream error with status code.
    pub fn upstream_with_status(message: impl Into<String>, status: u16) -> Self {
        Self::Upstream {
            message: message.into(),
            status: Some(status),
        }
    }

    /// Create a proxy error.
    pub fn proxy(message: impl Into<String>) -> Self {
        Self::Proxy {
            message: message.into(),
        }
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
        }
    }

    /// Create a validation error with field.
    pub fn validation_with_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Create an authorization denied error.
    pub fn authorization_denied(reason: impl Into<String>) -> Self {
        Self::AuthorizationDenied {
            reason: reason.into(),
        }
    }

    /// Create a health check error.
    pub fn health_check(message: impl Into<String>) -> Self {
        Self::HealthCheck {
            message: message.into(),
        }
    }

    /// Create a server error.
    pub fn server(message: impl Into<String>) -> Self {
        Self::Server {
            message: message.into(),
        }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Get the HTTP status code for this error.
    #[allow(clippy::match_same_arms)]
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Config { .. } => 500,
            Self::Upstream { status, .. } => status.unwrap_or(502),
            Self::Proxy { .. } => 502,
            Self::Validation { .. } => 400,
            Self::AuthorizationDenied { .. } => 403,
            Self::HealthCheck { .. } => 503,
            Self::Server { .. } => 500,
            Self::Io(_) => 500,
            Self::Http(_) => 400,
            Self::Json(_) => 400,
            Self::Request(_) => 502,
            Self::Internal { .. } => 500,
        }
    }

    /// Check if this error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Upstream { .. }
                | Self::Proxy { .. }
                | Self::Request(_)
                | Self::HealthCheck { .. }
        )
    }

    /// Get the error category for metrics.
    pub fn category(&self) -> &'static str {
        match self {
            Self::Config { .. } => "config",
            Self::Upstream { .. } => "upstream",
            Self::Proxy { .. } => "proxy",
            Self::Validation { .. } => "validation",
            Self::AuthorizationDenied { .. } => "authorization",
            Self::HealthCheck { .. } => "health",
            Self::Server { .. } => "server",
            Self::Io(_) => "io",
            Self::Http(_) => "http",
            Self::Json(_) => "json",
            Self::Request(_) => "request",
            Self::Internal { .. } => "internal",
        }
    }
}

/// Result type for sidecar operations.
pub type SidecarResult<T> = Result<T, SidecarError>;

/// Error response body for upstream errors.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    /// Error code/category.
    pub error: String,
    /// Human-readable message.
    pub message: String,
    /// Request ID for correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response.
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            request_id: None,
            details: None,
        }
    }

    /// Set the request ID.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set additional details.
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl From<SidecarError> for ErrorResponse {
    fn from(err: SidecarError) -> Self {
        Self::new(err.category(), err.to_string())
    }
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.error, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_constructors() {
        let err = SidecarError::config("missing field");
        assert_eq!(err.status_code(), 500);
        assert_eq!(err.category(), "config");

        let err = SidecarError::upstream("connection refused");
        assert_eq!(err.status_code(), 502);

        let err = SidecarError::upstream_with_status("bad response", 503);
        assert_eq!(err.status_code(), 503);

        let err = SidecarError::validation("invalid JSON");
        assert_eq!(err.status_code(), 400);

        let err = SidecarError::authorization_denied("insufficient permissions");
        assert_eq!(err.status_code(), 403);
    }

    #[test]
    fn test_error_display() {
        let err = SidecarError::config("test");
        assert!(err.to_string().contains("Configuration error"));

        let err = SidecarError::authorization_denied("test reason");
        assert!(err.to_string().contains("Authorization denied"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(SidecarError::upstream("test").is_recoverable());
        assert!(SidecarError::proxy("test").is_recoverable());
        assert!(!SidecarError::config("test").is_recoverable());
        assert!(!SidecarError::validation("test").is_recoverable());
    }

    #[test]
    fn test_error_response() {
        let resp = ErrorResponse::new("validation", "invalid input")
            .with_request_id("req-123")
            .with_details(serde_json::json!({"field": "name"}));

        assert_eq!(resp.error, "validation");
        assert_eq!(resp.message, "invalid input");
        assert_eq!(resp.request_id, Some("req-123".to_string()));

        let err = SidecarError::validation("test");
        let resp: ErrorResponse = err.into();
        assert_eq!(resp.error, "validation");
    }
}
