//! Python configuration types for Archimedes

use pyo3::prelude::*;
use std::path::PathBuf;

/// Configuration for an Archimedes application
///
/// This class configures all aspects of the Archimedes HTTP server.
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import Config
///
/// config = Config(
///     contract_path="api/contract.json",
///     listen_port=8080,
///     listen_addr="0.0.0.0",
///     enable_telemetry=True,
///     log_level="info",
/// )
/// ```
#[pyclass(name = "Config")]
#[derive(Clone, Debug)]
pub struct PyConfig {
    /// Path to the contract file (JSON)
    #[pyo3(get, set)]
    pub contract_path: String,

    /// Port to listen on
    #[pyo3(get, set)]
    pub listen_port: u16,

    /// Address to bind to
    #[pyo3(get, set)]
    pub listen_addr: String,

    /// Whether to enable OpenTelemetry
    #[pyo3(get, set)]
    pub enable_telemetry: bool,

    /// Log level (trace, debug, info, warn, error)
    #[pyo3(get, set)]
    pub log_level: String,

    /// Service name for telemetry
    #[pyo3(get, set)]
    pub service_name: String,

    /// OPA bundle URL (optional)
    #[pyo3(get, set)]
    pub opa_bundle_url: Option<String>,

    /// Enable request validation
    #[pyo3(get, set)]
    pub enable_validation: bool,

    /// Enable authorization
    #[pyo3(get, set)]
    pub enable_authorization: bool,

    /// Maximum request body size in bytes
    #[pyo3(get, set)]
    pub max_body_size: usize,

    /// Request timeout in seconds
    #[pyo3(get, set)]
    pub request_timeout_secs: u64,
}

