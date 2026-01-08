//! Main configuration types.
//!
//! This module provides the top-level [`ArchimedesConfig`] struct and its builder.

use serde::{Deserialize, Serialize};

use crate::{AuthorizationConfig, ContractConfig, ServerConfig, TelemetryConfigSection};

/// Complete Archimedes server configuration.
///
/// This is the root configuration type that contains all configuration sections.
/// Use [`ConfigLoader`](crate::ConfigLoader) to load configuration from files
/// and environment variables.
///
/// # Example
///
/// ```
/// use archimedes_config::ArchimedesConfig;
///
/// let config = ArchimedesConfig::default();
/// assert_eq!(config.server.http_addr, "0.0.0.0:8080");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ArchimedesConfig {
    /// Server configuration.
    #[serde(default)]
    pub server: ServerConfig,

    /// Telemetry configuration (metrics, tracing, logging).
    #[serde(default)]
    pub telemetry: TelemetryConfigSection,

    /// Authorization configuration.
    #[serde(default)]
    pub authorization: AuthorizationConfig,

    /// Contract validation configuration.
    #[serde(default)]
    pub contract: ContractConfig,
}

impl ArchimedesConfig {
    /// Create a new configuration builder.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::{ArchimedesConfig, ServerConfig};
    ///
    /// let config = ArchimedesConfig::builder()
    ///     .server(ServerConfig {
    ///         http_addr: "127.0.0.1:3000".to_string(),
    ///         ..Default::default()
    ///     })
    ///     .build();
    ///
    /// assert_eq!(config.server.http_addr, "127.0.0.1:3000");
    /// ```
    #[must_use]
    pub fn builder() -> ArchimedesConfigBuilder {
        ArchimedesConfigBuilder::new()
    }

    /// Validate the configuration.
    ///
    /// Returns `Ok(())` if the configuration is valid, or an error describing
    /// the validation failure.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ValidationError` if:
    /// - Server address is invalid
    /// - Metrics address is invalid
    /// - Sampling ratio is not in 0.0..=1.0
    /// - Required fields are missing when features are enabled
    pub fn validate(&self) -> Result<(), crate::ConfigError> {
        // Validate server address format
        if self
            .server
            .http_addr
            .parse::<std::net::SocketAddr>()
            .is_err()
        {
            return Err(crate::ConfigError::invalid_value(
                "server.http_addr",
                format!("invalid socket address: {}", self.server.http_addr),
            ));
        }

        // Validate metrics address if enabled
        if self.telemetry.metrics.enabled
            && self
                .telemetry
                .metrics
                .addr
                .parse::<std::net::SocketAddr>()
                .is_err()
        {
            return Err(crate::ConfigError::invalid_value(
                "telemetry.metrics.addr",
                format!("invalid socket address: {}", self.telemetry.metrics.addr),
            ));
        }

        // Validate sampling ratio
        if !(0.0..=1.0).contains(&self.telemetry.tracing.sampling_ratio) {
            return Err(crate::ConfigError::invalid_value(
                "telemetry.tracing.sampling_ratio",
                "must be between 0.0 and 1.0",
            ));
        }

        // Validate OPA endpoint is set when mode is OPA
        if self.authorization.enabled
            && self.authorization.mode == crate::AuthorizationMode::Opa
            && self.authorization.opa_endpoint.is_none()
            && self.authorization.policy_bundle_path.is_none()
        {
            return Err(crate::ConfigError::validation_error(
                "authorization.opa_endpoint or authorization.policy_bundle_path must be set when mode is 'opa'",
            ));
        }

        Ok(())
    }

