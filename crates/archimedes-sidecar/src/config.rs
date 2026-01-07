//! Configuration for the Archimedes sidecar.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{SidecarError, SidecarResult};

/// Sidecar configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SidecarConfig {
    /// Sidecar server settings.
    pub sidecar: SidecarSettings,
    /// Contract settings.
    pub contract: ContractSettings,
    /// Policy settings.
    pub policy: PolicySettings,
    /// Telemetry settings.
    pub telemetry: TelemetrySettings,
    /// Identity settings.
    pub identity: IdentitySettings,
}

impl SidecarConfig {
    /// Create a new configuration builder.
    pub fn builder() -> SidecarConfigBuilder {
        SidecarConfigBuilder::default()
    }

    /// Load configuration from a file.
    pub fn from_file(path: impl Into<PathBuf>) -> SidecarResult<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| SidecarError::config(format!("failed to read config file: {e}")))?;

        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match extension {
            "toml" => toml::from_str(&content)
                .map_err(|e| SidecarError::config(format!("invalid TOML: {e}"))),
            "json" => serde_json::from_str(&content)
                .map_err(|e| SidecarError::config(format!("invalid JSON: {e}"))),
            _ => Err(SidecarError::config(format!(
                "unsupported config format: {extension}"
            ))),
        }
    }

    /// Apply environment variable overrides.
    ///
    /// Environment variables are prefixed with `ARCHIMEDES_SIDECAR_` and use
    /// uppercase `snake_case`.
    #[must_use]
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(port) = std::env::var("ARCHIMEDES_SIDECAR_LISTEN_PORT") {
            if let Ok(port) = port.parse() {
                self.sidecar.listen_port = port;
            }
        }

        if let Ok(url) = std::env::var("ARCHIMEDES_SIDECAR_UPSTREAM_URL") {
            self.sidecar.upstream_url = url;
        }

        if let Ok(timeout) = std::env::var("ARCHIMEDES_SIDECAR_UPSTREAM_TIMEOUT") {
            if let Ok(secs) = timeout.parse::<u64>() {
                self.sidecar.upstream_timeout = Duration::from_secs(secs);
            }
        }

        if let Ok(path) = std::env::var("ARCHIMEDES_SIDECAR_CONTRACT_PATH") {
            self.contract.path = Some(PathBuf::from(path));
        }

        if let Ok(path) = std::env::var("ARCHIMEDES_SIDECAR_POLICY_BUNDLE_PATH") {
            self.policy.bundle_path = Some(PathBuf::from(path));
        }

        if let Ok(endpoint) = std::env::var("ARCHIMEDES_SIDECAR_OTLP_ENDPOINT") {
            self.telemetry.otlp_endpoint = Some(endpoint);
        }

        if let Ok(port) = std::env::var("ARCHIMEDES_SIDECAR_METRICS_PORT") {
            if let Ok(port) = port.parse() {
                self.telemetry.metrics_port = port;
            }
        }

        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> SidecarResult<()> {
        if self.sidecar.upstream_url.is_empty() {
            return Err(SidecarError::config("upstream_url is required"));
        }

        if !self.sidecar.upstream_url.starts_with("http://")
            && !self.sidecar.upstream_url.starts_with("https://")
        {
            return Err(SidecarError::config(
                "upstream_url must start with http:// or https://",
            ));
        }

        Ok(())
    }
}

/// Sidecar server settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SidecarSettings {
    /// Port the sidecar listens on.
    pub listen_port: u16,
    /// Address to bind to.
    pub listen_addr: String,
    /// Upstream service URL.
    pub upstream_url: String,
    /// Timeout for upstream requests.
    #[serde(with = "humantime_serde")]
    pub upstream_timeout: Duration,
    /// Health check path on upstream.
    pub upstream_health_path: String,
    /// Enable request body buffering.
    pub buffer_request_body: bool,
    /// Maximum request body size in bytes.
    pub max_request_body_size: usize,
    /// Enable response body buffering (for validation).
    pub buffer_response_body: bool,
    /// Maximum response body size in bytes.
    pub max_response_body_size: usize,
}

