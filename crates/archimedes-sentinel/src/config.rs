//! Configuration for the Sentinel.
//!
//! This module provides configuration types for validation behavior
//! and Sentinel operation.

use serde::{Deserialize, Serialize};

/// Configuration for validation behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Whether to validate incoming requests.
    pub validate_requests: bool,
    /// Whether to validate outgoing responses.
    pub validate_responses: bool,
    /// Enable strict mode (fail on any validation warning).
    pub strict_mode: bool,
    /// Allow properties not defined in schema.
    pub allow_additional_properties: bool,
    /// Allow missing path parameters (useful for optional params).
    pub allow_missing_path_params: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validate_requests: true,
            validate_responses: false,
            strict_mode: false,
            allow_additional_properties: true,
            allow_missing_path_params: false,
        }
    }
}

impl ValidationConfig {
    /// Create a strict configuration that validates everything.
    pub fn strict() -> Self {
        Self {
            validate_requests: true,
            validate_responses: true,
            strict_mode: true,
            allow_additional_properties: false,
            allow_missing_path_params: false,
        }
    }

    /// Create a permissive configuration.
    pub fn permissive() -> Self {
        Self {
            validate_requests: false,
            validate_responses: false,
            strict_mode: false,
            allow_additional_properties: true,
            allow_missing_path_params: true,
        }
    }

    /// Create a request-only validation configuration.
    pub fn request_only() -> Self {
        Self {
            validate_requests: true,
            validate_responses: false,
            strict_mode: false,
            allow_additional_properties: true,
            allow_missing_path_params: false,
        }
    }
}

/// Configuration for the Sentinel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelConfig {
    /// Validation configuration.
    pub validation: ValidationConfig,
    /// Whether to cache validation results.
    pub cache_validation: bool,
    /// Maximum number of cached validations.
    pub cache_size: usize,
    /// Registry URL for loading artifacts.
    pub registry_url: Option<String>,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            validation: ValidationConfig::default(),
            cache_validation: true,
            cache_size: 1000,
            registry_url: None,
        }
    }
}

impl SentinelConfig {
    /// Create a configuration for development.
    pub fn development() -> Self {
        Self {
            validation: ValidationConfig::strict(),
            cache_validation: false,
            cache_size: 0,
            registry_url: None,
        }
    }

    /// Create a configuration for production.
    pub fn production() -> Self {
        Self {
            validation: ValidationConfig::request_only(),
            cache_validation: true,
            cache_size: 10000,
            registry_url: None,
        }
    }

    /// Set the registry URL.
    pub fn with_registry(mut self, url: impl Into<String>) -> Self {
        self.registry_url = Some(url.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_validation_config() {
        let config = ValidationConfig::default();
        assert!(config.validate_requests);
        assert!(!config.validate_responses);
        assert!(!config.strict_mode);
        assert!(config.allow_additional_properties);
    }

    #[test]
    fn test_strict_validation_config() {
        let config = ValidationConfig::strict();
        assert!(config.validate_requests);
        assert!(config.validate_responses);
        assert!(config.strict_mode);
        assert!(!config.allow_additional_properties);
    }

    #[test]
    fn test_permissive_validation_config() {
        let config = ValidationConfig::permissive();
        assert!(!config.validate_requests);
        assert!(!config.validate_responses);
        assert!(!config.strict_mode);
        assert!(config.allow_additional_properties);
    }

    #[test]
    fn test_default_sentinel_config() {
        let config = SentinelConfig::default();
        assert!(config.cache_validation);
        assert_eq!(config.cache_size, 1000);
        assert!(config.registry_url.is_none());
    }

    #[test]
    fn test_sentinel_config_with_registry() {
        let config = SentinelConfig::default().with_registry("http://registry.example.com");
        assert_eq!(
            config.registry_url,
            Some("http://registry.example.com".to_string())
        );
    }

    #[test]
    fn test_development_config() {
        let config = SentinelConfig::development();
        assert!(!config.cache_validation);
        assert!(config.validation.strict_mode);
    }

    #[test]
    fn test_production_config() {
        let config = SentinelConfig::production();
        assert!(config.cache_validation);
        assert_eq!(config.cache_size, 10000);
        assert!(config.validation.validate_requests);
        assert!(!config.validation.validate_responses);
    }
}
