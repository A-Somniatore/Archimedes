//! Configuration loader with layered approach.
//!
//! This module provides the [`ConfigLoader`] for loading configuration from
//! multiple sources: defaults, files, and environment variables.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use crate::{ArchimedesConfig, ConfigError};

/// Configuration loader with layered approach.
///
/// The loader applies configuration in layers, with later layers overriding
/// earlier ones:
/// 1. Default values (built into the code)
/// 2. Configuration file (TOML or JSON)
/// 3. Environment variables
///
/// # Example
///
/// ```no_run
/// use archimedes_config::ConfigLoader;
///
/// # fn main() -> Result<(), archimedes_config::ConfigError> {
/// let config = ConfigLoader::new()
///     .with_defaults()
///     .with_file("config.toml")?
///     .with_env_prefix("ARCHIMEDES")
///     .load()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ConfigLoader {
    config: ArchimedesConfig,
    env_prefix: Option<String>,
    file_loaded: bool,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader {
    /// Create a new configuration loader.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let loader = ConfigLoader::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ArchimedesConfig::default(),
            env_prefix: None,
            file_loaded: false,
        }
    }

    /// Start with default configuration values.
    ///
    /// This is called automatically by `new()`, but can be chained for clarity.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let loader = ConfigLoader::new().with_defaults();
    /// ```
    #[must_use]
    pub fn with_defaults(mut self) -> Self {
        self.config = ArchimedesConfig::default();
        self
    }

    /// Start with development preset configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let config = ConfigLoader::new()
    ///     .with_development()
    ///     .load()
    ///     .unwrap();
    ///
    /// assert_eq!(config.telemetry.logging.level, "debug");
    /// ```
    #[must_use]
    pub fn with_development(mut self) -> Self {
        self.config = ArchimedesConfig::development();
        self
    }

    /// Start with production preset configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let config = ConfigLoader::new()
    ///     .with_production()
    ///     .load()
    ///     .unwrap();
    ///
    /// assert_eq!(config.telemetry.logging.format, archimedes_config::LogFormat::Json);
    /// ```
    #[must_use]
    pub fn with_production(mut self) -> Self {
        self.config = ArchimedesConfig::production();
        self
    }

    /// Load configuration from a file.
    ///
    /// Supports TOML (.toml) and JSON (.json) formats.
    /// The file format is determined by the file extension.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if:
    /// - The file does not exist
    /// - The file cannot be read
    /// - The file contains invalid TOML/JSON
    /// - The file contains unknown fields (strict mode)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use archimedes_config::ConfigLoader;
    ///
    /// # fn main() -> Result<(), archimedes_config::ConfigError> {
    /// let loader = ConfigLoader::new()
    ///     .with_file("config.toml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::file_not_found(path));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::read_error(path, e))?;

        let file_config = Self::parse_file(&content, path)?;
        self.merge_config(file_config);
        self.file_loaded = true;

        Ok(self)
    }

    /// Load configuration from an optional file.
    ///
    /// If the file exists, loads it. If not, silently continues.
    /// This is useful for optional configuration files.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if the file exists but:
    /// - Cannot be read
    /// - Contains invalid TOML/JSON
    /// - Contains unknown fields
    ///
    /// # Example
    ///
    /// ```no_run
    /// use archimedes_config::ConfigLoader;
    ///
    /// # fn main() -> Result<(), archimedes_config::ConfigError> {
    /// let loader = ConfigLoader::new()
    ///     .with_optional_file("config.toml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_optional_file<P: AsRef<Path>>(self, path: P) -> Result<Self, ConfigError> {
        if path.as_ref().exists() {
            self.with_file(path)
        } else {
            Ok(self)
        }
    }

    /// Load configuration from a string.
    ///
    /// # Arguments
    ///
    /// * `content` - Configuration content as a string
    /// * `format` - File format ("toml" or "json")
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if parsing fails.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let toml = r#"
    ///     [server]
    ///     http_addr = "127.0.0.1:3000"
    /// "#;
    ///
    /// let config = ConfigLoader::new()
    ///     .with_string(toml, "toml")
    ///     .unwrap()
    ///     .load()
    ///     .unwrap();
    ///
    /// assert_eq!(config.server.http_addr, "127.0.0.1:3000");
    /// ```
    pub fn with_string(mut self, content: &str, format: &str) -> Result<Self, ConfigError> {
        let file_config = match format.to_lowercase().as_str() {
            "toml" => toml::from_str(content)?,
            "json" => serde_json::from_str(content)?,
            _ => {
                return Err(ConfigError::validation_error(format!(
                    "unsupported configuration format: {format}"
                )))
            }
        };

        self.merge_config(file_config);
        Ok(self)
    }

    /// Set environment variable prefix for overrides.
    ///
    /// Environment variables use the format `PREFIX__SECTION__KEY`.
    /// For example, with prefix "ARCHIMEDES":
    /// - `ARCHIMEDES__SERVER__HTTP_ADDR=0.0.0.0:9000`
    /// - `ARCHIMEDES__TELEMETRY__SERVICE_NAME=my-service`
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let loader = ConfigLoader::new()
    ///     .with_env_prefix("ARCHIMEDES");
    /// ```
    #[must_use]
    pub fn with_env_prefix(mut self, prefix: &str) -> Self {
        self.env_prefix = Some(prefix.to_uppercase());
        self
    }

    /// Load a `.env` file for environment variables.
    ///
    /// Uses the `dotenvy` crate to load variables from a file.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if the file cannot be read.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use archimedes_config::ConfigLoader;
    ///
    /// # fn main() -> Result<(), archimedes_config::ConfigError> {
    /// let loader = ConfigLoader::new()
    ///     .with_dotenv()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_dotenv(self) -> Result<Self, ConfigError> {
        // Load .env file, ignore if not found
        let _ = dotenvy::dotenv();
        Ok(self)
    }

    /// Finalize and return the loaded configuration.
    ///
    /// Applies environment variable overrides (if a prefix was set) and
    /// validates the final configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if:
    /// - Environment variable parsing fails
    /// - Configuration validation fails
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let config = ConfigLoader::new()
    ///     .load()
    ///     .unwrap();
    ///
    /// assert_eq!(config.server.http_addr, "0.0.0.0:8080");
    /// ```
    pub fn load(mut self) -> Result<ArchimedesConfig, ConfigError> {
        // Apply environment variable overrides
        if let Some(prefix) = self.env_prefix.take() {
            self.apply_env_overrides(&prefix)?;
        }

        // Validate the final configuration
        self.config.validate()?;

        Ok(self.config)
    }

    /// Finalize without validation.
    ///
    /// Use this if you want to inspect or modify the configuration
    /// before validation.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_config::ConfigLoader;
    ///
    /// let config = ConfigLoader::new()
    ///     .load_unvalidated();
    ///
    /// // Modify and validate later
    /// let _ = config.validate();
    /// ```
    #[must_use]
    pub fn load_unvalidated(self) -> ArchimedesConfig {
        self.config
    }

    // Parse configuration file based on extension
    fn parse_file(content: &str, path: &Path) -> Result<ArchimedesConfig, ConfigError> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);

        match extension.as_deref() {
            Some("toml") => Ok(toml::from_str(content)?),
            Some("json") => Ok(serde_json::from_str(content)?),
            _ => Err(ConfigError::validation_error(format!(
                "unsupported configuration file format: {}",
                path.display()
            ))),
        }
    }

    // Merge file config into current config
    fn merge_config(&mut self, file_config: ArchimedesConfig) {
        // For now, we do a full replace. In a more sophisticated implementation,
        // we could do field-by-field merging to preserve defaults for unset fields.
        self.config = file_config;
    }

    // Apply environment variable overrides
    fn apply_env_overrides(&mut self, prefix: &str) -> Result<(), ConfigError> {
        let env_vars: HashMap<String, String> = env::vars()
            .filter(|(k, _)| k.starts_with(prefix))
            .collect();

        for (key, value) in env_vars {
            self.apply_env_var(&key, &value, prefix)?;
        }

        Ok(())
    }

    // Apply a single environment variable
    fn apply_env_var(&mut self, key: &str, value: &str, prefix: &str) -> Result<(), ConfigError> {
        // Remove prefix and split by double underscore
        let key_without_prefix = key.strip_prefix(prefix)
            .and_then(|k| k.strip_prefix("__"))
            .ok_or_else(|| ConfigError::env_parse_error(key, "invalid key format"))?;

        let parts: Vec<&str> = key_without_prefix.split("__").collect();

        match parts.as_slice() {
            // Server section
            ["SERVER", "HTTP_ADDR"] => {
                self.config.server.http_addr = value.to_string();
            }
            ["SERVER", "SHUTDOWN_TIMEOUT_SECS"] => {
                self.config.server.shutdown_timeout_secs = value
                    .parse()
                    .map_err(|_| ConfigError::env_parse_error(key, "expected integer"))?;
            }
            ["SERVER", "MAX_CONNECTIONS"] => {
                self.config.server.max_connections = value
                    .parse()
                    .map_err(|_| ConfigError::env_parse_error(key, "expected integer"))?;
            }
            ["SERVER", "REQUEST_TIMEOUT_MS"] => {
                self.config.server.request_timeout_ms = value
                    .parse()
                    .map_err(|_| ConfigError::env_parse_error(key, "expected integer"))?;
            }
            ["SERVER", "KEEP_ALIVE_SECS"] => {
                self.config.server.keep_alive_secs = if value.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(value.parse().map_err(|_| {
                        ConfigError::env_parse_error(key, "expected integer or 'none'")
                    })?)
                };
            }
            ["SERVER", "HTTP2_ENABLED"] => {
                self.config.server.http2_enabled = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }

            // Telemetry section
            ["TELEMETRY", "SERVICE_NAME"] => {
                self.config.telemetry.service_name = value.to_string();
            }
            ["TELEMETRY", "SERVICE_VERSION"] => {
                self.config.telemetry.service_version = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            ["TELEMETRY", "ENVIRONMENT"] => {
                self.config.telemetry.environment = value.to_string();
            }

            // Telemetry metrics
            ["TELEMETRY", "METRICS", "ENABLED"] => {
                self.config.telemetry.metrics.enabled = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }
            ["TELEMETRY", "METRICS", "ADDR"] => {
                self.config.telemetry.metrics.addr = value.to_string();
            }

            // Telemetry tracing
            ["TELEMETRY", "TRACING", "ENABLED"] => {
                self.config.telemetry.tracing.enabled = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }
            ["TELEMETRY", "TRACING", "OTLP_ENDPOINT"] => {
                self.config.telemetry.tracing.otlp_endpoint = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            ["TELEMETRY", "TRACING", "SAMPLING_RATIO"] => {
                self.config.telemetry.tracing.sampling_ratio = value
                    .parse()
                    .map_err(|_| ConfigError::env_parse_error(key, "expected float"))?;
            }

            // Telemetry logging
            ["TELEMETRY", "LOGGING", "ENABLED"] => {
                self.config.telemetry.logging.enabled = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }
            ["TELEMETRY", "LOGGING", "LEVEL"] => {
                self.config.telemetry.logging.level = value.to_string();
            }
            ["TELEMETRY", "LOGGING", "FORMAT"] => {
                self.config.telemetry.logging.format = match value.to_lowercase().as_str() {
                    "json" => crate::LogFormat::Json,
                    "pretty" => crate::LogFormat::Pretty,
                    _ => {
                        return Err(ConfigError::env_parse_error(
                            key,
                            "expected 'json' or 'pretty'",
                        ))
                    }
                };
            }
            ["TELEMETRY", "LOGGING", "ANSI_ENABLED"] => {
                self.config.telemetry.logging.ansi_enabled = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }

            // Authorization section
            ["AUTHORIZATION", "ENABLED"] => {
                self.config.authorization.enabled = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }
            ["AUTHORIZATION", "MODE"] => {
                self.config.authorization.mode = match value.to_lowercase().as_str() {
                    "allow_all" => crate::AuthorizationMode::AllowAll,
                    "deny_all" => crate::AuthorizationMode::DenyAll,
                    "rbac" => crate::AuthorizationMode::Rbac,
                    "opa" => crate::AuthorizationMode::Opa,
                    _ => {
                        return Err(ConfigError::env_parse_error(
                            key,
                            "expected 'allow_all', 'deny_all', 'rbac', or 'opa'",
                        ))
                    }
                };
            }
            ["AUTHORIZATION", "OPA_ENDPOINT"] => {
                self.config.authorization.opa_endpoint = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }

            // Contract section
            ["CONTRACT", "ENABLED"] => {
                self.config.contract.enabled = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }
            ["CONTRACT", "STRICT_VALIDATION"] => {
                self.config.contract.strict_validation = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }
            ["CONTRACT", "CONTRACT_PATH"] => {
                self.config.contract.contract_path = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            ["CONTRACT", "VALIDATE_RESPONSES"] => {
                self.config.contract.validate_responses = parse_bool(value)
                    .ok_or_else(|| ConfigError::env_parse_error(key, "expected boolean"))?;
            }

            // Unknown key - ignore (could also warn)
            _ => {}
        }

        Ok(())
    }
}

/// Parse a boolean from a string.
fn parse_bool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_new() {
        let config = ConfigLoader::new().load().unwrap();
        assert_eq!(config.server.http_addr, "0.0.0.0:8080");
    }

    #[test]
    fn test_loader_with_defaults() {
        let config = ConfigLoader::new()
            .with_defaults()
            .load()
            .unwrap();
        assert_eq!(config.server.http_addr, "0.0.0.0:8080");
    }

    #[test]
    fn test_loader_with_development() {
        let config = ConfigLoader::new()
            .with_development()
            .load()
            .unwrap();
        assert_eq!(config.telemetry.logging.level, "debug");
        assert_eq!(config.telemetry.logging.format, crate::LogFormat::Pretty);
    }

    #[test]
    fn test_loader_with_production() {
        let config = ConfigLoader::new()
            .with_production()
            .load()
            .unwrap();
        assert_eq!(config.telemetry.logging.format, crate::LogFormat::Json);
    }

    #[test]
    fn test_loader_with_string_toml() {
        let toml = r#"
            [server]
            http_addr = "127.0.0.1:3000"
        "#;

        let config = ConfigLoader::new()
            .with_string(toml, "toml")
            .unwrap()
            .load()
            .unwrap();

        assert_eq!(config.server.http_addr, "127.0.0.1:3000");
    }

    #[test]
    fn test_loader_with_string_json() {
        let json = r#"{"server": {"http_addr": "127.0.0.1:3000"}}"#;

        let config = ConfigLoader::new()
            .with_string(json, "json")
            .unwrap()
            .load()
            .unwrap();

        assert_eq!(config.server.http_addr, "127.0.0.1:3000");
    }

    #[test]
    fn test_loader_with_file_not_found() {
        let result = ConfigLoader::new()
            .with_file("/nonexistent/config.toml");

        assert!(result.is_err());
    }

    #[test]
    fn test_loader_with_optional_file_not_found() {
        let config = ConfigLoader::new()
            .with_optional_file("/nonexistent/config.toml")
            .unwrap()
            .load()
            .unwrap();

        // Should use defaults
        assert_eq!(config.server.http_addr, "0.0.0.0:8080");
    }

    #[test]
    fn test_loader_load_unvalidated() {
        let config = ConfigLoader::new()
            .load_unvalidated();

        assert_eq!(config.server.http_addr, "0.0.0.0:8080");
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("True"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("yes"), Some(true));
        assert_eq!(parse_bool("on"), Some(true));

        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("False"), Some(false));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("no"), Some(false));
        assert_eq!(parse_bool("off"), Some(false));

        assert_eq!(parse_bool("maybe"), None);
        assert_eq!(parse_bool(""), None);
    }

    // Note: Environment variable override tests are not included because
    // Rust 2024 requires unsafe blocks for set_var/remove_var, and this
    // project forbids unsafe code. The apply_env_var method is tested
    // indirectly through integration tests with actual environment setup.

    #[test]
    fn test_apply_env_var_server_addr() {
        let mut loader = ConfigLoader::new();
        loader.apply_env_var("TEST__SERVER__HTTP_ADDR", "192.168.1.1:9000", "TEST").unwrap();
        assert_eq!(loader.config.server.http_addr, "192.168.1.1:9000");
    }

    #[test]
    fn test_apply_env_var_telemetry() {
        let mut loader = ConfigLoader::new();
        loader.apply_env_var("TEST__TELEMETRY__SERVICE_NAME", "my-service", "TEST").unwrap();
        loader.apply_env_var("TEST__TELEMETRY__LOGGING__LEVEL", "debug", "TEST").unwrap();
        assert_eq!(loader.config.telemetry.service_name, "my-service");
        assert_eq!(loader.config.telemetry.logging.level, "debug");
    }

    #[test]
    fn test_apply_env_var_boolean() {
        let mut loader = ConfigLoader::new();
        loader.apply_env_var("TEST__SERVER__HTTP2_ENABLED", "false", "TEST").unwrap();
        assert!(!loader.config.server.http2_enabled);
    }

    #[test]
    fn test_apply_env_var_invalid_integer() {
        let mut loader = ConfigLoader::new();
        let result = loader.apply_env_var("TEST__SERVER__MAX_CONNECTIONS", "not-a-number", "TEST");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_env_var_authorization_mode() {
        let mut loader = ConfigLoader::new();
        loader.apply_env_var("TEST__AUTHORIZATION__MODE", "opa", "TEST").unwrap();
        loader.apply_env_var("TEST__AUTHORIZATION__OPA_ENDPOINT", "http://localhost:8181", "TEST").unwrap();
        assert_eq!(loader.config.authorization.mode, crate::AuthorizationMode::Opa);
        assert_eq!(loader.config.authorization.opa_endpoint, Some("http://localhost:8181".to_string()));
    }

    #[test]
    fn test_apply_env_var_log_format() {
        let mut loader = ConfigLoader::new();
        loader.apply_env_var("TEST__TELEMETRY__LOGGING__FORMAT", "pretty", "TEST").unwrap();
        assert_eq!(loader.config.telemetry.logging.format, crate::LogFormat::Pretty);
    }

    #[test]
    fn test_complete_toml_config() {
        let toml = r#"
            [server]
            http_addr = "0.0.0.0:8080"
            shutdown_timeout_secs = 60
            max_connections = 5000
            request_timeout_ms = 15000
            keep_alive_secs = 120
            http2_enabled = true

            [telemetry]
            service_name = "example-service"
            service_version = "1.0.0"
            environment = "staging"

            [telemetry.metrics]
            enabled = true
            addr = "0.0.0.0:9090"

            [telemetry.tracing]
            enabled = true
            otlp_endpoint = "http://jaeger:4317"
            sampling_ratio = 0.5

            [telemetry.logging]
            enabled = true
            level = "info"
            format = "json"
            ansi_enabled = false

            [authorization]
            enabled = true
            mode = "rbac"
            allow_anonymous = ["healthCheck", "readiness"]

            [contract]
            enabled = true
            strict_validation = true
            contract_path = "/etc/contracts/api.json"
            validate_responses = true
        "#;

        let config = ConfigLoader::new()
            .with_string(toml, "toml")
            .unwrap()
            .load()
            .unwrap();

        // Verify all values were parsed correctly
        assert_eq!(config.server.http_addr, "0.0.0.0:8080");
        assert_eq!(config.server.shutdown_timeout_secs, 60);
        assert_eq!(config.telemetry.service_name, "example-service");
        assert_eq!(config.telemetry.tracing.otlp_endpoint, Some("http://jaeger:4317".to_string()));
        assert!((config.telemetry.tracing.sampling_ratio - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.authorization.mode, crate::AuthorizationMode::Rbac);
        assert_eq!(config.authorization.allow_anonymous, vec!["healthCheck", "readiness"]);
        assert_eq!(config.contract.contract_path, Some("/etc/contracts/api.json".to_string()));
    }
}
