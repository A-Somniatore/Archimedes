//! Authorization integration for Python bindings
//!
//! This module provides OPA-based authorization for Python handlers,
//! ensuring parity with the Rust native implementation.
//!
//! ## Overview
//!
//! Authorization is evaluated by the OPA/Rego policy engine:
//! 1. Build `PolicyInput` from the request context
//! 2. Evaluate against loaded policy bundle
//! 3. Return `PolicyDecision` (allow/deny)
//!
//! ## Example
//!
//! ```python
//! from archimedes import PyAuthorizer
//!
//! # Create authorizer with policy bundle
//! authorizer = PyAuthorizer.from_bundle("policies.bundle.tar.gz")
//!
//! # Evaluate authorization in handler
//! @app.handler("getUser")
//! async def get_user(ctx):
//!     decision = authorizer.authorize(ctx)
//!     if not decision.allowed:
//!         return Response.forbidden(decision.reason)
//!     # ... handler logic
//! ```

use std::collections::HashMap;

use archimedes_authz::{Authorizer, EvaluatorConfig};
use pyo3::prelude::*;
use themis_platform_types::identity::{ApiKeyIdentity, SpiffeIdentity, UserIdentity};
use themis_platform_types::{CallerIdentity, PolicyInput, RequestId};
use uuid::Uuid;

use crate::context::PyRequestContext;
use crate::error::ArchimedesError;

/// Python-exposed authorization decision.
///
/// Represents the result of an OPA policy evaluation.
#[pyclass(name = "PolicyDecision")]
#[derive(Debug, Clone)]
pub struct PyPolicyDecision {
    /// Whether access is allowed.
    #[pyo3(get)]
    pub allowed: bool,

    /// Reason for denial (if denied).
    #[pyo3(get)]
    pub reason: Option<String>,

    /// Policy ID that made the decision.
    #[pyo3(get)]
    pub policy_id: String,

    /// Policy version.
    #[pyo3(get)]
    pub policy_version: String,

    /// Evaluation time in nanoseconds.
    #[pyo3(get)]
    pub evaluation_time_ns: Option<u64>,
}

#[pymethods]
impl PyPolicyDecision {
    /// Check if the decision is an allow.
    fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Check if the decision is a deny.
    fn is_denied(&self) -> bool {
        !self.allowed
    }

    fn __repr__(&self) -> String {
        if self.allowed {
            format!(
                "PolicyDecision(allowed=True, policy='{}@{}')",
                self.policy_id, self.policy_version
            )
        } else {
            format!(
                "PolicyDecision(allowed=False, reason={:?}, policy='{}@{}')",
                self.reason, self.policy_id, self.policy_version
            )
        }
    }

    fn __str__(&self) -> String {
        if self.allowed {
            "allowed".to_string()
        } else {
            format!("denied: {}", self.reason.as_deref().unwrap_or("no reason"))
        }
    }
}

/// Python-exposed authorizer.
///
/// Provides OPA policy evaluation for Python handlers.
#[pyclass(name = "Authorizer")]
pub struct PyAuthorizer {
    /// The underlying Rust authorizer.
    authorizer: Authorizer,
    /// Service name for policy input.
    service_name: String,
    /// Environment (production, staging, development).
    environment: String,
}

#[pymethods]
impl PyAuthorizer {
    /// Create a new authorizer with configuration.
    ///
    /// # Arguments
    ///
    /// * `service_name` - The service name for policy input
    /// * `environment` - Environment name (default: "development")
    #[new]
    #[pyo3(signature = (service_name, environment = None))]
    pub fn new(service_name: String, environment: Option<String>) -> PyResult<Self> {
        let config = EvaluatorConfig::development();
        let authorizer = Authorizer::with_config(config)
            .map_err(|e| ArchimedesError::new_err(format!("Failed to create authorizer: {}", e)))?;

        Ok(Self {
            authorizer,
            service_name,
            environment: environment.unwrap_or_else(|| "development".to_string()),
        })
    }

