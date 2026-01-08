//! Sentinel error types.

use std::fmt;

/// Result type for Sentinel operations.
pub type SentinelResult<T> = Result<T, SentinelError>;

/// Errors that can occur during Sentinel operations.
#[derive(Debug)]
pub enum SentinelError {
    /// Failed to load an artifact.
    ArtifactLoad(String),

    /// Failed to parse an artifact.
    ArtifactParse(String),

    /// Artifact checksum verification failed.
    ChecksumMismatch {
        /// Expected checksum.
        expected: String,
        /// Actual checksum.
        actual: String,
    },

    /// No operation found for the given method and path.
    OperationNotFound {
        /// HTTP method.
        method: String,
        /// Request path.
        path: String,
    },

    /// Path parameter extraction failed.
    PathParameterError {
        /// The parameter name.
        parameter: String,
        /// Description of the error.
        message: String,
    },

    /// Request validation failed.
    RequestValidation {
        /// Operation ID.
        operation_id: String,
        /// Validation errors.
        errors: Vec<ValidationError>,
    },

    /// Response validation failed.
    ResponseValidation {
        /// Operation ID.
        operation_id: String,
        /// HTTP status code.
        status_code: u16,
        /// Validation errors.
        errors: Vec<ValidationError>,
    },

    /// Schema not found.
    SchemaNotFound {
        /// The schema reference that was not found.
        reference: String,
    },

    /// IO error.
    Io(std::io::Error),
}

impl std::error::Error for SentinelError {}

impl fmt::Display for SentinelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArtifactLoad(msg) => write!(f, "failed to load artifact: {}", msg),
            Self::ArtifactParse(msg) => write!(f, "failed to parse artifact: {}", msg),
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "checksum mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            Self::OperationNotFound { method, path } => {
                write!(f, "no operation found for {} {}", method, path)
            }
            Self::PathParameterError { parameter, message } => {
                write!(f, "path parameter '{}' error: {}", parameter, message)
            }
            Self::RequestValidation {
                operation_id,
                errors,
            } => {
                write!(
                    f,
                    "request validation failed for '{}': {} error(s)",
                    operation_id,
                    errors.len()
                )
            }
            Self::ResponseValidation {
                operation_id,
                status_code,
                errors,
            } => {
                write!(
                    f,
                    "response validation failed for '{}' (status {}): {} error(s)",
                    operation_id,
                    status_code,
                    errors.len()
                )
            }
            Self::SchemaNotFound { reference } => {
                write!(f, "schema not found: {}", reference)
            }
            Self::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl From<std::io::Error> for SentinelError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// A validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// JSON path to the error location.
    pub path: String,
    /// Error message.
    pub message: String,
    /// Schema path that caused the error.
    pub schema_path: Option<String>,
    /// The invalid value (if available).
    pub value: Option<String>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)?;
        if let Some(ref schema_path) = self.schema_path {
            write!(f, " (schema: {})", schema_path)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_load_error_display() {
        let err = SentinelError::ArtifactLoad("file not found".to_string());
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_operation_not_found_display() {
        let err = SentinelError::OperationNotFound {
            method: "GET".to_string(),
            path: "/users".to_string(),
        };
        assert!(err.to_string().contains("GET"));
        assert!(err.to_string().contains("/users"));
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError {
            path: "body.email".to_string(),
            message: "invalid email format".to_string(),
            schema_path: Some("#/components/schemas/User".to_string()),
            value: Some("not-an-email".to_string()),
        };
        assert!(err.to_string().contains("body.email"));
        assert!(err.to_string().contains("invalid email format"));
    }

    #[test]
    fn test_checksum_mismatch_display() {
        let err = SentinelError::ChecksumMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert!(err.to_string().contains("abc123"));
        assert!(err.to_string().contains("def456"));
    }

    #[test]
    fn test_request_validation_display() {
        let err = SentinelError::RequestValidation {
            operation_id: "createUser".to_string(),
            errors: vec![ValidationError {
                path: "body.name".to_string(),
                message: "required".to_string(),
                schema_path: None,
                value: None,
            }],
        };
        assert!(err.to_string().contains("createUser"));
        assert!(err.to_string().contains("1 error"));
    }
}
