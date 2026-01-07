//! Error types for the authorization crate.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for authorization operations.
pub type AuthzResult<T> = Result<T, AuthzError>;

/// Errors that can occur during authorization.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AuthzError {
    /// Bundle loading failed.
    #[error("failed to load bundle from {path}: {message}")]
    BundleLoad {
        /// Path to the bundle.
        path: PathBuf,
        /// Error message.
        message: String,
    },

    /// Bundle parsing failed.
    #[error("failed to parse bundle: {0}")]
    BundleParse(String),

    /// Policy evaluation failed.
    #[error("policy evaluation failed: {0}")]
    Evaluation(String),

    /// Policy not found.
    #[error("policy not found: {0}")]
    PolicyNotFound(String),

    /// Invalid policy input.
    #[error("invalid policy input: {0}")]
    InvalidInput(String),

    /// Access denied by policy.
    #[error("access denied: {reason}")]
    AccessDenied {
        /// Reason for denial.
        reason: String,
    },

    /// Registry communication error.
    #[error("registry error: {0}")]
    Registry(String),

    /// Cache error.
    #[error("cache error: {0}")]
    Cache(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl AuthzError {
    /// Create a bundle load error.
    pub fn bundle_load(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::BundleLoad {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create an access denied error.
    pub fn access_denied(reason: impl Into<String>) -> Self {
        Self::AccessDenied {
            reason: reason.into(),
        }
    }

    /// Check if this is an access denied error.
    pub const fn is_access_denied(&self) -> bool {
        matches!(self, Self::AccessDenied { .. })
    }

    /// Check if this is a retryable error.
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Registry(_) | Self::Io(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_load_error() {
        let err = AuthzError::bundle_load("/path/to/bundle", "file not found");
        assert!(err.to_string().contains("bundle"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_access_denied_error() {
        let err = AuthzError::access_denied("insufficient permissions");
        assert!(err.is_access_denied());
        assert!(err.to_string().contains("insufficient permissions"));
    }

    #[test]
    fn test_retryable_error() {
        let registry_err = AuthzError::Registry("connection timeout".to_string());
        assert!(registry_err.is_retryable());

        let eval_err = AuthzError::Evaluation("syntax error".to_string());
        assert!(!eval_err.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = AuthzError::PolicyNotFound("authz.allow".to_string());
        assert_eq!(err.to_string(), "policy not found: authz.allow");
    }
}