    /// Create an authorizer from a policy bundle file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the policy bundle file (.tar.gz)
    /// * `service_name` - The service name for policy input
    /// * `environment` - Environment name (default: "development")
    #[staticmethod]
    #[pyo3(signature = (path, service_name, environment = None))]
    pub fn from_bundle(
        py: Python<'_>,
        path: String,
        service_name: String,
        environment: Option<String>,
    ) -> PyResult<Self> {
        // Use pyo3_asyncio to run the async bundle load
        let mut authorizer = Self::new(service_name.clone(), environment.clone())?;

        // Load the bundle synchronously by blocking on the async operation
        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                ArchimedesError::new_err(format!("Failed to create runtime: {}", e))
            })?;

            rt.block_on(async {
                authorizer
                    .authorizer
                    .load_bundle(&path)
                    .await
                    .map_err(|e| ArchimedesError::new_err(format!("Failed to load bundle: {}", e)))
            })?;

            Ok::<(), PyErr>(())
        })?;

        Ok(authorizer)
    }

    /// Load a policy bundle from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the policy bundle file
    pub fn load_bundle(&mut self, py: Python<'_>, path: String) -> PyResult<()> {
        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                ArchimedesError::new_err(format!("Failed to create runtime: {}", e))
            })?;

            rt.block_on(async {
                self.authorizer
                    .load_bundle(&path)
                    .await
                    .map_err(|e| ArchimedesError::new_err(format!("Failed to load bundle: {}", e)))
            })?;

            Ok(())
        })
    }

    /// Evaluate authorization for a request.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The request context
    ///
    /// # Returns
    ///
    /// A `PolicyDecision` indicating whether the request is allowed.
    pub fn authorize(&self, py: Python<'_>, ctx: &PyRequestContext) -> PyResult<PyPolicyDecision> {
        // Build CallerIdentity from context
        let caller = self.build_caller_identity(ctx);

        // Parse request ID from trace_id or generate new
        let request_id = Uuid::parse_str(&ctx.trace_id)
            .ok()
            .map(RequestId::from_uuid)
            .unwrap_or_else(RequestId::new);

        // Build PolicyInput
        let input_result = PolicyInput::builder()
            .caller(caller)
            .service(&self.service_name)
            .operation_id(&ctx.operation_id)
            .method(&ctx.method)
            .path(&ctx.path)
            .request_id(request_id)
            .environment(&self.environment)
            .headers(ctx.headers_rs().clone())
            .try_build();

        let input = input_result.map_err(|e| {
            ArchimedesError::new_err(format!("Failed to build policy input: {}", e))
        })?;

        // Evaluate policy
        let decision = py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                ArchimedesError::new_err(format!("Failed to create runtime: {}", e))
            })?;

            rt.block_on(async {
                self.authorizer
                    .authorize(&input)
                    .await
                    .map_err(|e| ArchimedesError::new_err(format!("Authorization failed: {}", e)))
            })
        })?;

        Ok(PyPolicyDecision {
            allowed: decision.allowed,
            reason: decision.reason.clone(),
            policy_id: decision.policy_id.clone(),
            policy_version: decision.policy_version.clone(),
            evaluation_time_ns: decision.evaluation_time_ns,
        })
    }

    /// Evaluate authorization with explicit identity.
    ///
    /// Use this method when you have a specific identity to authorize,
    /// rather than extracting it from the context.
    ///
    /// # Arguments
    ///
    /// * `identity_type` - Type of identity: "user", "spiffe", "api_key"
    /// * `subject` - Subject identifier (user_id, spiffe_id, key_id)
    /// * `operation_id` - Operation being accessed
    /// * `method` - HTTP method
    /// * `path` - Request path
    #[pyo3(signature = (identity_type, subject, operation_id, method, path, roles = None, permissions = None))]
    #[allow(clippy::too_many_arguments)]
    pub fn authorize_explicit(
        &self,
        py: Python<'_>,
        identity_type: String,
        subject: String,
        operation_id: String,
        method: String,
        path: String,
        roles: Option<Vec<String>>,
        permissions: Option<Vec<String>>,
    ) -> PyResult<PyPolicyDecision> {
        // Build CallerIdentity based on type
        let caller = match identity_type.as_str() {
            "user" => {
                // Use explicit struct construction to include roles
                let roles_vec = roles.unwrap_or_default();
                // Note: permissions are mapped to groups for now
                let groups_vec = permissions.unwrap_or_default();
                CallerIdentity::User(UserIdentity {
                    user_id: subject,
                    email: None,
                    name: None,
                    roles: roles_vec,
                    groups: groups_vec,
                    tenant_id: None,
                })
            }
            "spiffe" => CallerIdentity::Spiffe(SpiffeIdentity {
                spiffe_id: subject,
                trust_domain: None,
                service_name: None,
            }),
            "api_key" => CallerIdentity::ApiKey(ApiKeyIdentity {
                key_id: subject,
                name: "".to_string(),
                scopes: roles.unwrap_or_default(), // Use roles as scopes for API keys
                owner_id: None,
            }),
            _ => CallerIdentity::anonymous(),
        };

        // Build PolicyInput
        let input_result = PolicyInput::builder()
            .caller(caller)
            .service(&self.service_name)
            .operation_id(&operation_id)
            .method(&method)
            .path(&path)
            .request_id(RequestId::new())
            .environment(&self.environment)
            .try_build();

        let input = input_result.map_err(|e| {
            ArchimedesError::new_err(format!("Failed to build policy input: {}", e))
        })?;

        // Evaluate policy
        let decision = py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                ArchimedesError::new_err(format!("Failed to create runtime: {}", e))
            })?;

            rt.block_on(async {
                self.authorizer
                    .authorize(&input)
                    .await
                    .map_err(|e| ArchimedesError::new_err(format!("Authorization failed: {}", e)))
            })
        })?;

        Ok(PyPolicyDecision {
            allowed: decision.allowed,
            reason: decision.reason.clone(),
            policy_id: decision.policy_id.clone(),
            policy_version: decision.policy_version.clone(),
            evaluation_time_ns: decision.evaluation_time_ns,
        })
    }

    /// Clear the decision cache.
    pub fn clear_cache(&self) {
        self.authorizer.clear_cache();
    }

    fn __repr__(&self) -> String {
        format!(
            "Authorizer(service='{}', environment='{}')",
            self.service_name, self.environment
        )
    }
}