impl Default for SidecarSettings {
    fn default() -> Self {
        Self {
            listen_port: 8080,
            listen_addr: "0.0.0.0".to_string(),
            upstream_url: "http://localhost:3000".to_string(),
            upstream_timeout: Duration::from_secs(30),
            upstream_health_path: "/health".to_string(),
            buffer_request_body: true,
            max_request_body_size: 10 * 1024 * 1024, // 10MB
            buffer_response_body: false,
            max_response_body_size: 50 * 1024 * 1024, // 50MB
        }
    }
}

/// Contract validation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContractSettings {
    /// Path to contract artifact.
    pub path: Option<PathBuf>,
    /// Watch for contract file changes.
    pub watch: bool,
    /// Enable request validation.
    pub validate_requests: bool,
    /// Enable response validation.
    pub validate_responses: bool,
    /// Validation mode (enforce or monitor).
    pub mode: ValidationMode,
}

impl Default for ContractSettings {
    fn default() -> Self {
        Self {
            path: None,
            watch: true,
            validate_requests: true,
            validate_responses: false,
            mode: ValidationMode::Enforce,
        }
    }
}

/// Validation mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationMode {
    /// Enforce validation - reject invalid requests.
    #[default]
    Enforce,
    /// Monitor only - log but don't reject.
    Monitor,
}

/// Policy evaluation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PolicySettings {
    /// Path to OPA policy bundle.
    pub bundle_path: Option<PathBuf>,
    /// Watch for policy file changes.
    pub watch: bool,
    /// Enable authorization.
    pub enabled: bool,
    /// Default decision when no policy matches.
    pub default_deny: bool,
}

impl Default for PolicySettings {
    fn default() -> Self {
        Self {
            bundle_path: None,
            watch: true,
            enabled: true,
            default_deny: true,
        }
    }
}

/// Telemetry settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetrySettings {
    /// OTLP endpoint for traces.
    pub otlp_endpoint: Option<String>,
    /// Prometheus metrics port.
    pub metrics_port: u16,
    /// Service name for telemetry.
    pub service_name: String,
    /// Enable access logging.
    pub access_log: bool,
    /// Log level.
    pub log_level: String,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            metrics_port: 9090,
            service_name: "archimedes-sidecar".to_string(),
            access_log: true,
            log_level: "info".to_string(),
        }
    }
}

/// Identity extraction settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IdentitySettings {
    /// Enable mTLS identity extraction.
    pub mtls_enabled: bool,
    /// Path to client certificate (for mTLS).
    pub mtls_cert: Option<PathBuf>,
    /// Path to client key (for mTLS).
    pub mtls_key: Option<PathBuf>,
    /// Path to CA certificate (for mTLS).
    pub mtls_ca: Option<PathBuf>,
    /// Enable JWT identity extraction.
    pub jwt_enabled: bool,
    /// JWT issuer for validation.
    pub jwt_issuer: Option<String>,
    /// JWT audience for validation.
    pub jwt_audience: Option<String>,
    /// Enable API key identity extraction.
    pub api_key_enabled: bool,
    /// API key header name.
    pub api_key_header: String,
}

impl Default for IdentitySettings {
    fn default() -> Self {
        Self {
            mtls_enabled: false,
            mtls_cert: None,
            mtls_key: None,
            mtls_ca: None,
            jwt_enabled: true,
            jwt_issuer: None,
            jwt_audience: None,
            api_key_enabled: true,
            api_key_header: "X-Api-Key".to_string(),
        }
    }
}

/// Builder for `SidecarConfig`.
#[derive(Debug, Default)]
pub struct SidecarConfigBuilder {
    config: SidecarConfig,
}

impl SidecarConfigBuilder {
    /// Set the listen port.
    #[must_use]
    pub fn listen_port(mut self, port: u16) -> Self {
        self.config.sidecar.listen_port = port;
        self
    }

    /// Set the listen address.
    #[must_use]
    pub fn listen_addr(mut self, addr: impl Into<String>) -> Self {
        self.config.sidecar.listen_addr = addr.into();
        self
    }

    /// Set the upstream URL.
    #[must_use]
    pub fn upstream_url(mut self, url: impl Into<String>) -> Self {
        self.config.sidecar.upstream_url = url.into();
        self
    }

    /// Set the upstream timeout.
    #[must_use]
    pub fn upstream_timeout(mut self, timeout: Duration) -> Self {
        self.config.sidecar.upstream_timeout = timeout;
        self
    }

