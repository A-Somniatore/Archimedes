//! FFI configuration types
//!
//! Configuration structs passed from foreign code to Archimedes.

use std::os::raw::c_char;

/// Configuration for Archimedes application
///
/// All string fields are borrowed - Archimedes will copy them internally.
/// Null pointers indicate optional fields that should use defaults.
#[repr(C)]
#[derive(Debug)]
pub struct ArchimedesConfig {
    /// Path to Themis contract JSON file (required)
    pub contract_path: *const c_char,

    /// Path to OPA policy bundle (optional, null to disable authorization)
    pub policy_bundle_path: *const c_char,

    /// Listen address (default: "0.0.0.0")
    pub listen_addr: *const c_char,

    /// Listen port (default: 8080)
    pub listen_port: u16,

    /// Metrics port (default: 9090, 0 to disable)
    pub metrics_port: u16,

    /// Enable request validation against contract (default: true)
    pub enable_validation: bool,

    /// Enable response validation against contract (default: false)
    pub enable_response_validation: bool,

    /// Enable OPA authorization (default: true if policy_bundle_path set)
    pub enable_authorization: bool,

    /// Enable OpenTelemetry tracing (default: true)
    pub enable_tracing: bool,

    /// OTLP endpoint for traces (optional, null for default)
    pub otlp_endpoint: *const c_char,

    /// Service name for telemetry (default: "archimedes-service")
    pub service_name: *const c_char,

    /// Graceful shutdown timeout in seconds (default: 30)
    pub shutdown_timeout_secs: u32,

    /// Maximum request body size in bytes (default: 1MB)
    pub max_body_size: usize,

    /// Request timeout in seconds (default: 30, 0 for no timeout)
    pub request_timeout_secs: u32,
}

impl Default for ArchimedesConfig {
    fn default() -> Self {
        Self {
            contract_path: std::ptr::null(),
            policy_bundle_path: std::ptr::null(),
            listen_addr: std::ptr::null(),
            listen_port: 8080,
            metrics_port: 9090,
            enable_validation: true,
            enable_response_validation: false,
            enable_authorization: true,
            enable_tracing: true,
            otlp_endpoint: std::ptr::null(),
            service_name: std::ptr::null(),
            shutdown_timeout_secs: 30,
            max_body_size: 1024 * 1024, // 1MB
            request_timeout_secs: 30,
        }
    }
}

/// Internal Rust configuration converted from FFI config
#[derive(Debug, Clone)]
pub(crate) struct InternalConfig {
    pub contract_path: String,
    pub policy_bundle_path: Option<String>,
    pub listen_addr: String,
    pub listen_port: u16,
    pub metrics_port: u16,
    pub enable_validation: bool,
    pub enable_response_validation: bool,
    pub enable_authorization: bool,
    pub enable_tracing: bool,
    pub otlp_endpoint: Option<String>,
    pub service_name: String,
    pub shutdown_timeout_secs: u32,
    pub max_body_size: usize,
    pub request_timeout_secs: u32,
}

impl TryFrom<&ArchimedesConfig> for InternalConfig {
    type Error = &'static str;

    fn try_from(config: &ArchimedesConfig) -> Result<Self, Self::Error> {
        use crate::c_str_to_rust;

        let contract_path = c_str_to_rust(config.contract_path)
            .ok_or("contract_path is required")?;

        let policy_bundle_path = c_str_to_rust(config.policy_bundle_path);
        let listen_addr = c_str_to_rust(config.listen_addr)
            .unwrap_or_else(|| "0.0.0.0".to_string());
        let otlp_endpoint = c_str_to_rust(config.otlp_endpoint);
        let service_name = c_str_to_rust(config.service_name)
            .unwrap_or_else(|| "archimedes-service".to_string());

        let has_policy = policy_bundle_path.is_some();

        Ok(Self {
            contract_path,
            policy_bundle_path,
            listen_addr,
            listen_port: config.listen_port,
            metrics_port: config.metrics_port,
            enable_validation: config.enable_validation,
            enable_response_validation: config.enable_response_validation,
            enable_authorization: config.enable_authorization && has_policy,
            enable_tracing: config.enable_tracing,
            otlp_endpoint,
            service_name,
            shutdown_timeout_secs: config.shutdown_timeout_secs,
            max_body_size: config.max_body_size,
            request_timeout_secs: config.request_timeout_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_config_default() {
        let config = ArchimedesConfig::default();
        assert!(config.contract_path.is_null());
        assert_eq!(config.listen_port, 8080);
        assert_eq!(config.metrics_port, 9090);
        assert!(config.enable_validation);
        assert!(!config.enable_response_validation);
    }

    #[test]
    fn test_internal_config_conversion() {
        let contract_path = CString::new("contract.json").unwrap();
        let listen_addr = CString::new("127.0.0.1").unwrap();

        let config = ArchimedesConfig {
            contract_path: contract_path.as_ptr(),
            listen_addr: listen_addr.as_ptr(),
            listen_port: 3000,
            ..Default::default()
        };

        let internal = InternalConfig::try_from(&config).unwrap();
        assert_eq!(internal.contract_path, "contract.json");
        assert_eq!(internal.listen_addr, "127.0.0.1");
        assert_eq!(internal.listen_port, 3000);
        assert!(!internal.enable_authorization); // No policy bundle
    }

    #[test]
    fn test_internal_config_requires_contract() {
        let config = ArchimedesConfig::default();
        let result = InternalConfig::try_from(&config);
        assert!(result.is_err());
    }
}
