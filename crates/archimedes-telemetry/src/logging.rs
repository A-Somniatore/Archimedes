//! Structured JSON logging for Archimedes.
//!
//! This module provides structured JSON logging with trace correlation,
//! integrating with the tracing-subscriber ecosystem.
//!
//! # Features
//!
//! - JSON-formatted log output
//! - Trace ID correlation in logs
//! - Configurable log levels
//! - Span context in structured fields
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_telemetry::logging::{LogConfig, init_logging};
//!
//! let config = LogConfig::default();
//! init_logging(&config)?;
//!
//! tracing::info!(operation = "getUser", user_id = 123, "Processing request");
//! ```

use crate::error::TelemetryError;
use crate::TelemetryResult;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Logging configuration.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Whether logging is enabled.
    pub enabled: bool,

    /// Log level (e.g., "info", "debug", "warn").
    pub level: String,

    /// Whether to output JSON format.
    pub json_format: bool,

    /// Whether to include span events (enter, exit, close).
    pub span_events: bool,

    /// Whether to include file/line info.
    pub file_line_info: bool,

    /// Whether to include thread IDs.
    pub thread_ids: bool,

    /// Whether to include target (module path).
    pub include_target: bool,

    /// Service name for log fields.
    pub service_name: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
            json_format: true, // JSON by default for production
            span_events: false,
            file_line_info: false,
            thread_ids: false,
            include_target: true,
            service_name: "archimedes".to_string(),
        }
    }
}

impl LogConfig {
    /// Creates a development configuration with human-readable output.
    #[must_use]
    pub fn development() -> Self {
        Self {
            enabled: true,
            level: "debug".to_string(),
            json_format: false,
            span_events: true,
            file_line_info: true,
            thread_ids: false,
            include_target: true,
            service_name: "archimedes".to_string(),
        }
    }

    /// Creates a production configuration with JSON output.
    #[must_use]
    pub fn production() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
            json_format: true,
            span_events: false,
            file_line_info: false,
            thread_ids: false,
            include_target: true,
            service_name: "archimedes".to_string(),
        }
    }
}

/// Initializes the logging subsystem.
///
/// # Arguments
///
/// * `config` - Logging configuration
///
/// # Errors
///
/// Returns `TelemetryError::LoggingInit` if initialization fails.
pub fn init_logging(config: &LogConfig) -> TelemetryResult<()> {
    if !config.enabled {
        return Ok(());
    }

    // Build env filter
    let filter = EnvFilter::try_new(&config.level)
        .map_err(|e| TelemetryError::LoggingInit(format!("Invalid log level: {e}")))?;

    // Determine span events to capture
    let span_events = if config.span_events {
        FmtSpan::NEW | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    if config.json_format {
        // JSON format for production
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_span_events(span_events)
            .with_file(config.file_line_info)
            .with_line_number(config.file_line_info)
            .with_thread_ids(config.thread_ids)
            .with_target(config.include_target)
            .with_filter(filter);

        tracing_subscriber::registry()
            .with(fmt_layer)
            .try_init()
            .map_err(|e| TelemetryError::LoggingInit(e.to_string()))?;
    } else {
        // Pretty format for development
        let fmt_layer = tracing_subscriber::fmt::layer()
            .pretty()
            .with_span_events(span_events)
            .with_file(config.file_line_info)
            .with_line_number(config.file_line_info)
            .with_thread_ids(config.thread_ids)
            .with_target(config.include_target)
            .with_filter(filter);

        tracing_subscriber::registry()
            .with(fmt_layer)
            .try_init()
            .map_err(|e| TelemetryError::LoggingInit(e.to_string()))?;
    }

    Ok(())
}

/// Creates an env filter from a string.
///
/// # Arguments
///
/// * `filter` - Filter string (e.g., "info", "archimedes=debug,tower=warn")
///
/// # Errors
///
/// Returns error if the filter string is invalid.
pub fn create_env_filter(filter: &str) -> TelemetryResult<EnvFilter> {
    EnvFilter::try_new(filter).map_err(|e| TelemetryError::LoggingInit(e.to_string()))
}

/// Standard log fields for Archimedes.
///
/// Use these field names for consistency across logs.
pub mod fields {
    /// Request ID field name.
    pub const REQUEST_ID: &str = "request_id";

    /// Trace ID field name.
    pub const TRACE_ID: &str = "trace_id";

    /// Span ID field name.
    pub const SPAN_ID: &str = "span_id";

    /// Operation ID field name.
    pub const OPERATION_ID: &str = "operation_id";

    /// HTTP method field name.
    pub const HTTP_METHOD: &str = "http.method";

    /// HTTP path field name.
    pub const HTTP_PATH: &str = "http.path";

    /// HTTP status code field name.
    pub const HTTP_STATUS: &str = "http.status_code";

    /// Duration field name (in milliseconds).
    pub const DURATION_MS: &str = "duration_ms";

    /// Error field name.
    pub const ERROR: &str = "error";

    /// User ID field name.
    pub const USER_ID: &str = "user_id";

    /// Service name field name.
    pub const SERVICE_NAME: &str = "service.name";
}

/// Logs a request start event.
#[macro_export]
macro_rules! log_request_start {
    ($request_id:expr, $method:expr, $path:expr, $operation:expr) => {
        tracing::info!(
            request_id = %$request_id,
            http.method = %$method,
            http.path = %$path,
            operation_id = %$operation,
            "Request started"
        );
    };
}

/// Logs a request completion event.
#[macro_export]
macro_rules! log_request_complete {
    ($request_id:expr, $status:expr, $duration_ms:expr) => {
        tracing::info!(
            request_id = %$request_id,
            http.status_code = $status,
            duration_ms = $duration_ms,
            "Request completed"
        );
    };
}

/// Logs a request error event.
#[macro_export]
macro_rules! log_request_error {
    ($request_id:expr, $error:expr) => {
        tracing::error!(
            request_id = %$request_id,
            error = %$error,
            "Request failed"
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LogConfig::default();
        assert!(config.enabled);
        assert!(config.json_format);
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_development_config() {
        let config = LogConfig::development();
        assert!(!config.json_format);
        assert!(config.span_events);
        assert!(config.file_line_info);
        assert_eq!(config.level, "debug");
    }

    #[test]
    fn test_production_config() {
        let config = LogConfig::production();
        assert!(config.json_format);
        assert!(!config.span_events);
        assert!(!config.file_line_info);
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_field_names() {
        assert_eq!(fields::REQUEST_ID, "request_id");
        assert_eq!(fields::TRACE_ID, "trace_id");
        assert_eq!(fields::OPERATION_ID, "operation_id");
    }

    #[test]
    fn test_create_env_filter_valid() {
        let filter = create_env_filter("info");
        assert!(filter.is_ok());
    }

    #[test]
    fn test_disabled_logging() {
        let config = LogConfig {
            enabled: false,
            ..Default::default()
        };

        // Should return Ok even when disabled
        let result = init_logging(&config);
        assert!(result.is_ok());
    }
}
