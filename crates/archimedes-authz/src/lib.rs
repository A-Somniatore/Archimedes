//! Archimedes Authorization - Eunomia/OPA Policy Integration
//!
//! This crate provides authorization policy evaluation for Archimedes using
//! the Eunomia policy framework and OPA/Rego.
//!
//! # Overview
//!
//! The authorization system integrates with Eunomia policy bundles to:
//! - Load and cache policy bundles from the registry
//! - Evaluate authorization decisions using OPA/Rego
//! - Cache decisions for performance
//! - Provide audit logging for compliance
//!
//! # Architecture
//!
//! ```text
//!                      ┌────────────────────────────┐
//!                      │   Eunomia Registry         │
//!                      └──────────┬─────────────────┘
//!                                 │ pull bundle
//!                      ┌──────────▼─────────────────┐
//!                      │   BundleLoader             │
//!                      │   (fetch + validate)       │
//!                      └──────────┬─────────────────┘
//!                                 │ policies + data
//!                      ┌──────────▼─────────────────┐
//!      PolicyInput     │   PolicyEvaluator          │
//!          │           │   (regorus engine)         │
//!          ▼           └──────────┬─────────────────┘
//!     ┌────────────┐              │ evaluate
//!     │ Archimedes │──────────────▼
//!     │ Middleware │   PolicyDecision (allow/deny)
//!     └────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use archimedes_authz::{PolicyEvaluator, EvaluatorConfig};
//! use themis_platform_types::{CallerIdentity, PolicyInput, RequestId};
//!
//! // Create evaluator with policies
//! let evaluator = PolicyEvaluator::from_bundle("policies.bundle.tar.gz").await?;
//!
//! // Build policy input from request
//! let input = PolicyInput::builder()
//!     .caller(CallerIdentity::user("user-123", "user@example.com"))
//!     .service("users-service")
//!     .operation_id("getUser")
//!     .method("GET")
//!     .path("/users/456")
//!     .request_id(RequestId::new())
//!     .build();
//!
//! // Evaluate authorization
//! let decision = evaluator.evaluate(&input).await?;
//! if !decision.allowed {
//!     return Err(AuthzError::AccessDenied(decision.reason));
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bundle;
pub mod cache;
pub mod config;
pub mod error;
pub mod evaluator;

// Re-exports for convenience
pub use bundle::{Bundle, BundleLoader, BundleMetadata};
pub use cache::{DecisionCache, CacheConfig};
pub use config::EvaluatorConfig;
pub use error::{AuthzError, AuthzResult};
pub use evaluator::PolicyEvaluator;

/// Main authorization service for Archimedes.
///
/// Combines policy evaluation with caching and bundle management.
#[derive(Debug)]
pub struct Authorizer {
    /// Policy evaluator.
    evaluator: PolicyEvaluator,
    /// Decision cache.
    cache: DecisionCache,
    /// Current bundle metadata.
    bundle_metadata: Option<BundleMetadata>,
}

impl Authorizer {
    /// Create a new Authorizer with the given evaluator and cache.
    pub fn new(evaluator: PolicyEvaluator, cache: DecisionCache) -> Self {
        Self {
            evaluator,
            cache,
            bundle_metadata: None,
        }
    }

    /// Create an Authorizer from configuration.
    pub fn with_config(config: EvaluatorConfig) -> AuthzResult<Self> {
        let evaluator = PolicyEvaluator::new(config.clone())?;
        let cache = DecisionCache::new(config.cache_config);
        Ok(Self::new(evaluator, cache))
    }

    /// Create an Authorizer with default configuration.
    ///
    /// This is suitable for development and testing. For production,
    /// use `with_config(EvaluatorConfig::production())`.
    pub fn with_defaults() -> AuthzResult<Self> {
        Self::with_config(EvaluatorConfig::development())
    }

    /// Load a policy bundle from a file.
    pub async fn load_bundle(&mut self, path: impl AsRef<std::path::Path>) -> AuthzResult<()> {
        let metadata = self.evaluator.load_bundle_from_file(path).await?;
        self.bundle_metadata = Some(metadata);
        self.cache.clear();
        Ok(())
    }

    /// Evaluate an authorization request.
    ///
    /// First checks the cache, then evaluates against the loaded policy.
    pub async fn authorize(
        &self,
        input: &themis_platform_types::PolicyInput,
    ) -> AuthzResult<themis_platform_types::PolicyDecision> {
        // Check cache first
        if let Some(decision) = self.cache.get(input) {
            tracing::debug!(
                operation_id = %input.operation_id,
                cached = true,
                "returning cached decision"
            );
            return Ok(decision);
        }

        // Evaluate policy
        let decision = self.evaluator.evaluate(input)?;

        // Cache the decision
        if self.cache.should_cache(&decision) {
            self.cache.insert(input, &decision);
        }

        Ok(decision)
    }

    /// Get the current bundle metadata.
    pub fn bundle_metadata(&self) -> Option<&BundleMetadata> {
        self.bundle_metadata.as_ref()
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> cache::CacheStats {
        self.cache.stats()
    }

    /// Clear the decision cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorizer_creation() {
        let config = EvaluatorConfig::default();
        let result = Authorizer::with_config(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_stats() {
        let config = EvaluatorConfig::default();
        let authorizer = Authorizer::with_config(config).unwrap();
        let stats = authorizer.cache_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }
}
