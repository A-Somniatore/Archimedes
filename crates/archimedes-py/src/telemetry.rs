//! Telemetry integration for Python bindings
//!
//! This module provides OpenTelemetry-based observability for Python handlers,
//! ensuring parity with the Rust native implementation.
//!
//! ## Overview
//!
//! Telemetry in Archimedes includes:
//! 1. **Metrics**: Prometheus-format request counters, histograms, gauges
//! 2. **Tracing**: OpenTelemetry distributed tracing with W3C trace context
//! 3. **Logging**: Structured JSON logging with trace correlation
//!
//! ## Example
//!
//! ```python
//! from archimedes import PyTelemetry, PyTelemetryConfig
//!
//! # Initialize telemetry
//! config = PyTelemetryConfig(
//!     service_name="my-service",
//!     service_version="1.0.0",
//!     metrics_addr="0.0.0.0:9090",
//!     otlp_endpoint="http://localhost:4317",
//! )
//! telemetry = PyTelemetry.init(config)
//!
//! # Record metrics manually if needed
//! telemetry.record_request("getUser", 200, 0.045)
//! telemetry.increment_in_flight()
//! telemetry.decrement_in_flight()
//! ```

use std::time::Duration;

use archimedes_telemetry::metrics::{
    decrement_in_flight, increment_in_flight, init_metrics, record_request, record_request_size,
    record_response_size, render_metrics, MetricsConfig,
};
use archimedes_telemetry::{
    init_logging, init_tracing, LogConfig, TelemetryConfig, TelemetryGuard, TracingConfig,
};
use pyo3::prelude::*;

use crate::error::ArchimedesError;

/// Python-exposed telemetry configuration.
///
/// Configures metrics, tracing, and logging for the service.
#[pyclass(name = "TelemetryConfig")]
#[derive(Debug, Clone)]
pub struct PyTelemetryConfig {
    /// Service name for metrics and traces.
    #[pyo3(get, set)]
    pub service_name: String,

    /// Service version.
    #[pyo3(get, set)]
    pub service_version: String,

    /// Environment (production, staging, development).
    #[pyo3(get, set)]
    pub environment: String,

    /// Address to expose metrics on (e.g., "0.0.0.0:9090").
    #[pyo3(get, set)]
    pub metrics_addr: Option<String>,

    /// Whether metrics are enabled.
    #[pyo3(get, set)]
    pub metrics_enabled: bool,

    /// OTLP endpoint for tracing (e.g., "http://localhost:4317").
    #[pyo3(get, set)]
    pub otlp_endpoint: Option<String>,

    /// Whether tracing is enabled.
    #[pyo3(get, set)]
    pub tracing_enabled: bool,

    /// Sampling ratio for traces (0.0-1.0).
    #[pyo3(get, set)]
    pub sampling_ratio: f64,

    /// Log level (trace, debug, info, warn, error).
    #[pyo3(get, set)]
    pub log_level: String,

    /// Whether to output logs as JSON.
    #[pyo3(get, set)]
    pub log_json: bool,
}

#[pymethods]
impl PyTelemetryConfig {
    /// Create a new telemetry configuration.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name for metrics and traces
    /// * `service_version` - Service version (default: "0.0.0")
    /// * `environment` - Environment name (default: "development")
    #[new]
    #[pyo3(signature = (
        service_name,
        service_version = None,
        environment = None,
        metrics_addr = None,
        metrics_enabled = None,
        otlp_endpoint = None,
        tracing_enabled = None,
        sampling_ratio = None,
        log_level = None,
        log_json = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        service_name: String,
        service_version: Option<String>,
        environment: Option<String>,
        metrics_addr: Option<String>,
        metrics_enabled: Option<bool>,
        otlp_endpoint: Option<String>,
        tracing_enabled: Option<bool>,
        sampling_ratio: Option<f64>,
        log_level: Option<String>,
        log_json: Option<bool>,
    ) -> Self {
        Self {
            service_name,
            service_version: service_version.unwrap_or_else(|| "0.0.0".to_string()),
            environment: environment.unwrap_or_else(|| "development".to_string()),
            metrics_addr,
            metrics_enabled: metrics_enabled.unwrap_or(true),
            otlp_endpoint,
            tracing_enabled: tracing_enabled.unwrap_or(false),
            sampling_ratio: sampling_ratio.unwrap_or(1.0),
            log_level: log_level.unwrap_or_else(|| "info".to_string()),
            log_json: log_json.unwrap_or(true),
        }
    }

    /// Create a development configuration.
    ///
    /// Pre-configured for local development:
    /// - Metrics on 0.0.0.0:9090
    /// - Tracing disabled
    /// - Pretty logging
    #[staticmethod]
    pub fn development(service_name: String) -> Self {
        Self {
            service_name,
            service_version: "dev".to_string(),
            environment: "development".to_string(),
            metrics_addr: Some("0.0.0.0:9090".to_string()),
            metrics_enabled: true,
            otlp_endpoint: None,
            tracing_enabled: false,
            sampling_ratio: 1.0,
            log_level: "debug".to_string(),
            log_json: false,
        }
    }

    /// Create a production configuration.
    ///
    /// Pre-configured for production:
    /// - Metrics on 0.0.0.0:9090
    /// - Tracing enabled (requires OTLP endpoint)
    /// - JSON logging
    /// - 10% sampling ratio
    #[staticmethod]
    pub fn production(service_name: String, service_version: String) -> Self {
        Self {
            service_name,
            service_version,
            environment: "production".to_string(),
            metrics_addr: Some("0.0.0.0:9090".to_string()),
            metrics_enabled: true,
            otlp_endpoint: None, // Must be set explicitly
            tracing_enabled: true,
            sampling_ratio: 0.1,
            log_level: "info".to_string(),
            log_json: true,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TelemetryConfig(service='{}', version='{}', env='{}', metrics={}, tracing={})",
            self.service_name,
            self.service_version,
            self.environment,
            self.metrics_enabled,
            self.tracing_enabled
        )
    }
}

