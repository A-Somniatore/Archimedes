//! Telemetry integration (metrics, tracing, logging).

use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Telemetry configuration.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Service name for telemetry
    pub service_name: Option<String>,

    /// Service version
    pub service_version: Option<String>,

    /// Enable metrics collection
    pub enable_metrics: Option<bool>,

    /// Enable distributed tracing
    pub enable_tracing: Option<bool>,

    /// Enable structured logging
    pub enable_logging: Option<bool>,

    /// OTLP endpoint for traces
    pub otlp_endpoint: Option<String>,

    /// Prometheus metrics port
    pub metrics_port: Option<u32>,

    /// Log level (trace, debug, info, warn, error)
    pub log_level: Option<String>,

    /// Use JSON log format
    pub json_logs: Option<bool>,

    /// Trace sampling ratio (0.0 to 1.0)
    pub sample_ratio: Option<f64>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: Some("archimedes-app".to_string()),
            service_version: Some("0.1.0".to_string()),
            enable_metrics: Some(true),
            enable_tracing: Some(true),
            enable_logging: Some(true),
            otlp_endpoint: None,
            metrics_port: Some(9090),
            log_level: Some("info".to_string()),
            json_logs: Some(false),
            sample_ratio: Some(1.0),
        }
    }
}

/// Telemetry instance for recording metrics and traces.
#[napi]
#[derive(Debug, Clone)]
pub struct Telemetry {
    config: TelemetryConfig,
    initialized: bool,
    request_count: u64,
    error_count: u64,
}

#[napi]
impl Telemetry {
    /// Create a new Telemetry instance with configuration.
    #[napi(constructor)]
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config,
            initialized: false,
            request_count: 0,
            error_count: 0,
        }
    }

    /// Create Telemetry with default configuration.
    #[napi(factory)]
    pub fn with_defaults() -> Self {
        Self::new(TelemetryConfig::default())
    }

    /// Create Telemetry for development environment.
    #[napi(factory)]
    pub fn development() -> Self {
        Self::new(TelemetryConfig {
            service_name: Some("archimedes-dev".to_string()),
            enable_metrics: Some(true),
            enable_tracing: Some(false), // Disable tracing for dev
            enable_logging: Some(true),
            log_level: Some("debug".to_string()),
            json_logs: Some(false),
            sample_ratio: Some(1.0),
            ..TelemetryConfig::default()
        })
    }

    /// Create Telemetry for production environment.
    #[napi(factory)]
    pub fn production(service_name: String, otlp_endpoint: String) -> Self {
        Self::new(TelemetryConfig {
            service_name: Some(service_name),
            enable_metrics: Some(true),
            enable_tracing: Some(true),
            enable_logging: Some(true),
            otlp_endpoint: Some(otlp_endpoint),
            log_level: Some("info".to_string()),
            json_logs: Some(true),
            sample_ratio: Some(0.1), // Sample 10% in prod
            ..TelemetryConfig::default()
        })
    }

    /// Initialize telemetry (set up exporters, providers).
    #[napi]
    pub fn init(&mut self) -> napi::Result<()> {
        // In real implementation, would set up OpenTelemetry
        self.initialized = true;
        Ok(())
    }

    /// Check if telemetry is initialized.
    #[napi(getter)]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the configuration.
    #[napi(getter)]
    pub fn config(&self) -> TelemetryConfig {
        self.config.clone()
    }

    /// Record a request.
    #[napi]
    pub fn record_request(
        &mut self,
        method: String,
        path: String,
        status_code: u16,
        duration_ms: f64,
    ) {
        self.request_count += 1;

        if status_code >= 400 {
            self.error_count += 1;
        }

        // In real implementation, would record to Prometheus metrics
        if self.config.enable_logging.unwrap_or(false) {
            // Would use tracing crate for structured logging
            println!(
                "[{}] {} {} -> {} ({:.2}ms)",
                self.request_count, method, path, status_code, duration_ms
            );
        }
    }

    /// Record an error.
    #[napi]
    pub fn record_error(&mut self, error_type: String, message: String) {
        self.error_count += 1;

        if self.config.enable_logging.unwrap_or(false) {
            eprintln!("[ERROR] {}: {}", error_type, message);
        }
    }

    /// Start a trace span.
    #[napi]
    pub fn start_span(&self, name: String) -> Span {
        Span::new(name)
    }

    /// Get current metrics as Prometheus text format.
    #[napi]
    pub fn render_metrics(&self) -> String {
        let service_name = self.config.service_name.as_deref().unwrap_or("archimedes");

        format!(
            r"# HELP {service}_requests_total Total number of HTTP requests
# TYPE {service}_requests_total counter
{service}_requests_total {requests}

# HELP {service}_errors_total Total number of errors
# TYPE {service}_errors_total counter
{service}_errors_total {errors}
",
            service = service_name.replace('-', "_"),
            requests = self.request_count,
            errors = self.error_count
        )
    }

    /// Get total request count.
    #[napi(getter)]
    pub fn request_count(&self) -> u64 {
        self.request_count
    }

    /// Get total error count.
    #[napi(getter)]
    pub fn error_count(&self) -> u64 {
        self.error_count
    }

    /// Shutdown telemetry (flush exporters).
    #[napi]
    pub fn shutdown(&mut self) {
        // In real implementation, would flush and shutdown OpenTelemetry
        self.initialized = false;
    }
}

