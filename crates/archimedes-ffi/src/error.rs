//! FFI error handling
//!
//! Error types and conversion for FFI boundary.

use crate::types::ArchimedesError;
use thiserror::Error;

/// Internal error type for FFI operations
#[derive(Error, Debug)]
pub enum FfiError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Failed to load contract: {0}")]
    ContractLoad(String),

    #[error("Failed to load policy bundle: {0}")]
    PolicyLoad(String),

    #[error("Handler registration failed: {0}")]
    HandlerRegistration(String),

    #[error("Server start failed: {0}")]
    ServerStart(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Handler error: {0}")]
    Handler(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Authorization denied: {0}")]
    Authorization(String),

    #[error("Null pointer provided for: {0}")]
    NullPointer(&'static str),

    #[error("Invalid UTF-8 string: {0}")]
    InvalidUtf8(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<&FfiError> for ArchimedesError {
    fn from(err: &FfiError) -> Self {
        match err {
            FfiError::InvalidConfig(_) => ArchimedesError::InvalidConfig,
            FfiError::ContractLoad(_) => ArchimedesError::ContractLoadError,
            FfiError::PolicyLoad(_) => ArchimedesError::PolicyLoadError,
            FfiError::HandlerRegistration(_) => ArchimedesError::HandlerRegistrationError,
            FfiError::ServerStart(_) => ArchimedesError::ServerStartError,
            FfiError::InvalidOperation(_) => ArchimedesError::InvalidOperation,
            FfiError::Handler(_) => ArchimedesError::HandlerError,
            FfiError::Validation(_) => ArchimedesError::ValidationError,
            FfiError::Authorization(_) => ArchimedesError::AuthorizationError,
            FfiError::NullPointer(_) => ArchimedesError::NullPointer,
            FfiError::InvalidUtf8(_) => ArchimedesError::InvalidUtf8,
            FfiError::Internal(_) => ArchimedesError::Internal,
        }
    }
}

impl From<FfiError> for ArchimedesError {
    fn from(err: FfiError) -> Self {
        ArchimedesError::from(&err)
    }
}

/// Convert a Result to an FFI error code, setting last_error if needed
pub(crate) fn result_to_error<T>(result: Result<T, FfiError>) -> (Option<T>, ArchimedesError) {
    match result {
        Ok(value) => (Some(value), ArchimedesError::Ok),
        Err(err) => {
            crate::set_last_error(err.to_string());
            (None, ArchimedesError::from(err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let err = FfiError::InvalidConfig("bad config".to_string());
        assert_eq!(ArchimedesError::from(&err), ArchimedesError::InvalidConfig);
    }

    #[test]
    fn test_result_to_error_ok() {
        let result: Result<i32, FfiError> = Ok(42);
        let (value, code) = result_to_error(result);
        assert_eq!(value, Some(42));
        assert_eq!(code, ArchimedesError::Ok);
    }

    #[test]
    fn test_result_to_error_err() {
        let result: Result<i32, FfiError> = Err(FfiError::NullPointer("test"));
        let (value, code) = result_to_error(result);
        assert_eq!(value, None);
        assert_eq!(code, ArchimedesError::NullPointer);
    }
}