    /// Create a development configuration preset.
    ///
    /// This preset is optimized for local development with:
    /// - Pretty log formatting with ANSI colors
    /// - Debug log level
    /// - Allow-all authorization mode
    /// - Disabled response validation
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ArchimedesConfig;
    ///
    /// let config = ArchimedesConfig::development();
    /// assert_eq!(config.telemetry.logging.level, "debug");
    /// ```
    #[must_use]
    pub fn development() -> Self {
        let mut config = Self::default();

        // Development logging
        config.telemetry.logging.level = "debug".to_string();
        config.telemetry.logging.format = crate::LogFormat::Pretty;
        config.telemetry.logging.ansi_enabled = true;
        config.telemetry.logging.include_location = true;

        // Development environment
        config.telemetry.environment = "development".to_string();

        // Relaxed authorization for dev
        config.authorization.mode = crate::AuthorizationMode::AllowAll;

        // Relaxed validation for dev
        config.contract.validate_responses = false;

        config
    }

    /// Create a production configuration preset.
    ///
    /// This preset is optimized for production with:
    /// - JSON log formatting
    /// - Info log level
    /// - RBAC authorization mode
    /// - Strict validation enabled
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ArchimedesConfig;
    ///
    /// let config = ArchimedesConfig::production();
    /// assert_eq!(config.telemetry.logging.format, archimedes_config::LogFormat::Json);
    /// ```
    #[must_use]
    pub fn production() -> Self {
        let mut config = Self::default();

        // Production logging
        config.telemetry.logging.level = "info".to_string();
        config.telemetry.logging.format = crate::LogFormat::Json;
        config.telemetry.logging.ansi_enabled = false;

        // Production environment
        config.telemetry.environment = "production".to_string();

        // Strict authorization
        config.authorization.mode = crate::AuthorizationMode::Rbac;

        // Strict validation
        config.contract.strict_validation = true;
        config.contract.validate_responses = true;

        config
    }
}

/// Builder for [`ArchimedesConfig`].
#[derive(Debug, Default)]
pub struct ArchimedesConfigBuilder {
    server: Option<ServerConfig>,
    telemetry: Option<TelemetryConfigSection>,
    authorization: Option<AuthorizationConfig>,
    contract: Option<ContractConfig>,
}

impl ArchimedesConfigBuilder {
    /// Create a new builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server configuration.
    #[must_use]
    pub fn server(mut self, server: ServerConfig) -> Self {
        self.server = Some(server);
        self
    }

    /// Set the telemetry configuration.
    #[must_use]
    pub fn telemetry(mut self, telemetry: TelemetryConfigSection) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Set the authorization configuration.
    #[must_use]
    pub fn authorization(mut self, authorization: AuthorizationConfig) -> Self {
        self.authorization = Some(authorization);
        self
    }

    /// Set the contract configuration.
    #[must_use]
    pub fn contract(mut self, contract: ContractConfig) -> Self {
        self.contract = Some(contract);
        self
    }

    /// Build the configuration.
    ///
    /// Any unset sections will use their default values.
    #[must_use]
    pub fn build(self) -> ArchimedesConfig {
        ArchimedesConfig {
            server: self.server.unwrap_or_default(),
            telemetry: self.telemetry.unwrap_or_default(),
            authorization: self.authorization.unwrap_or_default(),
            contract: self.contract.unwrap_or_default(),
        }
    }

    /// Build and validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if validation fails.
    pub fn build_validated(self) -> Result<ArchimedesConfig, crate::ConfigError> {
        let config = self.build();
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ArchimedesConfig::default();
        assert_eq!(config.server.http_addr, "0.0.0.0:8080");
        assert_eq!(config.telemetry.service_name, "archimedes-service");
        assert!(config.authorization.enabled);
        assert!(config.contract.enabled);
    }

    #[test]
    fn test_builder_server() {
        let config = ArchimedesConfig::builder()
            .server(ServerConfig {
                http_addr: "127.0.0.1:3000".to_string(),
                ..Default::default()
            })
            .build();

        assert_eq!(config.server.http_addr, "127.0.0.1:3000");
        // Other sections use defaults
        assert_eq!(config.telemetry.service_name, "archimedes-service");
    }

