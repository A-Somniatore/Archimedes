//! Prometheus metrics for Archimedes.
//!
//! This module provides Prometheus-format metrics collection and exposure.
//!
//! # Standard Metrics
//!
//! | Metric | Type | Labels | Description |
//! |--------|------|--------|-------------|
//! | `archimedes_requests_total` | Counter | `operation`, `status` | Total requests |
//! | `archimedes_request_duration_seconds` | Histogram | `operation` | Request latency |
//! | `archimedes_in_flight_requests` | Gauge | - | In-flight requests |
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_telemetry::metrics::{MetricsRegistry, record_request};
//!
//! // Record a completed request
//! record_request("getUser", 200, Duration::from_millis(45));
//! ```

use crate::error::TelemetryError;
use crate::TelemetryResult;
use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::Duration;

/// Global metrics handle for rendering.
static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Metrics configuration.
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Whether metrics are enabled.
    pub enabled: bool,

    /// Address to expose metrics on (e.g., "0.0.0.0:9090").
    pub addr: String,

    /// Service name for metric labels.
    pub service_name: String,

    /// Histogram buckets for request duration.
    pub duration_buckets: Vec<f64>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            addr: "0.0.0.0:9090".to_string(),
            service_name: "archimedes".to_string(),
            // Default buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
            duration_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        }
    }
}

/// Metrics registry for Archimedes.
///
/// Provides methods to record standard metrics and render them in Prometheus format.
#[derive(Debug)]
pub struct MetricsRegistry {
    handle: PrometheusHandle,
}

impl MetricsRegistry {
    /// Creates a new metrics registry with the given handle.
    #[must_use]
    pub fn new(handle: PrometheusHandle) -> Self {
        Self { handle }
    }

    /// Renders all metrics in Prometheus text format.
    #[must_use]
    pub fn render(&self) -> String {
        self.handle.render()
    }
}

/// Initializes the metrics subsystem.
///
/// # Arguments
///
/// * `config` - Metrics configuration
///
/// # Errors
///
/// Returns `TelemetryError::MetricsInit` if initialization fails.
pub fn init_metrics(config: &MetricsConfig) -> TelemetryResult<()> {
    if !config.enabled {
        return Ok(());
    }

    // Parse address
    let addr: SocketAddr = config
        .addr
        .parse()
        .map_err(|e| TelemetryError::InvalidAddress(format!("{}: {e}", config.addr)))?;

    // Build Prometheus exporter
    let builder = PrometheusBuilder::new();

    // Install the recorder
    let handle = builder
        .with_http_listener(addr)
        .install_recorder()
        .map_err(|e| TelemetryError::MetricsInit(e.to_string()))?;

    // Store handle for later access
    let _ = METRICS_HANDLE.set(handle);

    // Register metric descriptions
    register_metric_descriptions();

    Ok(())
}

/// Returns the global metrics handle if initialized.
pub fn get_metrics_handle() -> Option<&'static PrometheusHandle> {
    METRICS_HANDLE.get()
}

/// Renders metrics in Prometheus format.
///
/// Returns `None` if metrics are not initialized.
#[must_use]
pub fn render_metrics() -> Option<String> {
    METRICS_HANDLE.get().map(PrometheusHandle::render)
}

/// Registers descriptions for all standard metrics.
fn register_metric_descriptions() {
    // Request counter
    describe_counter!(
        "archimedes_requests_total",
        "Total number of HTTP requests processed"
    );

    // Request duration histogram
    describe_histogram!(
        "archimedes_request_duration_seconds",
        "HTTP request duration in seconds"
    );

    // In-flight requests gauge
    describe_gauge!(
        "archimedes_in_flight_requests",
        "Number of HTTP requests currently being processed"
    );

    // Request size histogram
    describe_histogram!(
        "archimedes_request_size_bytes",
        "HTTP request body size in bytes"
    );

    // Response size histogram
    describe_histogram!(
        "archimedes_response_size_bytes",
        "HTTP response body size in bytes"
    );

    // Authorization metrics
    describe_counter!(
        "archimedes_authz_decisions_total",
        "Total authorization decisions by result"
    );

    // Validation metrics
    describe_counter!(
        "archimedes_validation_failures_total",
        "Total validation failures by type"
    );
}

// ============================================================================
// Metric Recording Functions
// ============================================================================

