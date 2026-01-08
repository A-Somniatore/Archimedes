//! Policy evaluation using OPA/Rego.
//!
//! This module provides the core policy evaluation logic using the `regorus`
//! crate, a pure Rust implementation of OPA.

use std::path::Path;
use std::time::Instant;

use regorus::Engine;
use serde_json::Value;
use themis_platform_types::{PolicyDecision, PolicyInput};
use tracing::{debug, info, instrument, warn};

use crate::bundle::{Bundle, BundleLoader, BundleMetadata};
use crate::config::EvaluatorConfig;
use crate::error::{AuthzError, AuthzResult};

/// OPA/Rego policy evaluator.
///
/// Evaluates authorization policies using the regorus engine.
#[derive(Debug)]
pub struct PolicyEvaluator {
    /// The regorus engine.
    engine: Engine,
    /// Evaluator configuration.
    config: EvaluatorConfig,
    /// Currently loaded bundle metadata.
    bundle_metadata: Option<BundleMetadata>,
}

impl PolicyEvaluator {
    /// Create a new policy evaluator with the given configuration.
    pub fn new(config: EvaluatorConfig) -> AuthzResult<Self> {
        let mut engine = Engine::new();

        // Configure strict mode if enabled
        if config.strict_mode {
            engine.set_strict_builtin_errors(true);
        }

        Ok(Self {
            engine,
            config,
            bundle_metadata: None,
        })
    }

    /// Create an evaluator with default configuration.
    pub fn with_defaults() -> AuthzResult<Self> {
        Self::new(EvaluatorConfig::default())
    }

    /// Load a policy bundle from a file.
    #[instrument(skip(self), fields(path = %path.as_ref().display()))]
    pub async fn load_bundle_from_file(
        &mut self,
        path: impl AsRef<Path>,
    ) -> AuthzResult<BundleMetadata> {
        let bundle = BundleLoader::from_file(path).await?;
        self.load_bundle(bundle)
    }

    /// Load a policy bundle from the registry.
    #[instrument(skip(self))]
    pub async fn load_bundle_from_registry(
        &mut self,
        registry_url: &str,
        service: &str,
        version: &str,
    ) -> AuthzResult<BundleMetadata> {
        let bundle = BundleLoader::from_registry(registry_url, service, version).await?;
        self.load_bundle(bundle)
    }

    /// Load a bundle into the evaluator.
    pub fn load_bundle(&mut self, bundle: Bundle) -> AuthzResult<BundleMetadata> {
        info!(
            revision = %bundle.metadata.revision,
            policies = bundle.policies.len(),
            "loading bundle into evaluator"
        );

        // Create a fresh engine
        let mut engine = Engine::new();
        if self.config.strict_mode {
            engine.set_strict_builtin_errors(true);
        }

        // Load all policies
        for (path, content) in &bundle.policies {
            debug!(path, "adding policy");
            engine
                .add_policy(path.clone(), content.clone())
                .map_err(|e| {
                    AuthzError::Evaluation(format!("failed to load policy {}: {}", path, e))
                })?;
        }

        // Load all data
        for (path, content) in &bundle.data {
            debug!(path, "adding data");
            let regorus_value: regorus::Value = content.clone().into();
            engine.add_data(regorus_value).map_err(|e| {
                AuthzError::Evaluation(format!("failed to load data {}: {}", path, e))
            })?;
        }

        let metadata = bundle.metadata.clone();
        self.engine = engine;
        self.bundle_metadata = Some(metadata.clone());

        Ok(metadata)
    }

    /// Add a single policy from source code.
    pub fn add_policy(&mut self, name: &str, source: &str) -> AuthzResult<()> {
        self.engine
            .add_policy(name.to_string(), source.to_string())
            .map_err(|e| AuthzError::Evaluation(format!("failed to add policy {}: {}", name, e)))?;
        Ok(())
    }

    /// Add data for policy evaluation.
    pub fn add_data(&mut self, data: Value) -> AuthzResult<()> {
        let regorus_value: regorus::Value = data.into();
        self.engine
            .add_data(regorus_value)
            .map_err(|e| AuthzError::Evaluation(format!("failed to add data: {}", e)))?;
        Ok(())
    }

