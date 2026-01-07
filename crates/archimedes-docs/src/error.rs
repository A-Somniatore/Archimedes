//! Error types for the documentation generation crate.
//!
//! This module defines errors that can occur during OpenAPI generation,
//! schema conversion, and documentation serving.

use thiserror::Error;

/// Errors that can occur during documentation generation.
#[derive(Debug, Error)]
pub enum DocsError {
    /// Failed to serialize OpenAPI spec to JSON.
    #[error("Failed to serialize OpenAPI spec: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Contract is missing required information.
    #[error("Contract missing required field: {field}")]
    MissingField {
        /// The name of the missing field.
        field: String,
    },

    /// Schema conversion failed.
    #[error("Failed to convert schema: {reason}")]
    SchemaConversionError {
        /// The reason for the conversion failure.
        reason: String,
    },

    /// Invalid operation definition in contract.
    #[error("Invalid operation '{operation_id}': {reason}")]
    InvalidOperation {
        /// The operation ID that is invalid.
        operation_id: String,
        /// The reason the operation is invalid.
        reason: String,
    },

    /// IO error when reading or writing files.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for documentation operations.
pub type DocsResult<T> = Result<T, DocsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_error() {
        let err: DocsError = serde_json::from_str::<String>("invalid")
            .unwrap_err()
            .into();
        assert!(matches!(err, DocsError::SerializationError(_)));
        assert!(err.to_string().contains("serialize"));
    }

    #[test]
    fn test_missing_field_error() {
        let err = DocsError::MissingField {
            field: "title".to_string(),
        };
        assert!(err.to_string().contains("title"));
    }

    #[test]
    fn test_schema_conversion_error() {
        let err = DocsError::SchemaConversionError {
            reason: "unsupported type".to_string(),
        };
        assert!(err.to_string().contains("unsupported type"));
    }

    #[test]
    fn test_invalid_operation_error() {
        let err = DocsError::InvalidOperation {
            operation_id: "getUser".to_string(),
            reason: "missing path".to_string(),
        };
        assert!(err.to_string().contains("getUser"));
        assert!(err.to_string().contains("missing path"));
    }
}