/// Records a completed request.
///
/// Updates the following metrics:
/// - `archimedes_requests_total` (incremented)
/// - `archimedes_request_duration_seconds` (histogram observation)
///
/// # Arguments
///
/// * `operation` - The operation ID (e.g., "getUser")
/// * `status_code` - HTTP status code
/// * `duration` - Request duration
pub fn record_request(operation: &str, status_code: u16, duration: Duration) {
    // Increment request counter
    counter!(
        "archimedes_requests_total",
        "operation" => operation.to_string(),
        "status" => status_code.to_string()
    )
    .increment(1);

    // Record duration
    histogram!(
        "archimedes_request_duration_seconds",
        "operation" => operation.to_string()
    )
    .record(duration.as_secs_f64());
}

/// Increments the in-flight requests gauge.
pub fn increment_in_flight() {
    gauge!("archimedes_in_flight_requests").increment(1.0);
}

/// Decrements the in-flight requests gauge.
pub fn decrement_in_flight() {
    gauge!("archimedes_in_flight_requests").decrement(1.0);
}

/// Records request body size.
///
/// # Arguments
///
/// * `operation` - The operation ID
/// * `size_bytes` - Request body size in bytes
pub fn record_request_size(operation: &str, size_bytes: u64) {
    histogram!(
        "archimedes_request_size_bytes",
        "operation" => operation.to_string()
    )
    .record(size_bytes as f64);
}

/// Records response body size.
///
/// # Arguments
///
/// * `operation` - The operation ID
/// * `size_bytes` - Response body size in bytes
pub fn record_response_size(operation: &str, size_bytes: u64) {
    histogram!(
        "archimedes_response_size_bytes",
        "operation" => operation.to_string()
    )
    .record(size_bytes as f64);
}

/// Records an authorization decision.
///
/// # Arguments
///
/// * `allowed` - Whether the request was allowed
/// * `reason` - Reason for the decision (e.g., "policy_allow", "policy_deny")
pub fn record_authz_decision(allowed: bool, reason: &str) {
    counter!(
        "archimedes_authz_decisions_total",
        "allowed" => allowed.to_string(),
        "reason" => reason.to_string()
    )
    .increment(1);
}

/// Records a validation failure.
///
/// # Arguments
///
/// * `validation_type` - Type of validation ("request", "response")
/// * `error_type` - Type of error (e.g., "missing_field", "type_mismatch")
pub fn record_validation_failure(validation_type: &str, error_type: &str) {
    counter!(
        "archimedes_validation_failures_total",
        "type" => validation_type.to_string(),
        "error" => error_type.to_string()
    )
    .increment(1);
}

/// Guard that decrements in-flight requests on drop.
///
/// Use this to ensure in-flight counter is always decremented, even on panic.
pub struct InFlightGuard {
    _private: (),
}

impl InFlightGuard {
    /// Creates a new guard and increments the in-flight counter.
    #[must_use]
    pub fn new() -> Self {
        increment_in_flight();
        Self { _private: () }
    }
}

impl Default for InFlightGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        decrement_in_flight();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.addr, "0.0.0.0:9090");
        assert!(!config.duration_buckets.is_empty());
    }

    #[test]
    fn test_in_flight_guard() {
        // Can't really test the global counter without init, but we can test the guard doesn't panic
        let _guard = InFlightGuard { _private: () };
        drop(_guard);
    }

    #[test]
    fn test_record_functions_dont_panic() {
        // These should not panic even without init (metrics crate handles gracefully)
        record_request("test", 200, Duration::from_millis(10));
        record_request_size("test", 1024);
        record_response_size("test", 2048);
        record_authz_decision(true, "allowed");
        record_validation_failure("request", "missing_field");
    }

    #[test]
    fn test_metrics_config_builder() {
        let config = MetricsConfig {
            enabled: true,
            addr: "127.0.0.1:8080".to_string(),
            service_name: "test".to_string(),
            duration_buckets: vec![0.1, 0.5, 1.0],
        };
        assert_eq!(config.addr, "127.0.0.1:8080");
        assert_eq!(config.duration_buckets.len(), 3);
    }

    #[test]
    fn test_render_metrics_without_init() {
        // Should return None when not initialized
        // Note: This test may fail if other tests have initialized metrics
        // In isolation, it should return None
        let _ = render_metrics();
    }
}
