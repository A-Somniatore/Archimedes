//! Authorization via OPA (Open Policy Agent).

use crate::context::Identity;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Authorization decision from OPA.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Whether the request is allowed
    pub allowed: bool,

    /// Reason for the decision
    pub reason: Option<String>,

    /// Additional data from the policy
    pub data: Option<HashMap<String, serde_json::Value>>,
}

impl Default for PolicyDecision {
    fn default() -> Self {
        Self {
            allowed: false,
            reason: Some("No policy evaluated".to_string()),
            data: None,
        }
    }
}

/// Authorization input for OPA evaluation.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzInput {
    /// Request ID
    pub request_id: Option<String>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Operation ID from contract
    pub operation_id: Option<String>,
    /// Caller identity
    pub identity: Option<Identity>,
}

/// OPA-based authorizer.
///
/// Evaluates authorization policies against the Open Policy Agent.
///
/// ## Example
///
/// ```typescript
/// const authorizer = new Authorizer('http://localhost:8181');
/// authorizer.initSync();
///
/// const input = { method: 'GET', path: '/users', identity: { subject: 'user1' } };
/// const decision = await authorizer.evaluate(input);
/// if (!decision.allowed) {
///   return Response.forbidden({ error: decision.reason });
/// }
/// ```
#[napi]
#[derive(Debug)]
pub struct Authorizer {
    /// OPA endpoint URL
    endpoint: String,

    /// Policy path in OPA
    policy_path: String,

    /// Whether the authorizer is initialized
    initialized: Arc<AtomicBool>,
}

impl Clone for Authorizer {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            policy_path: self.policy_path.clone(),
            initialized: Arc::new(AtomicBool::new(self.initialized.load(Ordering::SeqCst))),
        }
    }
}

