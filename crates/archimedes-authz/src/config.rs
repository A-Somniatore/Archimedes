//! Configuration for the authorization system.

use crate::cache::CacheConfig;

/// Configuration for the policy evaluator.
#[derive(Debug, Clone)]
pub struct EvaluatorConfig {
    /// Default policy ID to use when not specified.
    pub default_policy_id: String,
    /// Default policy version.
    pub default_policy_version: String,
    /// Query path for the allow decision.
    pub allow_query: String,
    /// Whether to enable strict mode for Rego evaluation.
    pub strict_mode: bool,
    /// Maximum evaluation time in milliseconds.
    pub max_eval_time_ms: u64,
    /// Cache configuration.
    pub cache_config: CacheConfig,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            default_policy_id: "authz".to_string(),
            default_policy_version: "1.0.0".to_string(),
            allow_query: "data.authz.allow".to_string(),
            strict_mode: false,
            max_eval_time_ms: 100,
            cache_config: CacheConfig::default(),
        }
    }
}

impl EvaluatorConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default policy ID.
    pub fn with_policy_id(mut self, policy_id: impl Into<String>) -> Self {
        self.default_policy_id = policy_id.into();
        self
    }

    /// Set the default policy version.
    pub fn with_policy_version(mut self, version: impl Into<String>) -> Self {
        self.default_policy_version = version.into();
        self
    }

    /// Set the allow query path.
    pub fn with_allow_query(mut self, query: impl Into<String>) -> Self {
        self.allow_query = query.into();
        self
    }

    /// Enable or disable strict mode.
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Set the maximum evaluation time.
    pub fn with_max_eval_time_ms(mut self, ms: u64) -> Self {
        self.max_eval_time_ms = ms;
        self
    }

    /// Set the cache configuration.
    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    /// Create a production configuration.
    pub fn production() -> Self {
        Self {
            default_policy_id: "authz".to_string(),
            default_policy_version: "1.0.0".to_string(),
            allow_query: "data.authz.allow".to_string(),
            strict_mode: true,
            max_eval_time_ms: 50,
            cache_config: CacheConfig::production(),
        }
    }

    /// Create a development configuration.
    pub fn development() -> Self {
        Self {
            default_policy_id: "authz".to_string(),
            default_policy_version: "dev".to_string(),
            allow_query: "data.authz.allow".to_string(),
            strict_mode: false,
            max_eval_time_ms: 500,
            cache_config: CacheConfig::development(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EvaluatorConfig::default();
        assert_eq!(config.default_policy_id, "authz");
        assert_eq!(config.allow_query, "data.authz.allow");
        assert!(!config.strict_mode);
    }

    #[test]
    fn test_builder_pattern() {
        let config = EvaluatorConfig::new()
            .with_policy_id("custom-policy")
            .with_strict_mode(true)
            .with_max_eval_time_ms(200);

        assert_eq!(config.default_policy_id, "custom-policy");
        assert!(config.strict_mode);
        assert_eq!(config.max_eval_time_ms, 200);
    }

    #[test]
    fn test_production_config() {
        let config = EvaluatorConfig::production();
        assert!(config.strict_mode);
        assert_eq!(config.max_eval_time_ms, 50);
    }

    #[test]
    fn test_development_config() {
        let config = EvaluatorConfig::development();
        assert!(!config.strict_mode);
        assert_eq!(config.default_policy_version, "dev");
    }
}