impl PyAuthorizer {
    /// Build CallerIdentity from PyRequestContext.
    fn build_caller_identity(&self, ctx: &PyRequestContext) -> CallerIdentity {
        if let Some(identity) = ctx.identity_ref() {
            // Try to determine identity type from available data
            let subject = identity.subject_rs();

            // Check if it looks like a SPIFFE ID
            if subject.starts_with("spiffe://") {
                return CallerIdentity::Spiffe(SpiffeIdentity {
                    spiffe_id: subject.to_string(),
                    trust_domain: identity.issuer.clone(),
                    service_name: None,
                });
            }

            // Otherwise treat as user identity
            let roles = identity.roles_rs().to_vec();
            let permissions = identity.permissions_rs().to_vec();

            CallerIdentity::User(UserIdentity {
                user_id: subject.to_string(),
                email: None,
                name: None,
                roles,
                groups: permissions, // Map permissions to groups
                tenant_id: None,
            })
        } else {
            CallerIdentity::anonymous()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_decision_allowed() {
        let decision = PyPolicyDecision {
            allowed: true,
            reason: None,
            policy_id: "test".to_string(),
            policy_version: "1.0.0".to_string(),
            evaluation_time_ns: Some(1000),
        };

        assert!(decision.is_allowed());
        assert!(!decision.is_denied());
        assert_eq!(decision.__str__(), "allowed");
    }

    #[test]
    fn test_policy_decision_denied() {
        let decision = PyPolicyDecision {
            allowed: false,
            reason: Some("insufficient permissions".to_string()),
            policy_id: "test".to_string(),
            policy_version: "1.0.0".to_string(),
            evaluation_time_ns: Some(2000),
        };

        assert!(!decision.is_allowed());
        assert!(decision.is_denied());
        assert_eq!(decision.__str__(), "denied: insufficient permissions");
    }

    #[test]
    fn test_policy_decision_repr() {
        let allowed = PyPolicyDecision {
            allowed: true,
            reason: None,
            policy_id: "authz".to_string(),
            policy_version: "2.0.0".to_string(),
            evaluation_time_ns: None,
        };

        assert!(allowed.__repr__().contains("allowed=True"));
        assert!(allowed.__repr__().contains("authz@2.0.0"));

        let denied = PyPolicyDecision {
            allowed: false,
            reason: Some("access denied".to_string()),
            policy_id: "authz".to_string(),
            policy_version: "2.0.0".to_string(),
            evaluation_time_ns: None,
        };

        assert!(denied.__repr__().contains("allowed=False"));
        assert!(denied.__repr__().contains("access denied"));
    }
}
