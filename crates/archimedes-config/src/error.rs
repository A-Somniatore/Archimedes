//! Configuration error types.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration loading.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Configuration file not found.
    #[error("configuration file not found: {path}")]
    FileNotFound {
        /// Path to the missing file.
        path: PathBuf,
    },

    /// Failed to read configuration file.
    #[error("failed to read configuration file: {path}")]
    ReadError {
        /// Path to the file.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// TOML parsing error.
    #[error("failed to parse TOML configuration: {0}")]
    TomlError(#[from] toml::de::Error),

    /// JSON parsing error.
    #[error("failed to parse JSON configuration: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Unknown field in configuration (strict mode).
    #[error("unknown configuration field: {field} in section {section}")]
    UnknownField {
        /// The unknown field name.
        field: String,
        /// The section containing the field.
        section: String,
    },

    /// Invalid configuration value.
    #[error("invalid configuration value for {field}: {reason}")]
    InvalidValue {
        /// The field with the invalid value.
        field: String,
        /// Explanation of why the value is invalid.
        reason: String,
    },

    /// Missing required field.
    #[error("missing required configuration field: {field}")]
    MissingField {
        /// The missing field name.
        field: String,
    },

    /// Environment variable parsing error.
    #[error("failed to parse environment variable {var}: {reason}")]
    EnvParseError {
        /// The environment variable name.
        var: String,
        /// Explanation of the parsing error.
        reason: String,
    },

    /// Validation error after loading.
    #[error("configuration validation failed: {0}")]
    ValidationError(String),

    /// Invalid configuration for a component.
    #[error("invalid configuration: {message}")]
    InvalidConfig {
        /// Description of the configuration error.
        message: String,
    },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl ConfigError {
    /// Create a new file not found error.
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// Create a new read error.
    pub fn read_error(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::ReadError {
            path: path.into(),
            source,
        }
    }

    /// Create a new unknown field error.
    pub fn unknown_field(field: impl Into<String>, section: impl Into<String>) -> Self {
        Self::UnknownField {
            field: field.into(),
            section: section.into(),
        }
    }

    /// Create a new invalid value error.
    pub fn invalid_value(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidValue {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// Create a new missing field error.
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Create a new environment variable parse error.
    pub fn env_parse_error(var: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::EnvParseError {
            var: var.into(),
            reason: reason.into(),
        }
    }

    /// Create a new validation error.
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_not_found_error() {
        let err = ConfigError::file_not_found("/path/to/config.toml");
        assert!(err.to_string().contains("/path/to/config.toml"));
    }

    #[test]
    fn test_unknown_field_error() {
        let err = ConfigError::unknown_field("invalid_key", "server");
        assert!(err.to_string().contains("invalid_key"));
        assert!(err.to_string().contains("server"));
    }

    #[test]
    fn test_invalid_value_error() {
        let err = ConfigError::invalid_value("http_addr", "not a valid address");
        assert!(err.to_string().contains("http_addr"));
        assert!(err.to_string().contains("not a valid address"));
    }

    #[test]
    fn test_missing_field_error() {
        let err = ConfigError::missing_field("service_name");
        assert!(err.to_string().contains("service_name"));
    }

    #[test]
    fn test_env_parse_error() {
        let err = ConfigError::env_parse_error("ARCHIMEDES__SERVER__PORT", "expected integer");
        assert!(err.to_string().contains("ARCHIMEDES__SERVER__PORT"));
        assert!(err.to_string().contains("expected integer"));
    }

    #[test]
    fn test_validation_error() {
        let err = ConfigError::validation_error("port must be between 1 and 65535");
        assert!(err.to_string().contains("port must be between 1 and 65535"));
    }
}
