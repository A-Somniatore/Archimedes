//! Archimedes Sentinel - Themis Contract Integration
//!
//! This crate provides contract-aware request handling for Archimedes,
//! enabling path-to-operation resolution and request/response validation
//! based on Themis contract artifacts.
//!
//! # Overview
//!
//! Sentinel acts as the bridge between Archimedes and Themis by:
//! - Loading contract artifacts from the registry or local files
//! - Resolving incoming requests to specific operation IDs
//! - Validating request bodies against operation schemas
//! - Validating response bodies against operation schemas
//!
//! # Architecture
//!
//! ```text
//!                      ┌────────────────────────────┐
//!                      │   Themis Registry          │
//!                      └──────────┬─────────────────┘
//!                                 │ fetch artifact
//!                      ┌──────────▼─────────────────┐
//!                      │   ArtifactLoader           │
//!                      └──────────┬─────────────────┘
//!                                 │ parse
//!                      ┌──────────▼─────────────────┐
//!      HTTP Request    │   OperationResolver       │
//!          │           │   (path + method → opId)   │
//!          ▼           └──────────┬─────────────────┘
//!     ┌────────────┐              │ resolve
//!     │ Archimedes │──────────────▼
//!     │   Router   │   operationId + parameters
//!     └────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use archimedes_sentinel::{Sentinel, SentinelConfig, ArtifactLoader};
//!
//! // Create sentinel from a local artifact
//! let artifact = ArtifactLoader::from_file("contract.artifact.json").await?;
//! let sentinel = Sentinel::new(artifact, SentinelConfig::default());
//!
//! // Resolve an incoming request to an operation
//! let resolution = sentinel.resolve("GET", "/users/123")?;
//! assert_eq!(resolution.operation_id, "getUserById");
//! assert_eq!(resolution.path_params.get("userId"), Some(&"123".to_string()));
//!
//! // Validate request body
//! let result = sentinel.validate_request(&resolution.operation_id, &request_body)?;
//! assert!(result.valid);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod artifact;
pub mod config;
pub mod error;
pub mod resolver;
pub mod validation;

// Re-exports for convenience
pub use artifact::{ArtifactLoader, LoadedArtifact, LoadedOperation, SchemaRef};
pub use config::{SentinelConfig, ValidationConfig};
pub use error::{SentinelError, SentinelResult, ValidationError};
pub use resolver::{OperationResolution, OperationResolver};
pub use validation::{ParamType, SchemaValidator, ValidationResult};

/// The main Sentinel service for contract-aware request handling.
///
/// Sentinel coordinates artifact loading, operation resolution, and validation.
#[derive(Debug)]
pub struct Sentinel {
    config: SentinelConfig,
    artifact: LoadedArtifact,
    resolver: OperationResolver,
    validator: SchemaValidator,
}

impl Sentinel {
    /// Create a new Sentinel with the given artifact and configuration.
    pub fn new(artifact: LoadedArtifact, config: SentinelConfig) -> Self {
        let resolver = OperationResolver::from_artifact(&artifact);
        let validator = SchemaValidator::from_artifact(&artifact, config.validation.clone());

        Self {
            config,
            artifact,
            resolver,
            validator,
        }
    }

    /// Create a new Sentinel with default configuration.
    pub fn with_defaults(artifact: LoadedArtifact) -> Self {
        Self::new(artifact, SentinelConfig::default())
    }

    /// Get the service name from the loaded artifact.
    pub fn service_name(&self) -> &str {
        &self.artifact.service
    }

    /// Get the artifact version.
    pub fn version(&self) -> &str {
        &self.artifact.version
    }

    /// Get the artifact format.
    pub fn format(&self) -> &str {
        &self.artifact.format
    }

    /// Resolve an HTTP request to an operation.
    ///
    /// Returns the operation ID and extracted path parameters.
    pub fn resolve(&self, method: &str, path: &str) -> SentinelResult<OperationResolution> {
        self.resolver.resolve(method, path)
    }

    /// Check if an operation exists for the given method and path.
    pub fn has_operation(&self, method: &str, path: &str) -> bool {
        self.resolver.has_route(method, path)
    }

