//! Telemetry error types.

use thiserror::Error;

/// Errors that can occur during telemetry operations.
#[derive(Debug, Error)]
pub enum TelemetryError {
    /// Failed to initialize metrics.
    #[error("Failed to initialize metrics: {0}")]
    MetricsInit(String),

    /// Failed to initialize tracing.
    #[error("Failed to initialize tracing: {0}")]
    TracingInit(String),

    /// Failed to initialize logging.
    #[error("Failed to initialize logging: {0}")]
    LoggingInit(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Failed to parse address.
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TelemetryError::MetricsInit("failed".to_string());
        assert_eq!(err.to_string(), "Failed to initialize metrics: failed");
    }

    #[test]
    fn test_error_variants() {
        let _ = TelemetryError::TracingInit("trace fail".to_string());
        let _ = TelemetryError::LoggingInit("log fail".to_string());
        let _ = TelemetryError::InvalidConfig("bad config".to_string());
        let _ = TelemetryError::InvalidAddress("bad addr".to_string());
    }
}