impl PyTelemetryConfig {
    /// Convert to native `TelemetryConfig`.
    fn to_native(&self) -> TelemetryConfig {
        TelemetryConfig::builder()
            .service_name(&self.service_name)
            .service_version(&self.service_version)
            .environment(&self.environment)
            .build()
    }

    /// Convert to native `MetricsConfig`.
    fn to_metrics_config(&self) -> MetricsConfig {
        MetricsConfig {
            enabled: self.metrics_enabled,
            addr: self
                .metrics_addr
                .clone()
                .unwrap_or_else(|| "0.0.0.0:9090".to_string()),
            service_name: self.service_name.clone(),
            ..MetricsConfig::default()
        }
    }

    /// Convert to native `TracingConfig`.
    fn to_tracing_config(&self) -> TracingConfig {
        TracingConfig {
            enabled: self.tracing_enabled,
            service_name: self.service_name.clone(),
            service_version: self.service_version.clone(),
            otlp_endpoint: self
                .otlp_endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:4317".to_string()),
            environment: self.environment.clone(),
            sample_ratio: self.sampling_ratio,
        }
    }

    /// Convert to native `LogConfig`.
    fn to_log_config(&self) -> LogConfig {
        LogConfig {
            level: self.log_level.clone(),
            json_format: self.log_json,
            service_name: self.service_name.clone(),
            ..LogConfig::default()
        }
    }
}

/// Python-exposed telemetry manager.
///
/// Manages the lifecycle of telemetry (metrics, tracing, logging).
/// Should be initialized once at application startup.
#[pyclass(name = "Telemetry")]
pub struct PyTelemetry {
    /// Configuration used to initialize telemetry.
    config: PyTelemetryConfig,
    /// Guard to keep telemetry providers alive.
    /// We use Option because TelemetryGuard is not Send/Sync for PyO3.
    #[allow(dead_code)]
    guard: Option<TelemetryGuard>,
    /// Whether telemetry has been initialized.
    initialized: bool,
}

#[pymethods]
impl PyTelemetry {
    /// Initialize telemetry with the given configuration.
    ///
    /// This should be called once at application startup.
    ///
    /// # Arguments
    ///
    /// * `config` - Telemetry configuration
    ///
    /// # Returns
    ///
    /// A Telemetry instance that keeps providers alive.
    #[staticmethod]
    pub fn init(config: PyTelemetryConfig) -> PyResult<Self> {
        // Initialize logging
        let log_config = config.to_log_config();
        init_logging(&log_config)
            .map_err(|e| ArchimedesError::new_err(format!("Failed to init logging: {e}")))?;

        // Initialize metrics if enabled
        if config.metrics_enabled {
            let metrics_config = config.to_metrics_config();
            init_metrics(&metrics_config)
                .map_err(|e| ArchimedesError::new_err(format!("Failed to init metrics: {e}")))?;
        }

        // Initialize tracing if enabled and endpoint is provided
        let guard = if config.tracing_enabled {
            let tracing_config = config.to_tracing_config();
            let provider = init_tracing(&tracing_config)
                .map_err(|e| ArchimedesError::new_err(format!("Failed to init tracing: {e}")))?;
            provider.map(|p| TelemetryGuard::new(Some(p)))
        } else {
            None
        };

        Ok(Self {
            config,
            guard,
            initialized: true,
        })
    }

    /// Record a completed request.
    ///
    /// Updates metrics:
    /// - `archimedes_requests_total` (incremented)
    /// - `archimedes_request_duration_seconds` (histogram)
    ///
    /// # Arguments
    ///
    /// * `operation` - Operation ID (e.g., "getUser")
    /// * `status_code` - HTTP status code
    /// * `duration_seconds` - Request duration in seconds
    pub fn record_request(&self, operation: &str, status_code: u16, duration_seconds: f64) {
        if self.initialized && self.config.metrics_enabled {
            record_request(
                operation,
                status_code,
                Duration::from_secs_f64(duration_seconds),
            );
        }
    }

