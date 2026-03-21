//! Middleware configuration for the server.
//!
//! This module provides configuration types for enabling and configuring
//! the middleware pipeline in the Archimedes server.
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_server::{Server, MiddlewareConfig};
//!
//! let server = Server::builder()
//!     .http_addr("0.0.0.0:8080")
//!     .middleware(
//!         MiddlewareConfig::builder()
//!             .enable_identity()
//!             .enable_authorization()
//!             .build()
//!     )
//!     .build();
//! ```

#[cfg(feature = "authz")]
use std::sync::Arc;

#[cfg(feature = "authz")]
use archimedes_authz::{EvaluatorConfig, PolicyEvaluator};

#[cfg(feature = "sentinel")]
use archimedes_sentinel::Sentinel;

/// Configuration for the server middleware pipeline.
///
/// This configuration controls which middleware stages are enabled
/// and how they are configured. By default, all middleware is disabled
/// to maintain backward compatibility with existing code.
#[derive(Clone)]
pub struct MiddlewareConfig {
    /// Whether to enable the request ID middleware (always enabled when middleware is active).
    pub(crate) request_id_enabled: bool,

    /// Whether to enable the tracing middleware.
    pub(crate) tracing_enabled: bool,

    /// Whether to enable the identity extraction middleware.
    pub(crate) identity_enabled: bool,

    /// JWT secret for token validation (optional).
    pub(crate) jwt_secret: Option<String>,

    /// Trusted identity header name (optional).
    pub(crate) trusted_identity_header: Option<String>,

    /// Whether to enable the authorization middleware.
    pub(crate) authorization_enabled: bool,

    /// Policy evaluator for authorization (optional).
    #[cfg(feature = "authz")]
    pub(crate) policy_evaluator: Option<Arc<parking_lot::RwLock<PolicyEvaluator>>>,

    /// Service name for policy evaluation.
    pub(crate) service_name: Option<String>,

    /// Whether to enable request validation middleware.
    pub(crate) validation_enabled: bool,

    /// Contract validator (optional).
    #[cfg(feature = "sentinel")]
    pub(crate) sentinel: Option<Arc<Sentinel>>,

    /// Whether to enable response validation middleware.
    pub(crate) response_validation_enabled: bool,

    /// Whether to enable telemetry middleware.
    pub(crate) telemetry_enabled: bool,

    /// Whether to enable error normalization middleware.
    pub(crate) error_normalization_enabled: bool,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            request_id_enabled: true,
            tracing_enabled: true,
            identity_enabled: false,
            jwt_secret: None,
            trusted_identity_header: None,
            authorization_enabled: false,
            #[cfg(feature = "authz")]
            policy_evaluator: None,
            service_name: None,
            validation_enabled: false,
            #[cfg(feature = "sentinel")]
            sentinel: None,
            response_validation_enabled: false,
            telemetry_enabled: true,
            error_normalization_enabled: true,
        }
    }
}

impl std::fmt::Debug for MiddlewareConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiddlewareConfig")
            .field("request_id_enabled", &self.request_id_enabled)
            .field("tracing_enabled", &self.tracing_enabled)
            .field("identity_enabled", &self.identity_enabled)
            .field("authorization_enabled", &self.authorization_enabled)
            .field("validation_enabled", &self.validation_enabled)
            .field("response_validation_enabled", &self.response_validation_enabled)
            .field("telemetry_enabled", &self.telemetry_enabled)
            .field("error_normalization_enabled", &self.error_normalization_enabled)
            .finish()
    }
}

impl MiddlewareConfig {
    /// Creates a new middleware configuration builder.
    #[must_use]
    pub fn builder() -> MiddlewareConfigBuilder {
        MiddlewareConfigBuilder::default()
    }

    /// Creates a middleware configuration with all stages enabled.
    #[must_use]
    pub fn full() -> Self {
        Self {
            request_id_enabled: true,
            tracing_enabled: true,
            identity_enabled: true,
            jwt_secret: None,
            trusted_identity_header: None,
            authorization_enabled: true,
            #[cfg(feature = "authz")]
            policy_evaluator: None,
            service_name: None,
            validation_enabled: true,
            #[cfg(feature = "sentinel")]
            sentinel: None,
            response_validation_enabled: true,
            telemetry_enabled: true,
            error_normalization_enabled: true,
        }
    }

