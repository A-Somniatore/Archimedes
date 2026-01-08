//! OpenTelemetry-based observability for Archimedes.
//!
//! This crate provides comprehensive observability capabilities for Archimedes services:
//!
//! - **Metrics**: Prometheus-format metrics via the `metrics` crate
//! - **Tracing**: Distributed tracing via OpenTelemetry with OTLP export
//! - **Logging**: Structured JSON logging with trace correlation
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Archimedes Service                       │
//! │                                                              │
//! │  ┌──────────────────────────────────────────────────────┐  │
//! │  │                  archimedes-telemetry                  │  │
//! │  │                                                        │  │
//! │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │  │
//! │  │  │   Metrics   │  │   Tracing   │  │   Logging   │   │  │
//! │  │  │ (Prometheus)│  │(OpenTelemetry)│ │ (JSON/OTLP)│   │  │
//! │  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘   │  │
//! │  │         │                │                │          │  │
//! │  └─────────┼────────────────┼────────────────┼──────────┘  │
//! │            │                │                │              │
//! └────────────┼────────────────┼────────────────┼──────────────┘
//!              │                │                │
//!              ▼                ▼                ▼
//!        ┌──────────┐    ┌──────────┐    ┌──────────┐
//!        │Prometheus│    │  OTLP    │    │ stdout/  │
//!        │  /metrics│    │ Collector│    │  stderr  │
//!        └──────────┘    └──────────┘    └──────────┘
//! ```
//!
//! # Standard Metrics
//!
//! Archimedes emits the following standard metrics:
//!
//! | Metric | Type | Labels | Description |
//! |--------|------|--------|-------------|
//! | `archimedes_requests_total` | Counter | `operation`, `status` | Total request count |
//! | `archimedes_request_duration_seconds` | Histogram | `operation` | Request latency |
//! | `archimedes_in_flight_requests` | Gauge | - | Currently processing requests |
//! | `archimedes_request_size_bytes` | Histogram | `operation` | Request body size |
//! | `archimedes_response_size_bytes` | Histogram | `operation` | Response body size |
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_telemetry::{TelemetryConfig, init_telemetry};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialize telemetry
//!     let config = TelemetryConfig::builder()
//!         .service_name("my-service")
//!         .service_version("1.0.0")
//!         .environment("production")
//!         .metrics_addr("0.0.0.0:9090")
//!         .otlp_endpoint("http://localhost:4317")
//!         .build();
//!
//!     let _guard = init_telemetry(config).expect("Failed to init telemetry");
//!
//!     // Telemetry is now active...
//! }
//! ```
//!
//! # Metrics Endpoint
//!
//! The `/metrics` endpoint exposes Prometheus-format metrics:
//!
//! ```text
//! # HELP archimedes_requests_total Total number of requests
//! # TYPE archimedes_requests_total counter
//! archimedes_requests_total{operation="getUser",status="200"} 1234
//! archimedes_requests_total{operation="getUser",status="404"} 56
//!
//! # HELP archimedes_request_duration_seconds Request duration histogram
//! # TYPE archimedes_request_duration_seconds histogram
//! archimedes_request_duration_seconds_bucket{operation="getUser",le="0.01"} 1000
//! archimedes_request_duration_seconds_bucket{operation="getUser",le="0.1"} 1200
//! ```

#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod logging;
pub mod metrics;
pub mod tracing;

pub use config::{TelemetryConfig, TelemetryConfigBuilder};
pub use error::TelemetryError;
pub use logging::{init_logging, LogConfig};
pub use metrics::{init_metrics, MetricsConfig, MetricsRegistry};
pub use tracing::{init_tracing, TracingConfig};

/// Result type for telemetry operations.
pub type TelemetryResult<T> = Result<T, TelemetryError>;

/// Guard that shuts down telemetry providers on drop.
///
/// This guard should be kept alive for the lifetime of the application.
/// When dropped, it will flush any pending telemetry data and shut down
/// the providers gracefully.
pub struct TelemetryGuard {
    /// Tracing provider shutdown handle
    #[allow(dead_code)]
    tracer_provider: Option<opentelemetry_sdk::trace::TracerProvider>,
}

impl TelemetryGuard {
    /// Creates a new telemetry guard.
    #[must_use]
    pub fn new(tracer_provider: Option<opentelemetry_sdk::trace::TracerProvider>) -> Self {
        Self { tracer_provider }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        // Shutdown tracer provider if present
        if let Some(provider) = self.tracer_provider.take() {
            // Force flush and shutdown
            for result in provider.force_flush() {
                if let Err(e) = result {
                    eprintln!("Error flushing tracer provider: {e}");
                }
            }
            if let Err(e) = provider.shutdown() {
                eprintln!("Error shutting down tracer provider: {e}");
            }
        }
    }
}

/// Initializes all telemetry subsystems.
///
/// This is the main entry point for setting up observability. It initializes:
/// - Prometheus metrics registry and exporter
/// - OpenTelemetry tracing with OTLP export
/// - Structured JSON logging with trace correlation
///
/// # Arguments
///
/// * `config` - Telemetry configuration
///
/// # Returns
///
/// A guard that must be kept alive for telemetry to remain active.
///
/// # Errors
///
/// Returns `TelemetryError` if any subsystem fails to initialize.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes_telemetry::{TelemetryConfig, init_telemetry};
///
/// let config = TelemetryConfig::builder()
///     .service_name("my-service")
///     .build();
///
/// let _guard = init_telemetry(config)?;
/// ```
pub fn init_telemetry(config: TelemetryConfig) -> TelemetryResult<TelemetryGuard> {
    // Initialize logging first
    init_logging(&config.logging)?;

    // Initialize metrics
    init_metrics(&config.metrics)?;

    // Initialize tracing
    let tracer_provider = init_tracing(&config.tracing)?;

    Ok(TelemetryGuard::new(tracer_provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_guard_creation() {
        let guard = TelemetryGuard::new(None);
        drop(guard); // Should not panic
    }

    #[test]
    fn test_telemetry_config_builder() {
        let config = TelemetryConfig::builder()
            .service_name("test-service")
            .service_version("1.0.0")
            .environment("test")
            .build();

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "1.0.0");
        assert_eq!(config.environment, "test");
    }
}
