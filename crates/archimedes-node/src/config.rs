//! Configuration types for Archimedes Node.js bindings.

use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server configuration exposed to JavaScript.
///
/// ## Example
///
/// ```typescript
/// const config = new Config({
///   contractPath: './contract.json',
///   listenPort: 8080,
///   opaEndpoint: 'http://localhost:8181',
/// });
/// ```
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the contract JSON file
    pub contract_path: Option<String>,

    /// Port to listen on
    pub listen_port: Option<u32>,

    /// Host to bind to (default: "0.0.0.0")
    pub listen_host: Option<String>,

    /// OPA endpoint for authorization
    pub opa_endpoint: Option<String>,

    /// OPA policy path (default: "archimedes/allow")
    pub opa_policy_path: Option<String>,

    /// Enable request validation (default: true)
    pub enable_validation: Option<bool>,

    /// Enable authorization (default: true)
    pub enable_authorization: Option<bool>,

    /// Enable telemetry (default: true)
    pub enable_telemetry: Option<bool>,

    /// Request timeout in milliseconds
    pub request_timeout_ms: Option<u32>,

    /// Maximum request body size in bytes
    pub max_body_size: Option<u32>,

    /// Enable CORS (default: false)
    pub enable_cors: Option<bool>,

    /// CORS allowed origins
    pub cors_origins: Option<Vec<String>>,

    /// Additional custom configuration
    pub custom: Option<HashMap<String, String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            contract_path: None,
            listen_port: Some(8080),
            listen_host: Some("0.0.0.0".to_string()),
            opa_endpoint: None,
            opa_policy_path: Some("archimedes/allow".to_string()),
            enable_validation: Some(true),
            enable_authorization: Some(true),
            enable_telemetry: Some(true),
            request_timeout_ms: Some(30000),
            max_body_size: Some(10 * 1024 * 1024), // 10MB
            enable_cors: Some(false),
            cors_origins: None,
            custom: None,
        }
    }
}

/// Configuration builder for programmatic construction.
#[napi]
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    config: Config,
}

#[napi]
impl ConfigBuilder {
    /// Create a new configuration builder.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Set the contract path.
    #[napi]
    pub fn contract_path(&mut self, path: String) -> &Self {
        self.config.contract_path = Some(path);
        self
    }

    /// Set the listen port.
    #[napi]
    pub fn listen_port(&mut self, port: u32) -> &Self {
        self.config.listen_port = Some(port);
        self
    }

    /// Set the listen host.
    #[napi]
    pub fn listen_host(&mut self, host: String) -> &Self {
        self.config.listen_host = Some(host);
        self
    }

    /// Set the OPA endpoint.
    #[napi]
    pub fn opa_endpoint(&mut self, endpoint: String) -> &Self {
        self.config.opa_endpoint = Some(endpoint);
        self
    }

    /// Set the OPA policy path.
    #[napi]
    pub fn opa_policy_path(&mut self, path: String) -> &Self {
        self.config.opa_policy_path = Some(path);
        self
    }

    /// Enable or disable validation.
    #[napi]
    pub fn enable_validation(&mut self, enable: bool) -> &Self {
        self.config.enable_validation = Some(enable);
        self
    }

    /// Enable or disable authorization.
    #[napi]
    pub fn enable_authorization(&mut self, enable: bool) -> &Self {
        self.config.enable_authorization = Some(enable);
        self
    }

    /// Enable or disable telemetry.
    #[napi]
    pub fn enable_telemetry(&mut self, enable: bool) -> &Self {
        self.config.enable_telemetry = Some(enable);
        self
    }

    /// Set the request timeout in milliseconds.
    #[napi]
    pub fn request_timeout_ms(&mut self, timeout: u32) -> &Self {
        self.config.request_timeout_ms = Some(timeout);
        self
    }

    /// Set the maximum body size in bytes.
    #[napi]
    pub fn max_body_size(&mut self, size: u32) -> &Self {
        self.config.max_body_size = Some(size);
        self
    }

    /// Enable or disable CORS.
    #[napi]
    pub fn enable_cors(&mut self, enable: bool) -> &Self {
        self.config.enable_cors = Some(enable);
        self
    }

    /// Set CORS allowed origins.
    #[napi]
    pub fn cors_origins(&mut self, origins: Vec<String>) -> &Self {
        self.config.cors_origins = Some(origins);
        self
    }

    /// Add a custom configuration value.
    #[napi]
    pub fn custom(&mut self, key: String, value: String) -> &Self {
        let custom = self.config.custom.get_or_insert_with(HashMap::new);
        custom.insert(key, value);
        self
    }

    /// Build the configuration.
    #[napi]
    pub fn build(&self) -> Config {
        self.config.clone()
    }
}

/// Create a development configuration preset.
#[napi]
pub fn dev_config() -> Config {
    Config {
        contract_path: Some("contract.json".to_string()),
        listen_port: Some(8080),
        listen_host: Some("127.0.0.1".to_string()),
        enable_validation: Some(true),
        enable_authorization: Some(false), // Disabled for dev
        enable_telemetry: Some(true),
        enable_cors: Some(true),
        cors_origins: Some(vec!["*".to_string()]),
        ..Config::default()
    }
}

/// Create a production configuration preset.
#[napi]
pub fn prod_config() -> Config {
    Config {
        listen_port: Some(8080),
        listen_host: Some("0.0.0.0".to_string()),
        enable_validation: Some(true),
        enable_authorization: Some(true),
        enable_telemetry: Some(true),
        enable_cors: Some(false),
        ..Config::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.listen_port, Some(8080));
        assert_eq!(config.listen_host, Some("0.0.0.0".to_string()));
        assert_eq!(config.enable_validation, Some(true));
    }

    #[test]
    fn test_config_builder() {
        let mut builder = ConfigBuilder::new();
        builder.contract_path("test.json".to_string());
        builder.listen_port(9090);
        builder.enable_cors(true);
        let config = builder.build();

        assert_eq!(config.contract_path, Some("test.json".to_string()));
        assert_eq!(config.listen_port, Some(9090));
        assert_eq!(config.enable_cors, Some(true));
    }

    #[test]
    fn test_dev_config() {
        let config = dev_config();
        assert_eq!(config.listen_host, Some("127.0.0.1".to_string()));
        assert_eq!(config.enable_authorization, Some(false));
        assert_eq!(config.enable_cors, Some(true));
    }

    #[test]
    fn test_prod_config() {
        let config = prod_config();
        assert_eq!(config.listen_host, Some("0.0.0.0".to_string()));
        assert_eq!(config.enable_authorization, Some(true));
        assert_eq!(config.enable_cors, Some(false));
    }

    #[test]
    fn test_config_builder_custom() {
        let mut builder = ConfigBuilder::new();
        builder.custom("key1".to_string(), "value1".to_string());
        builder.custom("key2".to_string(), "value2".to_string());
        let config = builder.build();

        let custom = config.custom.unwrap();
        assert_eq!(custom.get("key1"), Some(&"value1".to_string()));
        assert_eq!(custom.get("key2"), Some(&"value2".to_string()));
    }
}
