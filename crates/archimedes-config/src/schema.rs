//! Configuration schema types.
//!
//! This module defines the structure of all configuration sections.

use serde::{Deserialize, Serialize};

/// Server configuration section.
///
/// Controls the HTTP server behavior including bind address, timeouts,
/// and connection limits.
///
/// # Example
///
/// ```
/// use archimedes_config::ServerConfig;
///
/// let config = ServerConfig {
///     http_addr: "0.0.0.0:8080".to_string(),
///     shutdown_timeout_secs: 30,
///     max_connections: 10000,
///     request_timeout_ms: 30000,
///     keep_alive_secs: Some(60),
///     http2_enabled: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// HTTP server bind address (e.g., "0.0.0.0:8080").
    #[serde(default = "default_http_addr")]
    pub http_addr: String,

    /// Graceful shutdown timeout in seconds.
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,

    /// Maximum number of concurrent connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Request timeout in milliseconds.
    #[serde(default = "default_request_timeout")]
    pub request_timeout_ms: u64,

    /// Keep-alive timeout in seconds. None disables keep-alive.
    #[serde(default = "default_keep_alive")]
    pub keep_alive_secs: Option<u64>,

    /// Enable HTTP/2 support.
    #[serde(default = "default_http2_enabled")]
    pub http2_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http_addr: default_http_addr(),
            shutdown_timeout_secs: default_shutdown_timeout(),
            max_connections: default_max_connections(),
            request_timeout_ms: default_request_timeout(),
            keep_alive_secs: default_keep_alive(),
            http2_enabled: default_http2_enabled(),
        }
    }
}

fn default_http_addr() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_shutdown_timeout() -> u64 {
    30
}

fn default_max_connections() -> u32 {
    10000
}

fn default_request_timeout() -> u64 {
    30000
}

#[allow(clippy::unnecessary_wraps)]
fn default_keep_alive() -> Option<u64> {
    Some(60)
}

fn default_http2_enabled() -> bool {
    true
}

/// Metrics configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MetricsConfig {
    /// Enable metrics collection and export.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Prometheus metrics endpoint address.
    #[serde(default = "default_metrics_addr")]
    pub addr: String,

    /// Histogram bucket boundaries for request duration.
    #[serde(default = "default_histogram_buckets")]
    pub histogram_buckets: Vec<f64>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            addr: default_metrics_addr(),
            histogram_buckets: default_histogram_buckets(),
        }
    }
}

fn default_metrics_addr() -> String {
    "0.0.0.0:9090".to_string()
}

fn default_histogram_buckets() -> Vec<f64> {
    vec![
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ]
}

/// Tracing configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TracingConfig {
    /// Enable distributed tracing.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// OTLP exporter endpoint (e.g., `http://localhost:4317`).
    #[serde(default)]
    pub otlp_endpoint: Option<String>,

    /// Sampling ratio (0.0 to 1.0). 1.0 means sample all traces.
    #[serde(default = "default_sampling_ratio")]
    pub sampling_ratio: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            otlp_endpoint: None,
            sampling_ratio: default_sampling_ratio(),
        }
    }
}

fn default_sampling_ratio() -> f64 {
    1.0
}

/// Log format.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON formatted logs (production).
    #[default]
    Json,
    /// Human-readable pretty format (development).
    Pretty,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    /// Enable logging.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Log level (trace, debug, info, warn, error).
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log output format.
    #[serde(default)]
    pub format: LogFormat,

    /// Include ANSI color codes in output.
    #[serde(default)]
    pub ansi_enabled: bool,

    /// Include source file and line in logs.
    #[serde(default)]
    pub include_location: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: default_log_level(),
            format: LogFormat::default(),
            ansi_enabled: false,
            include_location: false,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Telemetry configuration section.