    #[test]
    fn test_builder_all_sections() {
        let config = ArchimedesConfig::builder()
            .server(ServerConfig {
                http_addr: "127.0.0.1:3000".to_string(),
                ..Default::default()
            })
            .telemetry(TelemetryConfigSection {
                service_name: "my-service".to_string(),
                ..Default::default()
            })
            .authorization(AuthorizationConfig {
                mode: crate::AuthorizationMode::AllowAll,
                ..Default::default()
            })
            .contract(ContractConfig {
                strict_validation: false,
                ..Default::default()
            })
            .build();

        assert_eq!(config.server.http_addr, "127.0.0.1:3000");
        assert_eq!(config.telemetry.service_name, "my-service");
        assert_eq!(
            config.authorization.mode,
            crate::AuthorizationMode::AllowAll
        );
        assert!(!config.contract.strict_validation);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = ArchimedesConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_server_addr() {
        let config = ArchimedesConfig::builder()
            .server(ServerConfig {
                http_addr: "not-an-address".to_string(),
                ..Default::default()
            })
            .build();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("http_addr"));
    }

    #[test]
    fn test_validate_invalid_metrics_addr() {
        let config = ArchimedesConfig::builder()
            .telemetry(TelemetryConfigSection {
                metrics: crate::MetricsConfig {
                    enabled: true,
                    addr: "invalid".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .build();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("metrics.addr"));
    }

    #[test]
    fn test_validate_invalid_sampling_ratio() {
        let config = ArchimedesConfig::builder()
            .telemetry(TelemetryConfigSection {
                tracing: crate::TracingConfig {
                    sampling_ratio: 2.0,
                    ..Default::default()
                },
                ..Default::default()
            })
            .build();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sampling_ratio"));
    }

    #[test]
    fn test_validate_opa_without_endpoint() {
        let config = ArchimedesConfig::builder()
            .authorization(AuthorizationConfig {
                enabled: true,
                mode: crate::AuthorizationMode::Opa,
                opa_endpoint: None,
                policy_bundle_path: None,
                ..Default::default()
            })
            .build();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("opa_endpoint"));
    }

    #[test]
    fn test_validate_opa_with_endpoint() {
        let config = ArchimedesConfig::builder()
            .authorization(AuthorizationConfig {
                enabled: true,
                mode: crate::AuthorizationMode::Opa,
                opa_endpoint: Some("http://localhost:8181".to_string()),
                ..Default::default()
            })
            .build();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_development_preset() {
        let config = ArchimedesConfig::development();
        assert_eq!(config.telemetry.logging.level, "debug");
        assert_eq!(config.telemetry.logging.format, crate::LogFormat::Pretty);
        assert!(config.telemetry.logging.ansi_enabled);
        assert_eq!(
            config.authorization.mode,
            crate::AuthorizationMode::AllowAll
        );
        assert!(!config.contract.validate_responses);
    }

    #[test]
    fn test_production_preset() {
        let config = ArchimedesConfig::production();
        assert_eq!(config.telemetry.logging.level, "info");
        assert_eq!(config.telemetry.logging.format, crate::LogFormat::Json);
        assert!(!config.telemetry.logging.ansi_enabled);
        assert_eq!(config.authorization.mode, crate::AuthorizationMode::Rbac);
        assert!(config.contract.validate_responses);
    }

    #[test]
    fn test_build_validated_success() {
        let result = ArchimedesConfig::builder().build_validated();
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_validated_failure() {
        let result = ArchimedesConfig::builder()
            .server(ServerConfig {
                http_addr: "invalid".to_string(),
                ..Default::default()
            })
            .build_validated();

        assert!(result.is_err());
    }

    #[test]
    fn test_toml_serialization() {
        let config = ArchimedesConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("[telemetry]"));
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
            [server]
            http_addr = "127.0.0.1:8000"

            [telemetry]
            service_name = "test-service"
        "#;

        let config: ArchimedesConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.http_addr, "127.0.0.1:8000");
        assert_eq!(config.telemetry.service_name, "test-service");
    }

    #[test]
    fn test_unknown_field_rejected() {
        let toml_str = r#"
            [server]
            http_addr = "127.0.0.1:8000"
            unknown_field = "value"
        "#;

        let result: Result<ArchimedesConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }
}
