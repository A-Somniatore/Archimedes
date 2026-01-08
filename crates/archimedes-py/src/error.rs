//! Python error types for Archimedes

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

// Create the Python exception type
create_exception!(archimedes, ArchimedesError, PyException);

pub use ArchimedesError as PyArchimedesError;

/// Error kinds that can occur in Archimedes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Configuration error
    Config,
    /// Contract validation error
    Contract,
    /// Request validation error
    Validation,
    /// Authorization error
    Authorization,
    /// Handler error
    Handler,
    /// Server error
    Server,
    /// Internal error
    Internal,
}

impl ErrorKind {
    /// Get the error kind name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Config => "ConfigError",
            Self::Contract => "ContractError",
            Self::Validation => "ValidationError",
            Self::Authorization => "AuthorizationError",
            Self::Handler => "HandlerError",
            Self::Server => "ServerError",
            Self::Internal => "InternalError",
        }
    }
}

/// Create a config error
pub fn config_error(message: impl Into<String>) -> PyErr {
    PyArchimedesError::new_err(format!("ConfigError: {}", message.into()))
}

/// Create a contract error
pub fn contract_error(message: impl Into<String>) -> PyErr {
    PyArchimedesError::new_err(format!("ContractError: {}", message.into()))
}

/// Create a validation error
pub fn validation_error(message: impl Into<String>) -> PyErr {
    PyArchimedesError::new_err(format!("ValidationError: {}", message.into()))
}

/// Create an authorization error
pub fn authorization_error(message: impl Into<String>) -> PyErr {
    PyArchimedesError::new_err(format!("AuthorizationError: {}", message.into()))
}

/// Create a handler error
pub fn handler_error(message: impl Into<String>) -> PyErr {
    PyArchimedesError::new_err(format!("HandlerError: {}", message.into()))
}

/// Create a server error
pub fn server_error(message: impl Into<String>) -> PyErr {
    PyArchimedesError::new_err(format!("ServerError: {}", message.into()))
}

/// Create an internal error
pub fn internal_error(message: impl Into<String>) -> PyErr {
    PyArchimedesError::new_err(format!("InternalError: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind_as_str() {
        assert_eq!(ErrorKind::Config.as_str(), "ConfigError");
        assert_eq!(ErrorKind::Validation.as_str(), "ValidationError");
        assert_eq!(ErrorKind::Authorization.as_str(), "AuthorizationError");
    }

    #[test]
    fn test_error_creation() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|_py| {
            let err = config_error("test message");
            assert!(err.to_string().contains("ConfigError"));
            assert!(err.to_string().contains("test message"));
        });
    }
}