///
/// Controls all observability features: metrics, tracing, and logging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TelemetryConfigSection {
    /// Service name for telemetry identification.
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Service version.
    #[serde(default)]
    pub service_version: Option<String>,

    /// Deployment environment (e.g., "development", "staging", "production").
    #[serde(default = "default_environment")]
    pub environment: String,

    /// Metrics configuration.
    #[serde(default)]
    pub metrics: MetricsConfig,

    /// Tracing configuration.
    #[serde(default)]
    pub tracing: TracingConfig,

    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for TelemetryConfigSection {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            service_version: None,
            environment: default_environment(),
            metrics: MetricsConfig::default(),
            tracing: TracingConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

fn default_service_name() -> String {
    "archimedes-service".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

/// Authorization mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationMode {
    /// Allow all requests (development only).
    AllowAll,
    /// Deny all requests.
    DenyAll,
    /// Role-based access control.
    #[default]
    Rbac,
    /// OPA policy evaluation.
    Opa,
}

/// Authorization configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationConfig {
    /// Enable authorization middleware.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Authorization mode.
    #[serde(default)]
    pub mode: AuthorizationMode,

    /// OPA policy endpoint (when mode is "opa").
    #[serde(default)]
    pub opa_endpoint: Option<String>,

    /// Policy bundle path (when using embedded OPA).
    #[serde(default)]
    pub policy_bundle_path: Option<String>,

    /// Allow anonymous access to specified operations.
    #[serde(default)]
    pub allow_anonymous: Vec<String>,
}

impl Default for AuthorizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: AuthorizationMode::default(),
            opa_endpoint: None,
            policy_bundle_path: None,
            allow_anonymous: Vec::new(),
        }
    }
}

/// Contract validation configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractConfig {
    /// Enable contract validation middleware.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Strict validation mode (reject requests with extra fields).
    #[serde(default = "default_true")]
    pub strict_validation: bool,

    /// Contract file path (Themis contract definition).
    #[serde(default)]
    pub contract_path: Option<String>,

    /// Validate response bodies against contract.
    #[serde(default = "default_true")]
    pub validate_responses: bool,
}

impl Default for ContractConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strict_validation: true,
            contract_path: None,
            validate_responses: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.http_addr, "0.0.0.0:8080");
        assert_eq!(config.shutdown_timeout_secs, 30);
        assert_eq!(config.max_connections, 10000);
        assert_eq!(config.request_timeout_ms, 30000);
        assert_eq!(config.keep_alive_secs, Some(60));
        assert!(config.http2_enabled);
    }

    #[test]
    fn test_server_config_deserialize() {
        let toml = r#"
            http_addr = "127.0.0.1:3000"
            shutdown_timeout_secs = 60
        "#;
        let config: ServerConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.http_addr, "127.0.0.1:3000");
        assert_eq!(config.shutdown_timeout_secs, 60);
        // Defaults applied
        assert_eq!(config.max_connections, 10000);
    }

    #[test]
    fn test_server_config_unknown_field_rejected() {
        let toml = r#"
            http_addr = "127.0.0.1:3000"
            unknown_field = "value"
        "#;
        let result: Result<ServerConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.addr, "0.0.0.0:9090");
        assert!(!config.histogram_buckets.is_empty());
    }

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert!(config.enabled);
        assert!(config.otlp_endpoint.is_none());
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.level, "info");
        assert_eq!(config.format, LogFormat::Json);
    }

    #[test]
    fn test_log_format_deserialize() {
        let json = r#""json""#;
        let format: LogFormat = serde_json::from_str(json).unwrap();
        assert_eq!(format, LogFormat::Json);

        let pretty = r#""pretty""#;
        let format: LogFormat = serde_json::from_str(pretty).unwrap();
        assert_eq!(format, LogFormat::Pretty);
    }

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfigSection::default();
        assert_eq!(config.service_name, "archimedes-service");
        assert_eq!(config.environment, "development");
    }

    #[test]
    fn test_authorization_mode_deserialize() {
        let json = r#""allow_all""#;
        let mode: AuthorizationMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, AuthorizationMode::AllowAll);

        let rbac = r#""rbac""#;
        let mode: AuthorizationMode = serde_json::from_str(rbac).unwrap();
        assert_eq!(mode, AuthorizationMode::Rbac);
    }

    #[test]
    fn test_authorization_config_default() {
        let config = AuthorizationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, AuthorizationMode::Rbac);
        assert!(config.allow_anonymous.is_empty());
    }

    #[test]
    fn test_contract_config_default() {
        let config = ContractConfig::default();
        assert!(config.enabled);
        assert!(config.strict_validation);
        assert!(config.validate_responses);
    }
}