    /// Creates a minimal middleware configuration with only essential stages.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            request_id_enabled: true,
            tracing_enabled: false,
            identity_enabled: false,
            jwt_secret: None,
            trusted_identity_header: None,
            authorization_enabled: false,
            #[cfg(feature = "authz")]
            policy_evaluator: None,
            service_name: None,
            validation_enabled: false,
            #[cfg(feature = "sentinel")]
            sentinel: None,
            response_validation_enabled: false,
            telemetry_enabled: false,
            error_normalization_enabled: true,
        }
    }

    /// Returns whether identity extraction is enabled.
    #[must_use]
    pub fn identity_enabled(&self) -> bool {
        self.identity_enabled
    }

    /// Returns whether authorization is enabled.
    #[must_use]
    pub fn authorization_enabled(&self) -> bool {
        self.authorization_enabled
    }

    /// Returns whether validation is enabled.
    #[must_use]
    pub fn validation_enabled(&self) -> bool {
        self.validation_enabled
    }

    /// Returns the service name for policy evaluation.
    #[must_use]
    pub fn service_name(&self) -> Option<&str> {
        self.service_name.as_deref()
    }
}

/// Builder for [`MiddlewareConfig`].
#[derive(Default)]
pub struct MiddlewareConfigBuilder {
    config: MiddlewareConfig,
}

impl MiddlewareConfigBuilder {
    /// Enables request ID generation/propagation.
    #[must_use]
    pub fn enable_request_id(mut self) -> Self {
        self.config.request_id_enabled = true;
        self
    }

    /// Disables request ID generation/propagation.
    #[must_use]
    pub fn disable_request_id(mut self) -> Self {
        self.config.request_id_enabled = false;
        self
    }

    /// Enables tracing/OpenTelemetry span creation.
    #[must_use]
    pub fn enable_tracing(mut self) -> Self {
        self.config.tracing_enabled = true;
        self
    }

    /// Disables tracing.
    #[must_use]
    pub fn disable_tracing(mut self) -> Self {
        self.config.tracing_enabled = false;
        self
    }

    /// Enables identity extraction from headers.
    ///
    /// This enables extracting caller identity from:
    /// - `Authorization` header (Bearer JWT tokens)
    /// - `X-Caller-Identity` header (trusted proxies)
    /// - SPIFFE headers
    #[must_use]
    pub fn enable_identity(mut self) -> Self {
        self.config.identity_enabled = true;
        self
    }

    /// Disables identity extraction.
    #[must_use]
    pub fn disable_identity(mut self) -> Self {
        self.config.identity_enabled = false;
        self
    }

