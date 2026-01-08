//! Telemetry configuration.

use crate::logging::LogConfig;
use crate::metrics::MetricsConfig;
use crate::tracing::TracingConfig;

/// Configuration for all telemetry subsystems.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Service name (used in metrics, traces, and logs).
    pub service_name: String,

    /// Service version.
    pub service_version: String,

    /// Environment (production, staging, development).
    pub environment: String,

    /// Metrics configuration.
    pub metrics: MetricsConfig,

    /// Tracing configuration.
    pub tracing: TracingConfig,

    /// Logging configuration.
    pub logging: LogConfig,
}

impl TelemetryConfig {
    /// Creates a new configuration builder.
    #[must_use]
    pub fn builder() -> TelemetryConfigBuilder {
        TelemetryConfigBuilder::new()
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "archimedes-service".to_string(),
            service_version: "0.1.0".to_string(),
            environment: "development".to_string(),
            metrics: MetricsConfig::default(),
            tracing: TracingConfig::default(),
            logging: LogConfig::default(),
        }
    }
}

/// Builder for [`TelemetryConfig`].
#[derive(Debug, Default)]
pub struct TelemetryConfigBuilder {
    service_name: Option<String>,
    service_version: Option<String>,
    environment: Option<String>,
    metrics: Option<MetricsConfig>,
    tracing: Option<TracingConfig>,
    logging: Option<LogConfig>,
}

impl TelemetryConfigBuilder {
    /// Creates a new builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the service name.
    #[must_use]
    pub fn service_name(mut self, name: &str) -> Self {
        self.service_name = Some(name.to_string());
        self
    }

    /// Sets the service version.
    #[must_use]
    pub fn service_version(mut self, version: &str) -> Self {
        self.service_version = Some(version.to_string());
        self
    }

    /// Sets the environment.
    #[must_use]
    pub fn environment(mut self, env: &str) -> Self {
        self.environment = Some(env.to_string());
        self
    }

    /// Sets the metrics configuration.
    #[must_use]
    pub fn metrics(mut self, config: MetricsConfig) -> Self {
        self.metrics = Some(config);
        self
    }

    /// Sets the tracing configuration.
    #[must_use]
    pub fn tracing(mut self, config: TracingConfig) -> Self {
        self.tracing = Some(config);
        self
    }

    /// Sets the logging configuration.
    #[must_use]
    pub fn logging(mut self, config: LogConfig) -> Self {
        self.logging = Some(config);
        self
    }

    /// Sets the metrics endpoint address.
    #[must_use]
    pub fn metrics_addr(mut self, addr: &str) -> Self {
        let config = self.metrics.take().unwrap_or_default();
        self.metrics = Some(MetricsConfig {
            enabled: true,
            addr: addr.to_string(),
            ..config
        });
        self
    }

    /// Sets the OTLP endpoint for tracing.
    #[must_use]
    pub fn otlp_endpoint(mut self, endpoint: &str) -> Self {
        let config = self.tracing.take().unwrap_or_default();
        self.tracing = Some(TracingConfig {
            enabled: true,
            otlp_endpoint: endpoint.to_string(),
            ..config
        });
        self
    }

    /// Builds the configuration.
    #[must_use]
    pub fn build(self) -> TelemetryConfig {
        let defaults = TelemetryConfig::default();

        let service_name = self.service_name.unwrap_or(defaults.service_name);
        let service_version = self.service_version.unwrap_or(defaults.service_version);
        let environment = self.environment.unwrap_or(defaults.environment);

        // Update sub-configs with service info
        let mut metrics = self.metrics.unwrap_or(defaults.metrics);
        metrics.service_name = service_name.clone();

        let mut tracing = self.tracing.unwrap_or(defaults.tracing);
        tracing.service_name = service_name.clone();
        tracing.service_version = service_version.clone();
        tracing.environment = environment.clone();

        let mut logging = self.logging.unwrap_or(defaults.logging);
        logging.service_name = service_name.clone();

        TelemetryConfig {
            service_name,
            service_version,
            environment,
            metrics,
            tracing,
            logging,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "archimedes-service");
        assert_eq!(config.environment, "development");
    }

    #[test]
    fn test_builder_basic() {
        let config = TelemetryConfig::builder()
            .service_name("test-service")
            .service_version("2.0.0")
            .environment("production")
            .build();

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "2.0.0");
        assert_eq!(config.environment, "production");
    }

    #[test]
    fn test_builder_propagates_service_name() {
        let config = TelemetryConfig::builder()
            .service_name("my-service")
            .build();

        assert_eq!(config.metrics.service_name, "my-service");
        assert_eq!(config.tracing.service_name, "my-service");
        assert_eq!(config.logging.service_name, "my-service");
    }

    #[test]
    fn test_builder_metrics_addr() {
        let config = TelemetryConfig::builder()
            .metrics_addr("0.0.0.0:9999")
            .build();

        assert!(config.metrics.enabled);
        assert_eq!(config.metrics.addr, "0.0.0.0:9999");
    }

    #[test]
    fn test_builder_otlp_endpoint() {
        let config = TelemetryConfig::builder()
            .otlp_endpoint("http://jaeger:4317")
            .build();

        assert!(config.tracing.enabled);
        assert_eq!(config.tracing.otlp_endpoint, "http://jaeger:4317");
    }
}