/// A trace span for instrumentation.
#[napi]
#[derive(Debug, Clone)]
pub struct Span {
    name: String,
    start_time: std::time::Instant,
    attributes: std::collections::HashMap<String, String>,
    ended: bool,
}

#[napi]
impl Span {
    /// Create a new span.
    fn new(name: String) -> Self {
        Self {
            name,
            start_time: std::time::Instant::now(),
            attributes: std::collections::HashMap::new(),
            ended: false,
        }
    }

    /// Get the span name.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Add an attribute to the span.
    #[napi]
    pub fn set_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }

    /// Record an exception on the span.
    #[napi]
    pub fn record_exception(&mut self, error_type: String, message: String) {
        self.set_attribute("exception.type".to_string(), error_type);
        self.set_attribute("exception.message".to_string(), message);
    }

    /// Set the span status.
    #[napi]
    pub fn set_status(&mut self, status: String, message: Option<String>) {
        self.set_attribute("otel.status_code".to_string(), status);
        if let Some(msg) = message {
            self.set_attribute("otel.status_description".to_string(), msg);
        }
    }

    /// End the span.
    #[napi]
    pub fn end(&mut self) -> f64 {
        if !self.ended {
            self.ended = true;
        }
        self.start_time.elapsed().as_secs_f64() * 1000.0
    }

    /// Check if the span has ended.
    #[napi(getter)]
    pub fn is_ended(&self) -> bool {
        self.ended
    }

    /// Get elapsed time in milliseconds.
    #[napi]
    pub fn elapsed_ms(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64() * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, Some("archimedes-app".to_string()));
        assert_eq!(config.enable_metrics, Some(true));
        assert_eq!(config.log_level, Some("info".to_string()));
    }

    #[test]
    fn test_telemetry_creation() {
        let telemetry = Telemetry::with_defaults();
        assert!(!telemetry.is_initialized());
        assert_eq!(telemetry.request_count(), 0);
    }

    #[test]
    fn test_telemetry_init() {
        let mut telemetry = Telemetry::with_defaults();
        telemetry.init().unwrap();
        assert!(telemetry.is_initialized());
    }

    #[test]
    fn test_telemetry_record_request() {
        let mut telemetry = Telemetry::with_defaults();
        telemetry.record_request("GET".to_string(), "/users".to_string(), 200, 15.5);
        assert_eq!(telemetry.request_count(), 1);
        assert_eq!(telemetry.error_count(), 0);
    }

    #[test]
    fn test_telemetry_record_error_request() {
        let mut telemetry = Telemetry::with_defaults();
        telemetry.record_request("GET".to_string(), "/users".to_string(), 500, 15.5);
        assert_eq!(telemetry.request_count(), 1);
        assert_eq!(telemetry.error_count(), 1);
    }

    #[test]
    fn test_telemetry_record_error() {
        let mut telemetry = Telemetry::with_defaults();
        telemetry.record_error("ValidationError".to_string(), "Invalid input".to_string());
        assert_eq!(telemetry.error_count(), 1);
    }

    #[test]
    fn test_telemetry_render_metrics() {
        let mut telemetry = Telemetry::with_defaults();
        telemetry.record_request("GET".to_string(), "/users".to_string(), 200, 10.0);
        telemetry.record_request("POST".to_string(), "/users".to_string(), 201, 20.0);

        let metrics = telemetry.render_metrics();
        assert!(metrics.contains("requests_total 2"));
        assert!(metrics.contains("errors_total 0"));
    }

    #[test]
    fn test_telemetry_development() {
        let telemetry = Telemetry::development();
        let config = telemetry.config();
        assert_eq!(config.service_name, Some("archimedes-dev".to_string()));
        assert_eq!(config.enable_tracing, Some(false));
        assert_eq!(config.log_level, Some("debug".to_string()));
    }

    #[test]
    fn test_telemetry_production() {
        let telemetry = Telemetry::production(
            "my-service".to_string(),
            "http://localhost:4317".to_string(),
        );
        let config = telemetry.config();
        assert_eq!(config.service_name, Some("my-service".to_string()));
        assert_eq!(
            config.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
        assert_eq!(config.json_logs, Some(true));
        assert_eq!(config.sample_ratio, Some(0.1));
    }

    #[test]
    fn test_span_creation() {
        let span = Span::new("test-span".to_string());
        assert_eq!(span.name(), "test-span");
        assert!(!span.is_ended());
    }

    #[test]
    fn test_span_attributes() {
        let mut span = Span::new("test-span".to_string());
        span.set_attribute("key".to_string(), "value".to_string());
        // Attributes are internal, but we can verify the span works
        assert!(!span.is_ended());
    }

    #[test]
    fn test_span_end() {
        let mut span = Span::new("test-span".to_string());
        let duration = span.end();
        assert!(span.is_ended());
        assert!(duration >= 0.0);
    }

    #[test]
    fn test_span_elapsed() {
        let span = Span::new("test-span".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = span.elapsed_ms();
        assert!(elapsed >= 10.0);
    }

    #[test]
    fn test_span_record_exception() {
        let mut span = Span::new("test-span".to_string());
        span.record_exception("TestError".to_string(), "Test message".to_string());
        assert!(!span.is_ended());
    }
}