    /// Record request body size.
    ///
    /// # Arguments
    ///
    /// * `operation` - Operation ID
    /// * `size_bytes` - Request body size in bytes
    pub fn record_request_size(&self, operation: &str, size_bytes: u64) {
        if self.initialized && self.config.metrics_enabled {
            record_request_size(operation, size_bytes);
        }
    }

    /// Record response body size.
    ///
    /// # Arguments
    ///
    /// * `operation` - Operation ID
    /// * `size_bytes` - Response body size in bytes
    pub fn record_response_size(&self, operation: &str, size_bytes: u64) {
        if self.initialized && self.config.metrics_enabled {
            record_response_size(operation, size_bytes);
        }
    }

    /// Increment the in-flight requests gauge.
    pub fn increment_in_flight(&self) {
        if self.initialized && self.config.metrics_enabled {
            increment_in_flight();
        }
    }

    /// Decrement the in-flight requests gauge.
    pub fn decrement_in_flight(&self) {
        if self.initialized && self.config.metrics_enabled {
            decrement_in_flight();
        }
    }

    /// Render all metrics in Prometheus text format.
    ///
    /// Returns the metrics string or None if metrics are not initialized.
    pub fn render_metrics(&self) -> Option<String> {
        if self.initialized && self.config.metrics_enabled {
            render_metrics()
        } else {
            None
        }
    }

    /// Get the current configuration.
    #[getter]
    pub fn config(&self) -> PyTelemetryConfig {
        self.config.clone()
    }

    /// Check if telemetry is initialized.
    #[getter]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn __repr__(&self) -> String {
        format!(
            "Telemetry(service='{}', initialized={}, metrics={}, tracing={})",
            self.config.service_name,
            self.initialized,
            self.config.metrics_enabled,
            self.config.tracing_enabled
        )
    }
}

/// Record a completed request (standalone function).
///
/// Convenience function that can be called without a Telemetry instance
/// if metrics have been initialized.
///
/// # Arguments
///
/// * `operation` - Operation ID
/// * `status_code` - HTTP status code
/// * `duration_seconds` - Request duration in seconds
#[pyfunction]
pub fn py_record_request(operation: &str, status_code: u16, duration_seconds: f64) {
    record_request(
        operation,
        status_code,
        Duration::from_secs_f64(duration_seconds),
    );
}

/// Render all metrics in Prometheus text format.
///
/// Standalone function that returns metrics if initialized.
#[pyfunction]
pub fn py_render_metrics() -> Option<String> {
    render_metrics()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_new() {
        let config = PyTelemetryConfig::new(
            "test-service".to_string(),
            Some("1.0.0".to_string()),
            Some("production".to_string()),
            Some("0.0.0.0:9090".to_string()),
            Some(true),
            Some("http://localhost:4317".to_string()),
            Some(true),
            Some(0.5),
            Some("debug".to_string()),
            Some(true),
        );

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "1.0.0");
        assert_eq!(config.environment, "production");
        assert_eq!(config.metrics_addr, Some("0.0.0.0:9090".to_string()));
        assert!(config.metrics_enabled);
        assert_eq!(
            config.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
        assert!(config.tracing_enabled);
        assert!((config.sampling_ratio - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.log_level, "debug");
        assert!(config.log_json);
    }

    #[test]
    fn test_telemetry_config_defaults() {
        let config = PyTelemetryConfig::new(
            "test-service".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "0.0.0");
        assert_eq!(config.environment, "development");
        assert!(config.metrics_addr.is_none());
        assert!(config.metrics_enabled);
        assert!(config.otlp_endpoint.is_none());
        assert!(!config.tracing_enabled);
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
        assert_eq!(config.log_level, "info");
        assert!(config.log_json);
    }

    #[test]
    fn test_telemetry_config_development() {
        let config = PyTelemetryConfig::development("dev-service".to_string());

        assert_eq!(config.service_name, "dev-service");
        assert_eq!(config.service_version, "dev");
        assert_eq!(config.environment, "development");
        assert_eq!(config.metrics_addr, Some("0.0.0.0:9090".to_string()));
        assert!(config.metrics_enabled);
        assert!(!config.tracing_enabled);
        assert!(!config.log_json);
    }

    #[test]
    fn test_telemetry_config_production() {
        let config = PyTelemetryConfig::production("prod-service".to_string(), "2.0.0".to_string());

        assert_eq!(config.service_name, "prod-service");
        assert_eq!(config.service_version, "2.0.0");
        assert_eq!(config.environment, "production");
        assert!(config.metrics_enabled);
        assert!(config.tracing_enabled);
        assert!(config.log_json);
        assert!((config.sampling_ratio - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_telemetry_config_repr() {
        let config = PyTelemetryConfig::development("test".to_string());
        let repr = config.__repr__();
        assert!(repr.contains("test"));
        assert!(repr.contains("development"));
    }
}