#[pymethods]
impl PyConfig {
    /// Create a new configuration
    ///
    /// Args:
    ///     contract_path: Path to the contract JSON file
    ///     listen_port: Port to listen on (default: 8080)
    ///     listen_addr: Address to bind to (default: "127.0.0.1")
    ///     enable_telemetry: Enable OpenTelemetry (default: False)
    ///     log_level: Log level (default: "info")
    ///     service_name: Service name for telemetry (default: "archimedes-py")
    ///     opa_bundle_url: URL for OPA policy bundle (optional)
    ///     enable_validation: Enable request validation (default: True)
    ///     enable_authorization: Enable authorization (default: True)
    ///     max_body_size: Maximum request body size (default: 1MB)
    ///     request_timeout_secs: Request timeout in seconds (default: 30)
    #[new]
    #[pyo3(signature = (
        contract_path,
        listen_port = 8080,
        listen_addr = "127.0.0.1".to_string(),
        enable_telemetry = false,
        log_level = "info".to_string(),
        service_name = "archimedes-py".to_string(),
        opa_bundle_url = None,
        enable_validation = true,
        enable_authorization = true,
        max_body_size = 1_048_576,
        request_timeout_secs = 30
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        contract_path: String,
        listen_port: u16,
        listen_addr: String,
        enable_telemetry: bool,
        log_level: String,
        service_name: String,
        opa_bundle_url: Option<String>,
        enable_validation: bool,
        enable_authorization: bool,
        max_body_size: usize,
        request_timeout_secs: u64,
    ) -> Self {
        Self {
            contract_path,
            listen_port,
            listen_addr,
            enable_telemetry,
            log_level,
            service_name,
            opa_bundle_url,
            enable_validation,
            enable_authorization,
            max_body_size,
            request_timeout_secs,
        }
    }

    /// Create configuration from a YAML or JSON file
    ///
    /// Args:
    ///     path: Path to the configuration file
    ///
    /// Example:
    ///     ```python
    ///     config = Config.from_file("config.yaml")
    ///     ```
    #[staticmethod]
    fn from_file(path: String) -> PyResult<Self> {
        let path = PathBuf::from(&path);
        let content = std::fs::read_to_string(&path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to read config file: {e}"))
        })?;

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match ext {
            "json" => {
                let raw: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON: {e}"))
                })?;
                Self::from_json_value(raw)
            }
            "yaml" | "yml" => {
                let raw: serde_json::Value = serde_yaml::from_str(&content).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!("Invalid YAML: {e}"))
                })?;
                Self::from_json_value(raw)
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(
                "Config file must be .json, .yaml, or .yml",
            )),
        }
    }

    /// Create configuration from environment variables
    ///
    /// Environment variables:
    ///     - ARCHIMEDES_CONTRACT_PATH
    ///     - ARCHIMEDES_PORT
    ///     - ARCHIMEDES_ADDR
    ///     - ARCHIMEDES_TELEMETRY_ENABLED
    ///     - ARCHIMEDES_LOG_LEVEL
    ///     - ARCHIMEDES_SERVICE_NAME
    ///     - ARCHIMEDES_OPA_BUNDLE_URL
    ///
    /// Example:
    ///     ```python
    ///     config = Config.from_env()
    ///     ```
    #[staticmethod]
    fn from_env() -> PyResult<Self> {
        let contract_path = std::env::var("ARCHIMEDES_CONTRACT_PATH").map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(
                "ARCHIMEDES_CONTRACT_PATH environment variable required",
            )
        })?;

        Ok(Self {
            contract_path,
            listen_port: std::env::var("ARCHIMEDES_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
            listen_addr: std::env::var("ARCHIMEDES_ADDR").unwrap_or_else(|_| "127.0.0.1".into()),
            enable_telemetry: std::env::var("ARCHIMEDES_TELEMETRY_ENABLED")
                .ok()
                .map(|s| s.to_lowercase() == "true" || s == "1")
                .unwrap_or(false),
            log_level: std::env::var("ARCHIMEDES_LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            service_name: std::env::var("ARCHIMEDES_SERVICE_NAME")
                .unwrap_or_else(|_| "archimedes-py".into()),
            opa_bundle_url: std::env::var("ARCHIMEDES_OPA_BUNDLE_URL").ok(),
            enable_validation: std::env::var("ARCHIMEDES_VALIDATION_ENABLED")
                .ok()
                .map(|s| s.to_lowercase() != "false" && s != "0")
                .unwrap_or(true),
            enable_authorization: std::env::var("ARCHIMEDES_AUTHORIZATION_ENABLED")
                .ok()
                .map(|s| s.to_lowercase() != "false" && s != "0")
                .unwrap_or(true),
            max_body_size: std::env::var("ARCHIMEDES_MAX_BODY_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1_048_576),
            request_timeout_secs: std::env::var("ARCHIMEDES_REQUEST_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        })
    }

    /// Get the full bind address
    fn bind_address(&self) -> String {
        format!("{}:{}", self.listen_addr, self.listen_port)
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "Config(contract_path={:?}, listen_port={}, listen_addr={:?})",
            self.contract_path, self.listen_port, self.listen_addr
        )
    }

    /// Convert to dictionary
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("contract_path", &self.contract_path)?;
        dict.set_item("listen_port", self.listen_port)?;
        dict.set_item("listen_addr", &self.listen_addr)?;
        dict.set_item("enable_telemetry", self.enable_telemetry)?;
        dict.set_item("log_level", &self.log_level)?;
        dict.set_item("service_name", &self.service_name)?;
        dict.set_item("opa_bundle_url", &self.opa_bundle_url)?;
        dict.set_item("enable_validation", self.enable_validation)?;
        dict.set_item("enable_authorization", self.enable_authorization)?;
        dict.set_item("max_body_size", self.max_body_size)?;
        dict.set_item("request_timeout_secs", self.request_timeout_secs)?;
        Ok(dict.into())
    }
}

impl PyConfig {
    /// Get listen address
    pub fn listen_addr(&self) -> &str {
        &self.listen_addr
    }

    /// Get listen port
    pub fn listen_port(&self) -> u16 {
        self.listen_port
    }

    fn from_json_value(value: serde_json::Value) -> PyResult<Self> {
        let obj = value.as_object().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("Config must be a JSON object")
        })?;

        let contract_path = obj
            .get("contract_path")
            .or_else(|| obj.get("contractPath"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("contract_path is required")
            })?
            .to_string();

        Ok(Self {
            contract_path,
            listen_port: obj
                .get("listen_port")
                .or_else(|| obj.get("listenPort"))
                .or_else(|| obj.get("port"))
                .and_then(|v| v.as_u64())
                .map(|v| v as u16)
                .unwrap_or(8080),
            listen_addr: obj
                .get("listen_addr")
                .or_else(|| obj.get("listenAddr"))
                .or_else(|| obj.get("addr"))
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1")
                .to_string(),
            enable_telemetry: obj
                .get("enable_telemetry")
                .or_else(|| obj.get("enableTelemetry"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            log_level: obj
                .get("log_level")
                .or_else(|| obj.get("logLevel"))
                .and_then(|v| v.as_str())
                .unwrap_or("info")
                .to_string(),
            service_name: obj
                .get("service_name")
                .or_else(|| obj.get("serviceName"))
                .and_then(|v| v.as_str())
                .unwrap_or("archimedes-py")
                .to_string(),
            opa_bundle_url: obj
                .get("opa_bundle_url")
                .or_else(|| obj.get("opaBundleUrl"))
                .and_then(|v| v.as_str())
                .map(String::from),
            enable_validation: obj
                .get("enable_validation")
                .or_else(|| obj.get("enableValidation"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            enable_authorization: obj
                .get("enable_authorization")
                .or_else(|| obj.get("enableAuthorization"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            max_body_size: obj
                .get("max_body_size")
                .or_else(|| obj.get("maxBodySize"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(1_048_576),
            request_timeout_secs: obj
                .get("request_timeout_secs")
                .or_else(|| obj.get("requestTimeoutSecs"))
                .or_else(|| obj.get("timeout"))
                .and_then(|v| v.as_u64())
                .unwrap_or(30),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|_py| {
            let config = PyConfig::new(
                "contract.json".to_string(),
                8080,
                "127.0.0.1".to_string(),
                false,
                "info".to_string(),
                "archimedes-py".to_string(),
                None,
                true,
                true,
                1_048_576,
                30,
            );

            assert_eq!(config.contract_path, "contract.json");
            assert_eq!(config.listen_port, 8080);
            assert_eq!(config.listen_addr, "127.0.0.1");
            assert!(!config.enable_telemetry);
        });
    }

    #[test]
    fn test_config_bind_address() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|_py| {
            let config = PyConfig::new(
                "contract.json".to_string(),
                3000,
                "0.0.0.0".to_string(),
                false,
                "info".to_string(),
                "test".to_string(),
                None,
                true,
                true,
                1_048_576,
                30,
            );

            assert_eq!(config.bind_address(), "0.0.0.0:3000");
        });
    }

    #[test]
    fn test_config_from_json_value() {
        let json = serde_json::json!({
            "contract_path": "api.json",
            "listen_port": 9000,
            "listen_addr": "localhost"
        });

        let config = PyConfig::from_json_value(json).unwrap();
        assert_eq!(config.contract_path, "api.json");
        assert_eq!(config.listen_port, 9000);
        assert_eq!(config.listen_addr, "localhost");
    }
}