#[napi]
impl Authorizer {
    /// Create a new Authorizer with OPA endpoint.
    #[napi(constructor)]
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            policy_path: "archimedes/allow".to_string(),
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create an Authorizer with custom policy path.
    #[napi(factory)]
    pub fn with_policy(endpoint: String, policy_path: String) -> Self {
        Self {
            endpoint,
            policy_path,
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize the authorizer synchronously.
    #[napi]
    pub fn init_sync(&self) {
        self.initialized.store(true, Ordering::SeqCst);
    }

    /// Check if the authorizer is initialized.
    #[napi(getter)]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Get the OPA endpoint.
    #[napi(getter)]
    pub fn endpoint(&self) -> String {
        self.endpoint.clone()
    }

    /// Get the policy path.
    #[napi(getter)]
    pub fn policy_path(&self) -> String {
        self.policy_path.clone()
    }

    /// Evaluate an authorization decision.
    #[napi]
    pub async fn evaluate(&self, input: AuthzInput) -> napi::Result<PolicyDecision> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Authorizer not initialized. Call initSync() first.",
            ));
        }

        // Build OPA input
        let opa_input = self.build_opa_input(&input);

        // In real implementation, would call OPA REST API
        // For now, return a mock decision based on identity
        Ok(self.mock_evaluate(&opa_input))
    }

    /// Evaluate authorization synchronously.
    #[napi]
    pub fn evaluate_sync(&self, input: AuthzInput) -> napi::Result<PolicyDecision> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Authorizer not initialized. Call initSync() first.",
            ));
        }

        let opa_input = self.build_opa_input(&input);
        Ok(self.mock_evaluate(&opa_input))
    }

    /// Build OPA input from authz input.
    fn build_opa_input(&self, input: &AuthzInput) -> HashMap<String, serde_json::Value> {
        let mut opa_input = HashMap::new();

        if let Some(req_id) = &input.request_id {
            opa_input.insert(
                "request_id".to_string(),
                serde_json::Value::String(req_id.clone()),
            );
        }

        opa_input.insert(
            "method".to_string(),
            serde_json::Value::String(input.method.clone()),
        );
        opa_input.insert(
            "path".to_string(),
            serde_json::Value::String(input.path.clone()),
        );

        if let Some(op_id) = &input.operation_id {
            opa_input.insert(
                "operation_id".to_string(),
                serde_json::Value::String(op_id.clone()),
            );
        }

        if let Some(identity) = &input.identity {
            let mut id_map = serde_json::Map::new();

            if let Some(sub) = &identity.subject {
                id_map.insert(
                    "subject".to_string(),
                    serde_json::Value::String(sub.clone()),
                );
            }
            if let Some(roles) = &identity.roles {
                id_map.insert(
                    "roles".to_string(),
                    serde_json::Value::Array(
                        roles
                            .iter()
                            .map(|r| serde_json::Value::String(r.clone()))
                            .collect(),
                    ),
                );
            }
            if let Some(scopes) = &identity.scopes {
                id_map.insert(
                    "scopes".to_string(),
                    serde_json::Value::Array(
                        scopes
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }

            opa_input.insert("identity".to_string(), serde_json::Value::Object(id_map));
        }

        opa_input
    }

    /// Mock evaluation for testing (would be replaced with actual OPA call).
    fn mock_evaluate(&self, input: &HashMap<String, serde_json::Value>) -> PolicyDecision {
        // Default allow if identity exists with a subject
        let has_identity = input
            .get("identity")
            .and_then(|i| i.get("subject"))
            .is_some_and(|s| !s.as_str().unwrap_or("").is_empty());

        if has_identity {
            PolicyDecision {
                allowed: true,
                reason: Some("Identity verified".to_string()),
                data: None,
            }
        } else {
            PolicyDecision {
                allowed: false,
                reason: Some("No identity provided".to_string()),
                data: None,
            }
        }
    }
}

/// Create a mock authorizer that always allows requests (for testing).
#[napi]
pub fn allow_all_authorizer() -> Authorizer {
    let auth = Authorizer::new("mock://allow-all".to_string());
    auth.init_sync();
    auth
}

/// Create a mock authorizer that always denies requests (for testing).
#[napi]
pub fn deny_all_authorizer() -> Authorizer {
    let auth = Authorizer::new("mock://deny-all".to_string());
    auth.init_sync();
    auth
}

/// Build an authorization input for OPA from components.
#[napi]
pub fn build_authz_input(
    method: String,
    path: String,
    operation_id: Option<String>,
    identity: Option<Identity>,
) -> AuthzInput {
    AuthzInput {
        request_id: None,
        method,
        path,
        operation_id,
        identity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::IdentityBuilder;

    #[test]
    fn test_authorizer_creation() {
        let auth = Authorizer::new("http://localhost:8181".to_string());
        assert_eq!(auth.endpoint(), "http://localhost:8181");
        assert_eq!(auth.policy_path(), "archimedes/allow");
        assert!(!auth.is_initialized());
    }

    #[test]
    fn test_authorizer_with_policy() {
        let auth = Authorizer::with_policy(
            "http://localhost:8181".to_string(),
            "custom/policy".to_string(),
        );
        assert_eq!(auth.policy_path(), "custom/policy");
    }

    #[test]
    fn test_authorizer_init_sync() {
        let auth = Authorizer::new("http://localhost:8181".to_string());
        assert!(!auth.is_initialized());

        auth.init_sync();
        assert!(auth.is_initialized());
    }

    #[tokio::test]
    async fn test_evaluate_with_identity() {
        let auth = Authorizer::new("http://localhost:8181".to_string());
        auth.init_sync();

        let mut builder = IdentityBuilder::new();
        builder.subject("user123".to_string());
        builder.role("admin".to_string());
        let identity = builder.build();

        let input = AuthzInput {
            request_id: None,
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: None,
            identity: Some(identity),
        };

        let decision = auth.evaluate(input).await.unwrap();
        assert!(decision.allowed);
    }

    #[tokio::test]
    async fn test_evaluate_without_identity() {
        let auth = Authorizer::new("http://localhost:8181".to_string());
        auth.init_sync();

        let input = AuthzInput {
            request_id: None,
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: None,
            identity: None,
        };

        let decision = auth.evaluate(input).await.unwrap();
        assert!(!decision.allowed);
        assert!(decision.reason.unwrap().contains("No identity"));
    }

    #[tokio::test]
    async fn test_evaluate_not_initialized() {
        let auth = Authorizer::new("http://localhost:8181".to_string());

        let input = AuthzInput {
            request_id: None,
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: None,
            identity: None,
        };

        let result = auth.evaluate(input).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_sync() {
        let auth = Authorizer::new("http://localhost:8181".to_string());
        auth.init_sync();

        let mut builder = IdentityBuilder::new();
        builder.subject("user123".to_string());
        let identity = builder.build();

        let input = AuthzInput {
            request_id: None,
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: None,
            identity: Some(identity),
        };

        let decision = auth.evaluate_sync(input).unwrap();
        assert!(decision.allowed);
    }

    #[test]
    fn test_allow_all_authorizer() {
        let auth = allow_all_authorizer();
        assert!(auth.is_initialized());
    }

    #[test]
    fn test_deny_all_authorizer() {
        let auth = deny_all_authorizer();
        assert!(auth.is_initialized());
    }

    #[test]
    fn test_build_authz_input() {
        let mut builder = IdentityBuilder::new();
        builder.subject("user123".to_string());
        let identity = builder.build();

        let input = build_authz_input(
            "GET".to_string(),
            "/users".to_string(),
            Some("listUsers".to_string()),
            Some(identity),
        );

        assert_eq!(input.method, "GET");
        assert_eq!(input.path, "/users");
        assert_eq!(input.operation_id, Some("listUsers".to_string()));
    }

    #[test]
    fn test_policy_decision_default() {
        let decision = PolicyDecision::default();
        assert!(!decision.allowed);
        assert!(decision.reason.is_some());
    }
}