    /// Sets the JWT secret for token validation.
    #[must_use]
    pub fn jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.config.jwt_secret = Some(secret.into());
        self
    }

    /// Sets the trusted identity header name.
    ///
    /// When set, the server will trust identity information from this header
    /// (typically set by a trusted proxy or sidecar).
    #[must_use]
    pub fn trusted_identity_header(mut self, header: impl Into<String>) -> Self {
        self.config.trusted_identity_header = Some(header.into());
        self
    }

    /// Enables authorization policy evaluation.
    #[must_use]
    pub fn enable_authorization(mut self) -> Self {
        self.config.authorization_enabled = true;
        self
    }

    /// Disables authorization.
    #[must_use]
    pub fn disable_authorization(mut self) -> Self {
        self.config.authorization_enabled = false;
        self
    }

    /// Sets the policy evaluator for authorization.
    #[cfg(feature = "authz")]
    #[must_use]
    pub fn policy_evaluator(mut self, evaluator: PolicyEvaluator) -> Self {
        self.config.policy_evaluator = Some(Arc::new(parking_lot::RwLock::new(evaluator)));
        self
    }

    /// Sets the policy evaluator from an Arc (for sharing).
    #[cfg(feature = "authz")]
    #[must_use]
    pub fn policy_evaluator_arc(
        mut self,
        evaluator: Arc<parking_lot::RwLock<PolicyEvaluator>>,
    ) -> Self {
        self.config.policy_evaluator = Some(evaluator);
        self
    }

    /// Sets the service name for policy evaluation.
    #[must_use]
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.config.service_name = Some(name.into());
        self
    }

    /// Enables request validation against the contract.
    #[must_use]
    pub fn enable_validation(mut self) -> Self {
        self.config.validation_enabled = true;
        self
    }

    /// Disables request validation.
    #[must_use]
    pub fn disable_validation(mut self) -> Self {
        self.config.validation_enabled = false;
        self
    }

    /// Sets the Sentinel for contract validation.
    #[cfg(feature = "sentinel")]
    #[must_use]
    pub fn sentinel(mut self, sentinel: Sentinel) -> Self {
        self.config.sentinel = Some(Arc::new(sentinel));
        self
    }

    /// Sets the Sentinel from an Arc (for sharing).
    #[cfg(feature = "sentinel")]
    #[must_use]
    pub fn sentinel_arc(mut self, sentinel: Arc<Sentinel>) -> Self {
        self.config.sentinel = Some(sentinel);
        self
    }

    /// Enables response validation.
    #[must_use]
    pub fn enable_response_validation(mut self) -> Self {
        self.config.response_validation_enabled = true;
        self
    }

    /// Disables response validation.
    #[must_use]
    pub fn disable_response_validation(mut self) -> Self {
        self.config.response_validation_enabled = false;
        self
    }

    /// Enables telemetry/metrics emission.
    #[must_use]
    pub fn enable_telemetry(mut self) -> Self {
        self.config.telemetry_enabled = true;
        self
    }

    /// Disables telemetry.
    #[must_use]
    pub fn disable_telemetry(mut self) -> Self {
        self.config.telemetry_enabled = false;
        self
    }

    /// Enables error normalization.
    #[must_use]
    pub fn enable_error_normalization(mut self) -> Self {
        self.config.error_normalization_enabled = true;
        self
    }

    /// Disables error normalization.
    #[must_use]
    pub fn disable_error_normalization(mut self) -> Self {
        self.config.error_normalization_enabled = false;
        self
    }

    /// Builds the middleware configuration.
    #[must_use]
    pub fn build(self) -> MiddlewareConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_config_default() {
        let config = MiddlewareConfig::default();
        assert!(config.request_id_enabled);
        assert!(config.tracing_enabled);
        assert!(!config.identity_enabled);
        assert!(!config.authorization_enabled);
        assert!(!config.validation_enabled);
    }

    #[test]
    fn test_middleware_config_full() {
        let config = MiddlewareConfig::full();
        assert!(config.request_id_enabled);
        assert!(config.tracing_enabled);
        assert!(config.identity_enabled);
        assert!(config.authorization_enabled);
        assert!(config.validation_enabled);
        assert!(config.response_validation_enabled);
        assert!(config.telemetry_enabled);
        assert!(config.error_normalization_enabled);
    }

    #[test]
    fn test_middleware_config_minimal() {
        let config = MiddlewareConfig::minimal();
        assert!(config.request_id_enabled);
        assert!(!config.tracing_enabled);
        assert!(!config.identity_enabled);
        assert!(!config.authorization_enabled);
        assert!(!config.validation_enabled);
        assert!(config.error_normalization_enabled);
    }

    #[test]
    fn test_middleware_config_builder() {
        let config = MiddlewareConfig::builder()
            .enable_identity()
            .enable_authorization()
            .service_name("test-service")
            .jwt_secret("secret123")
            .build();

        assert!(config.identity_enabled);
        assert!(config.authorization_enabled);
        assert_eq!(config.service_name(), Some("test-service"));
        assert_eq!(config.jwt_secret, Some("secret123".to_string()));
    }

    #[test]
    fn test_middleware_config_debug() {
        let config = MiddlewareConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("MiddlewareConfig"));
        assert!(debug.contains("request_id_enabled"));
    }
}
