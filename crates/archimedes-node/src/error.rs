//! Error types for the Archimedes Node.js bindings.

use napi_derive::napi;
use std::fmt;

/// Archimedes error exposed to JavaScript.
///
/// Wraps internal errors from validation, authorization, and server operations.
#[napi]
#[derive(Debug, Clone)]
pub struct ArchimedesError {
    /// Error kind/category
    kind: String,
    /// Human-readable error message
    message: String,
    /// Optional error code for programmatic handling
    code: Option<String>,
    /// Optional additional details
    details: Option<String>,
}

#[napi]
impl ArchimedesError {
    /// Create a new error.
    #[napi(constructor)]
    pub fn new(kind: String, message: String) -> Self {
        Self {
            kind,
            message,
            code: None,
            details: None,
        }
    }

    /// Create a validation error.
    #[napi(factory)]
    pub fn validation(message: String) -> Self {
        Self {
            kind: "ValidationError".to_string(),
            message,
            code: Some("VALIDATION_FAILED".to_string()),
            details: None,
        }
    }

    /// Create an authorization error.
    #[napi(factory)]
    pub fn authorization(message: String) -> Self {
        Self {
            kind: "AuthorizationError".to_string(),
            message,
            code: Some("AUTHORIZATION_DENIED".to_string()),
            details: None,
        }
    }

    /// Create a not found error.
    #[napi(factory)]
    pub fn not_found(message: String) -> Self {
        Self {
            kind: "NotFoundError".to_string(),
            message,
            code: Some("NOT_FOUND".to_string()),
            details: None,
        }
    }

    /// Create an internal server error.
    #[napi(factory)]
    pub fn internal(message: String) -> Self {
        Self {
            kind: "InternalError".to_string(),
            message,
            code: Some("INTERNAL_ERROR".to_string()),
            details: None,
        }
    }

    /// Create an operation not found error.
    #[napi(factory)]
    pub fn operation_not_found(operation_id: String) -> Self {
        Self {
            kind: "OperationNotFoundError".to_string(),
            message: format!("Operation '{}' not found in contract", operation_id),
            code: Some("OPERATION_NOT_FOUND".to_string()),
            details: None,
        }
    }

    /// Create a handler not registered error.
    #[napi(factory)]
    pub fn handler_not_registered(operation_id: String) -> Self {
        Self {
            kind: "HandlerNotRegisteredError".to_string(),
            message: format!("No handler registered for operation '{}'", operation_id),
            code: Some("HANDLER_NOT_REGISTERED".to_string()),
            details: None,
        }
    }

    /// Get the error kind.
    #[napi(getter)]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    /// Get the error message.
    #[napi(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// Get the error code.
    #[napi(getter)]
    pub fn code(&self) -> Option<String> {
        self.code.clone()
    }

    /// Get additional details.
    #[napi(getter)]
    pub fn details(&self) -> Option<String> {
        self.details.clone()
    }

    /// Set additional details.
    #[napi(setter)]
    pub fn set_details(&mut self, details: Option<String>) {
        self.details = details;
    }

    /// Convert to a JavaScript Error.
    #[napi]
    pub fn to_js_error(&self) -> napi::Error {
        napi::Error::new(napi::Status::GenericFailure, self.to_string())
    }
}

impl fmt::Display for ArchimedesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = &self.code {
            write!(f, "[{}] {}: {}", code, self.kind, self.message)
        } else {
            write!(f, "{}: {}", self.kind, self.message)
        }
    }
}

impl From<ArchimedesError> for napi::Error {
    fn from(err: ArchimedesError) -> Self {
        napi::Error::new(napi::Status::GenericFailure, err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ArchimedesError::new("TestError".to_string(), "test message".to_string());
        assert_eq!(err.kind(), "TestError");
        assert_eq!(err.message(), "test message");
        assert!(err.code().is_none());
    }

    #[test]
    fn test_validation_error() {
        let err = ArchimedesError::validation("invalid input".to_string());
        assert_eq!(err.kind(), "ValidationError");
        assert_eq!(err.code(), Some("VALIDATION_FAILED".to_string()));
    }

    #[test]
    fn test_authorization_error() {
        let err = ArchimedesError::authorization("access denied".to_string());
        assert_eq!(err.kind(), "AuthorizationError");
        assert_eq!(err.code(), Some("AUTHORIZATION_DENIED".to_string()));
    }

    #[test]
    fn test_not_found_error() {
        let err = ArchimedesError::not_found("resource not found".to_string());
        assert_eq!(err.kind(), "NotFoundError");
        assert_eq!(err.code(), Some("NOT_FOUND".to_string()));
    }

    #[test]
    fn test_operation_not_found_error() {
        let err = ArchimedesError::operation_not_found("getUser".to_string());
        assert!(err.message().contains("getUser"));
        assert_eq!(err.code(), Some("OPERATION_NOT_FOUND".to_string()));
    }

    #[test]
    fn test_error_display() {
        let err = ArchimedesError::validation("bad input".to_string());
        let display = err.to_string();
        assert!(display.contains("VALIDATION_FAILED"));
        assert!(display.contains("bad input"));
    }
}