    /// Validate a request body against the operation schema.
    pub fn validate_request(
        &self,
        operation_id: &str,
        body: &serde_json::Value,
    ) -> SentinelResult<ValidationResult> {
        if !self.config.validation.validate_requests {
            return Ok(ValidationResult::success(None));
        }
        self.validator.validate_request(operation_id, &self.artifact, body)
    }

    /// Validate a response body against the operation schema.
    pub fn validate_response(
        &self,
        operation_id: &str,
        status_code: u16,
        body: &serde_json::Value,
    ) -> SentinelResult<ValidationResult> {
        if !self.config.validation.validate_responses {
            return Ok(ValidationResult::success(None));
        }
        self.validator
            .validate_response(operation_id, &self.artifact, status_code, body)
    }

    /// Get the underlying artifact.
    pub fn artifact(&self) -> &LoadedArtifact {
        &self.artifact
    }

    /// Get the operation count.
    pub fn operation_count(&self) -> usize {
        self.artifact.operations.len()
    }

    /// Get all registered HTTP methods.
    pub fn methods(&self) -> Vec<&str> {
        self.resolver.methods()
    }

    /// Get all routes for a specific method.
    pub fn routes_for_method(&self, method: &str) -> Vec<&str> {
        self.resolver.routes_for_method(method)
    }

    /// Get the configuration.
    pub fn config(&self) -> &SentinelConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use std::collections::HashMap;

    fn create_test_artifact() -> LoadedArtifact {
        LoadedArtifact {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            format: "openapi".to_string(),
            operations: vec![
                LoadedOperation {
                    id: "listUsers".to_string(),
                    method: "GET".to_string(),
                    path: "/users".to_string(),
                    summary: Some("List all users".to_string()),
                    deprecated: false,
                    security: vec![],
                    request_schema: None,
                    response_schemas: HashMap::new(),
                    tags: vec!["users".to_string()],
                },
                LoadedOperation {
                    id: "getUser".to_string(),
                    method: "GET".to_string(),
                    path: "/users/{userId}".to_string(),
                    summary: Some("Get a user by ID".to_string()),
                    deprecated: false,
                    security: vec![],
                    request_schema: None,
                    response_schemas: HashMap::new(),
                    tags: vec!["users".to_string()],
                },
            ],
            schemas: IndexMap::new(),
        }
    }

    #[test]
    fn test_sentinel_creation() {
        let artifact = create_test_artifact();
        let sentinel = Sentinel::with_defaults(artifact);

        assert_eq!(sentinel.service_name(), "test-service");
        assert_eq!(sentinel.version(), "1.0.0");
        assert_eq!(sentinel.operation_count(), 2);
    }

    #[test]
    fn test_sentinel_resolve() {
        let artifact = create_test_artifact();
        let sentinel = Sentinel::with_defaults(artifact);

        let resolution = sentinel.resolve("GET", "/users").unwrap();
        assert_eq!(resolution.operation_id, "listUsers");

        let resolution = sentinel.resolve("GET", "/users/123").unwrap();
        assert_eq!(resolution.operation_id, "getUser");
        assert_eq!(resolution.path_params.get("userId"), Some(&"123".to_string()));
    }

    #[test]
    fn test_sentinel_has_operation() {
        let artifact = create_test_artifact();
        let sentinel = Sentinel::with_defaults(artifact);

        assert!(sentinel.has_operation("GET", "/users"));
        assert!(sentinel.has_operation("GET", "/users/123"));
        assert!(!sentinel.has_operation("POST", "/users"));
        assert!(!sentinel.has_operation("GET", "/nonexistent"));
    }

    #[test]
    fn test_sentinel_methods() {
        let artifact = create_test_artifact();
        let sentinel = Sentinel::with_defaults(artifact);

        let methods = sentinel.methods();
        assert!(methods.contains(&"GET"));
    }

    #[test]
    fn test_sentinel_routes_for_method() {
        let artifact = create_test_artifact();
        let sentinel = Sentinel::with_defaults(artifact);

        let routes = sentinel.routes_for_method("GET");
        assert!(routes.contains(&"/users"));
        assert!(routes.contains(&"/users/{userId}"));
    }

    #[test]
    fn test_sentinel_config() {
        let artifact = create_test_artifact();
        let config = SentinelConfig::development();
        let sentinel = Sentinel::new(artifact, config);

        assert!(sentinel.config().validation.strict_mode);
    }
}