    /// Set the contract path.
    #[must_use]
    pub fn contract_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.contract.path = Some(path.into());
        self
    }

    /// Set the policy bundle path.
    #[must_use]
    pub fn policy_bundle_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.policy.bundle_path = Some(path.into());
        self
    }

    /// Set the OTLP endpoint.
    #[must_use]
    pub fn otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.config.telemetry.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Set the metrics port.
    #[must_use]
    pub fn metrics_port(mut self, port: u16) -> Self {
        self.config.telemetry.metrics_port = port;
        self
    }

    /// Set the service name.
    #[must_use]
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.config.telemetry.service_name = name.into();
        self
    }

    /// Enable mTLS.
    #[must_use]
    pub fn mtls(
        mut self,
        cert: impl Into<PathBuf>,
        key: impl Into<PathBuf>,
        ca: impl Into<PathBuf>,
    ) -> Self {
        self.config.identity.mtls_enabled = true;
        self.config.identity.mtls_cert = Some(cert.into());
        self.config.identity.mtls_key = Some(key.into());
        self.config.identity.mtls_ca = Some(ca.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> SidecarResult<SidecarConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Custom deserializer for Duration using humantime format.
mod humantime_serde {
    use std::time::Duration;

    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}s", duration.as_secs());
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_duration(&s).map_err(serde::de::Error::custom)
    }

    fn parse_duration(s: &str) -> Result<Duration, String> {
        let s = s.trim();
        if let Some(stripped) = s.strip_suffix("ms") {
            let n: u64 = stripped
                .trim()
                .parse()
                .map_err(|_| "invalid duration")?;
            Ok(Duration::from_millis(n))
        } else if let Some(stripped) = s.strip_suffix('s') {
            let n: u64 = stripped
                .trim()
                .parse()
                .map_err(|_| "invalid duration")?;
            Ok(Duration::from_secs(n))
        } else if let Some(stripped) = s.strip_suffix('m') {
            let n: u64 = stripped
                .trim()
                .parse()
                .map_err(|_| "invalid duration")?;
            Ok(Duration::from_secs(n * 60))
        } else if let Some(stripped) = s.strip_suffix('h') {
            let n: u64 = stripped
                .trim()
                .parse()
                .map_err(|_| "invalid duration")?;
            Ok(Duration::from_secs(n * 3600))
        } else {
            // Assume seconds
            let n: u64 = s.parse().map_err(|_| "invalid duration")?;
            Ok(Duration::from_secs(n))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SidecarConfig::default();
        assert_eq!(config.sidecar.listen_port, 8080);
        assert_eq!(config.sidecar.upstream_url, "http://localhost:3000");
        assert_eq!(config.sidecar.upstream_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_config_builder() {
        let config = SidecarConfig::builder()
            .listen_port(9000)
            .upstream_url("http://app:3000")
            .upstream_timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        assert_eq!(config.sidecar.listen_port, 9000);
        assert_eq!(config.sidecar.upstream_url, "http://app:3000");
        assert_eq!(config.sidecar.upstream_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_config_validation() {
        let config = SidecarConfig::builder()
            .upstream_url("")
            .build();
        assert!(config.is_err());

        let config = SidecarConfig::builder()
            .upstream_url("invalid-url")
            .build();
        assert!(config.is_err());

        let config = SidecarConfig::builder()
            .upstream_url("http://localhost:3000")
            .build();
        assert!(config.is_ok());
    }

    #[test]
    fn test_validation_mode() {
        assert_eq!(
            serde_json::from_str::<ValidationMode>(r#""enforce""#).unwrap(),
            ValidationMode::Enforce
        );
        assert_eq!(
            serde_json::from_str::<ValidationMode>(r#""monitor""#).unwrap(),
            ValidationMode::Monitor
        );
    }

    #[test]
    fn test_toml_config() {
        let toml = r#"
[sidecar]
listen_port = 8080
upstream_url = "http://localhost:3000"
upstream_timeout = "30s"

[contract]
validate_requests = true

[telemetry]
service_name = "test-service"
"#;
        let config: SidecarConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.sidecar.listen_port, 8080);
        assert!(config.contract.validate_requests);
        assert_eq!(config.telemetry.service_name, "test-service");
    }
}