    /// Evaluate a policy decision for the given input.
    #[instrument(skip(self, input), fields(
        service = %input.service,
        operation_id = %input.operation_id,
        method = %input.method
    ))]
    pub fn evaluate(&self, input: &PolicyInput) -> AuthzResult<PolicyDecision> {
        let start = Instant::now();

        // Convert input to JSON for OPA
        let input_json = serde_json::to_value(input)
            .map_err(|e| AuthzError::InvalidInput(format!("failed to serialize input: {}", e)))?;

        // Set input in the engine
        let regorus_input: regorus::Value = input_json.into();

        // Create a mutable clone for evaluation
        let mut engine = self.engine.clone();
        engine.set_input(regorus_input);

        // Evaluate the allow query
        let result = engine
            .eval_query(self.config.allow_query.clone(), false)
            .map_err(|e| AuthzError::Evaluation(format!("query evaluation failed: {}", e)))?;

        let elapsed = start.elapsed();
        let elapsed_ns = elapsed.as_nanos() as u64;

        debug!(
            query = %self.config.allow_query,
            elapsed_ms = elapsed.as_millis(),
            "policy evaluation complete"
        );

        // Parse the result
        let allowed = self.extract_allow_result(&result);

        // Get policy metadata
        let (policy_id, policy_version) = self.bundle_metadata.as_ref().map_or_else(
            || {
                (
                    self.config.default_policy_id.clone(),
                    self.config.default_policy_version.clone(),
                )
            },
            |m| (self.config.default_policy_id.clone(), m.revision.clone()),
        );

        let decision = if allowed {
            PolicyDecision::allow(policy_id, policy_version).with_evaluation_time(elapsed_ns)
        } else {
            // Try to extract a denial reason
            let reason = self.extract_denial_reason(&mut engine);
            PolicyDecision::deny(policy_id, policy_version, reason).with_evaluation_time(elapsed_ns)
        };

        Ok(decision)
    }

    /// Get the currently loaded bundle metadata.
    pub fn bundle_metadata(&self) -> Option<&BundleMetadata> {
        self.bundle_metadata.as_ref()
    }

    /// Check if a policy is loaded.
    pub fn has_policy(&self) -> bool {
        self.bundle_metadata.is_some()
    }

    fn extract_allow_result(&self, result: &regorus::QueryResults) -> bool {
        // Try to extract a boolean from the query results
        for r in &result.result {
            for expr in &r.expressions {
                if let regorus::Value::Bool(b) = &expr.value {
                    return *b;
                }
            }
        }
        // Default to deny if no result
        false
    }

    fn extract_denial_reason(&self, engine: &mut Engine) -> String {
        // Try to evaluate a reason query
        if let Ok(result) = engine.eval_query("data.authz.deny_reason".to_string(), false) {
            for r in &result.result {
                for expr in &r.expressions {
                    if let regorus::Value::String(s) = &expr.value {
                        return s.to_string();
                    }
                }
            }
        }
        "access denied by policy".to_string()
    }
}

impl Clone for PolicyEvaluator {
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            config: self.config.clone(),
            bundle_metadata: self.bundle_metadata.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use themis_platform_types::{CallerIdentity, RequestId};

    fn create_test_input() -> PolicyInput {
        PolicyInput::builder()
            .caller(CallerIdentity::user("user-123", "user@example.com"))
            .service("test-service")
            .operation_id("testOp")
            .method("GET")
            .path("/test")
            .request_id(RequestId::new())
            .try_build()
            .unwrap()
    }

    #[test]
    fn test_evaluator_creation() {
        let evaluator = PolicyEvaluator::with_defaults();
        assert!(evaluator.is_ok());
    }

    #[test]
    fn test_add_policy() {
        let mut evaluator = PolicyEvaluator::with_defaults().unwrap();
        let result = evaluator.add_policy("test.rego", "package authz\nallow = true");
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_allow() {
        let mut evaluator = PolicyEvaluator::with_defaults().unwrap();
        evaluator
            .add_policy("authz.rego", "package authz\nallow = true")
            .unwrap();

        let input = create_test_input();
        let decision = evaluator.evaluate(&input).unwrap();

        assert!(decision.allowed);
        assert!(decision.evaluation_time_ns.is_some());
    }

    #[test]
    fn test_evaluate_deny() {
        let mut evaluator = PolicyEvaluator::with_defaults().unwrap();
        evaluator
            .add_policy("authz.rego", "package authz\nallow = false")
            .unwrap();

        let input = create_test_input();
        let decision = evaluator.evaluate(&input).unwrap();

        assert!(!decision.allowed);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn test_evaluate_with_input() {
        let mut evaluator = PolicyEvaluator::with_defaults().unwrap();
        evaluator
            .add_policy(
                "authz.rego",
                r#"
                package authz
                allow if {
                    input.method == "GET"
                }
                "#,
            )
            .unwrap();

        let input = create_test_input();
        let decision = evaluator.evaluate(&input).unwrap();
        assert!(decision.allowed);
    }

    #[test]
    fn test_has_policy() {
        let evaluator = PolicyEvaluator::with_defaults().unwrap();
        assert!(!evaluator.has_policy());
    }

    #[test]
    fn test_bundle_metadata() {
        let evaluator = PolicyEvaluator::with_defaults().unwrap();
        assert!(evaluator.bundle_metadata().is_none());
    }
}
