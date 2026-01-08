//! Authorization middleware stage.
//!
//! This middleware enforces authorization policies for incoming requests.
//! In the mock implementation, it uses simple role-based checks.
//! In production, this will integrate with OPA (Open Policy Agent) via Eunomia.
//!
//! # Pipeline Position
//!
//! Authorization runs after Identity extraction and before Validation:
//!
//! ```text
//! Request → RequestId → Tracing → Identity → [Authorization] → Validation → Handler
//! ```
//!
//! # Mock Implementation
//!
//! The mock authorization middleware supports:
//!
//! - Allow-all mode (for development/testing)
//! - Deny-all mode (for testing rejection flows)
//! - Role-based access control (simple RBAC)
//! - Operation-based permissions
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_middleware::stages::AuthorizationMiddleware;
//!
//! // Allow all requests (development mode)
//! let allow_all = AuthorizationMiddleware::allow_all();
//!
//! // Deny all requests (testing)
//! let deny_all = AuthorizationMiddleware::deny_all();
//!
//! // Role-based access
//! let rbac = AuthorizationMiddleware::rbac()
//!     .allow_role("admin", vec!["*"])
//!     .allow_role("user", vec!["getUser", "listUsers"])
//!     .build();
//! ```

use crate::{
    context::MiddlewareContext,
    middleware::{BoxFuture, Middleware, Next},
    types::{Request, Response, ResponseExt},
};
use archimedes_core::CallerIdentity;
use http::StatusCode;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(feature = "opa")]
use archimedes_authz::Authorizer;

#[cfg(feature = "opa")]
use themis_platform_types::PolicyInput;

#[cfg(feature = "opa")]
use std::collections::HashMap as StdHashMap;

/// Authorization middleware that enforces access control policies.
///
/// This middleware supports multiple authorization modes:
///
/// - **AllowAll**: Allow all requests (development mode)
/// - **DenyAll**: Deny all requests (testing)
/// - **Rbac**: Role-based access control (mock)
/// - **Custom**: Custom policy evaluator trait
/// - **Opa**: OPA policy evaluation via archimedes-authz (requires `opa` feature)
#[derive(Clone)]
pub struct AuthorizationMiddleware {
    /// The authorization mode.
    mode: AuthorizationMode,
}

impl std::fmt::Debug for AuthorizationMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizationMiddleware")
            .field("mode", &self.mode.name())
            .finish()
    }
}

/// Authorization mode configuration.
#[derive(Clone)]
enum AuthorizationMode {
    /// Allow all requests (development mode).
    AllowAll,
    /// Deny all requests (testing).
    DenyAll,
    /// Role-based access control.
    Rbac(Arc<RbacConfig>),
    /// Custom policy function.
    Custom(Arc<dyn PolicyEvaluator>),
    /// OPA policy evaluation (requires `opa` feature).
    #[cfg(feature = "opa")]
    Opa(Arc<Authorizer>),
}

impl AuthorizationMode {
    fn name(&self) -> &'static str {
        match self {
            Self::AllowAll => "allow_all",
            Self::DenyAll => "deny_all",
            Self::Rbac(_) => "rbac",
            Self::Custom(_) => "custom",
            #[cfg(feature = "opa")]
            Self::Opa(_) => "opa",
        }
    }
}

/// Role-based access control configuration.
#[derive(Debug, Default)]
struct RbacConfig {
    /// Maps role names to allowed operation IDs.
    /// Use "*" to allow all operations.
    role_permissions: HashMap<String, HashSet<String>>,
    /// Operations allowed for anonymous users.
    anonymous_operations: HashSet<String>,
    /// Whether to allow anonymous access by default.
    allow_anonymous: bool,
}

/// Custom policy evaluator trait.
pub trait PolicyEvaluator: Send + Sync + std::fmt::Debug {
    /// Evaluates whether the request should be allowed.
    fn evaluate(&self, identity: &CallerIdentity, operation_id: &str) -> PolicyDecision;
}

/// Policy evaluation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Request is allowed.
    Allow,
    /// Request is denied with a reason.
    Deny {
        /// The reason for denial.
        reason: String,
    },
}

impl AuthorizationMiddleware {
    /// Creates a new authorization middleware that allows all requests.
    ///
    /// Use this for development or when authorization is handled elsewhere.
    #[must_use]
    pub fn allow_all() -> Self {
        Self {
            mode: AuthorizationMode::AllowAll,
        }
    }

    /// Creates a new authorization middleware that denies all requests.
    ///
    /// Use this for testing rejection flows.
    #[must_use]
    pub fn deny_all() -> Self {
        Self {
            mode: AuthorizationMode::DenyAll,
        }
    }

    /// Creates a new RBAC authorization middleware builder.
    #[must_use]
    pub fn rbac() -> RbacBuilder {
        RbacBuilder::default()
    }

    /// Creates a new authorization middleware with a custom policy evaluator.
    #[must_use]
    pub fn custom<P: PolicyEvaluator + 'static>(evaluator: P) -> Self {
        Self {
            mode: AuthorizationMode::Custom(Arc::new(evaluator)),
        }
    }

    /// Creates a new authorization middleware using OPA policy evaluation.
    ///
    /// This requires the `opa` feature to be enabled.
    ///
    /// # Arguments
    ///
    /// * `authorizer` - A pre-configured `Authorizer` from `archimedes-authz`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_authz::{Authorizer, EvaluatorConfig};
    /// use archimedes_middleware::stages::AuthorizationMiddleware;
    ///
    /// let authorizer = Authorizer::with_config(EvaluatorConfig::production())?;
    /// let middleware = AuthorizationMiddleware::opa(authorizer);
    /// ```
    #[cfg(feature = "opa")]
    #[must_use]
    pub fn opa(authorizer: Authorizer) -> Self {
        Self {
            mode: AuthorizationMode::Opa(Arc::new(authorizer)),
        }
    }

    /// Creates a new authorization middleware using OPA with default configuration.
    ///
    /// This requires the `opa` feature to be enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the OPA evaluator cannot be initialized.
    #[cfg(feature = "opa")]
    pub fn opa_default() -> Result<Self, archimedes_authz::AuthzError> {
        let authorizer = Authorizer::with_defaults()?;
        Ok(Self::opa(authorizer))
    }

    /// Evaluates authorization for the given identity and operation (sync mock modes).
    fn evaluate(&self, identity: &CallerIdentity, operation_id: &str) -> PolicyDecision {
        match &self.mode {
            AuthorizationMode::AllowAll => PolicyDecision::Allow,
            AuthorizationMode::DenyAll => PolicyDecision::Deny {
                reason: "Authorization denied (deny-all mode)".to_string(),
            },
            AuthorizationMode::Rbac(config) => {
                Self::evaluate_rbac(config, identity, operation_id)
            }
            AuthorizationMode::Custom(evaluator) => evaluator.evaluate(identity, operation_id),
            #[cfg(feature = "opa")]
            AuthorizationMode::Opa(_) => {
                // OPA evaluation is handled in the async middleware process method
                unreachable!("OPA mode should use evaluate_opa_async instead")
            }
        }
    }

    /// Evaluates OPA authorization asynchronously.
    #[cfg(feature = "opa")]
    async fn evaluate_opa_async(
        authorizer: &Authorizer,
        identity: &CallerIdentity,
        operation_id: &str,
        ctx: &MiddlewareContext,
    ) -> Result<themis_platform_types::PolicyDecision, archimedes_authz::AuthzError> {
        // Build PolicyInput from context
        let request_id = ctx.request_id().clone();

        let mut input_builder = PolicyInput::builder()
            .caller(identity.clone())
            .service(ctx.service_name().unwrap_or("unknown"))
            .operation_id(operation_id)
            .method(ctx.method().as_str())
            .path(ctx.path())
            .request_id(request_id);

        // Add headers as context if available
        if let Some(headers) = ctx.headers() {
            let headers_map: StdHashMap<String, String> = headers
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                .collect();
            input_builder = input_builder.headers(headers_map);
        }

        let input = input_builder
            .try_build()
            .map_err(|e| archimedes_authz::AuthzError::Evaluation(format!("Failed to build policy input: {}", e)))?;

        authorizer.authorize(&input).await
    }

    /// Evaluates RBAC policy.
    fn evaluate_rbac(
        config: &RbacConfig,
        identity: &CallerIdentity,
        operation_id: &str,
    ) -> PolicyDecision {
        // Handle anonymous users
        if matches!(identity, CallerIdentity::Anonymous) {
            if config.allow_anonymous || config.anonymous_operations.contains(operation_id) {
                return PolicyDecision::Allow;
            }
            return PolicyDecision::Deny {
                reason: "Anonymous access not permitted".to_string(),
            };
        }

        // Extract roles from identity
        let roles = Self::extract_roles(identity);

        // Check if any role has permission
        for role in &roles {
            if let Some(permissions) = config.role_permissions.get(role) {
                // Wildcard allows all operations
                if permissions.contains("*") {
                    return PolicyDecision::Allow;
                }
                // Check specific operation
                if permissions.contains(operation_id) {
                    return PolicyDecision::Allow;
                }
            }
        }

        PolicyDecision::Deny {
            reason: format!(
                "No permission for operation '{operation_id}' with roles {roles:?}"
            ),
        }
    }

    /// Extracts roles from a caller identity.
    ///
    /// Uses the `CallerIdentityExt` trait from `archimedes-core`.
    fn extract_roles(identity: &CallerIdentity) -> Vec<String> {
        match identity {
            CallerIdentity::Spiffe(s) => {
                // SPIFFE identities get a role based on trust domain
                // Extract trust domain from spiffe://trust-domain/path
                if let Some(rest) = s.spiffe_id.strip_prefix("spiffe://") {
                    if let Some(trust_domain) = rest.split('/').next() {
                        return vec![format!("spiffe:{trust_domain}")];
                    }
                }
                vec![]
            }
            CallerIdentity::User(u) => {
                // Users have explicit roles
                u.roles.clone()
            }
            CallerIdentity::ApiKey(k) => {
                // API keys get a role based on key ID
                vec![format!("api_key:{}", k.key_id)]
            }
            CallerIdentity::Anonymous => {
                // Anonymous has no roles
                vec![]
            }
        }
    }
}

impl Middleware for AuthorizationMiddleware {
    fn name(&self) -> &'static str {
        "authorization"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let operation_id = ctx.operation_id().unwrap_or("unknown").to_string();
            let identity = ctx.identity().clone();

            // Handle OPA mode with async evaluation
            #[cfg(feature = "opa")]
            if let AuthorizationMode::Opa(authorizer) = &self.mode {
                match Self::evaluate_opa_async(authorizer, &identity, &operation_id, ctx).await {
                    Ok(decision) => {
                        if decision.allowed {
                            ctx.set_extension(AuthorizationResult {
                                allowed: true,
                                operation_id,
                                reason: None,
                            });
                            return next.run(ctx, request).await;
                        } else {
                            let reason = decision.reason.unwrap_or_else(|| "access denied".to_string());
                            ctx.set_extension(AuthorizationResult {
                                allowed: false,
                                operation_id,
                                reason: Some(reason.clone()),
                            });
                            return Response::json_error(
                                StatusCode::FORBIDDEN,
                                "AUTHORIZATION_DENIED",
                                &reason,
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "OPA authorization evaluation failed");
                        ctx.set_extension(AuthorizationResult {
                            allowed: false,
                            operation_id,
                            reason: Some(format!("Authorization error: {e}")),
                        });
                        return Response::json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "AUTHORIZATION_ERROR",
                            &format!("Policy evaluation failed: {e}"),
                        );
                    }
                }
            }

            // Evaluate sync authorization policy for non-OPA modes
            let decision = self.evaluate(&identity, &operation_id);

            match decision {
                PolicyDecision::Allow => {
                    // Store decision in context for auditing
                    ctx.set_extension(AuthorizationResult {
                        allowed: true,
                        operation_id,
                        reason: None,
                    });

                    // Continue to next middleware
                    next.run(ctx, request).await
                }
                PolicyDecision::Deny { reason } => {
                    // Store decision in context for auditing
                    ctx.set_extension(AuthorizationResult {
                        allowed: false,
                        operation_id,
                        reason: Some(reason.clone()),
                    });

                    // Return 403 Forbidden response
                    Response::json_error(
                        StatusCode::FORBIDDEN,
                        "AUTHORIZATION_DENIED",
                        &reason,
                    )
                }
            }
        })
    }
}

/// Authorization result stored in context for auditing.
#[derive(Debug, Clone)]
pub struct AuthorizationResult {
    /// Whether the request was allowed.
    pub allowed: bool,
    /// The operation that was evaluated.
    pub operation_id: String,
    /// Denial reason if not allowed.
    pub reason: Option<String>,
}

/// Builder for RBAC authorization middleware.
#[derive(Debug, Default)]
pub struct RbacBuilder {
    config: RbacConfig,
}

impl RbacBuilder {
    /// Allows a role to access specific operations.
    ///
    /// Use `["*"]` to allow all operations.
    #[must_use]
    pub fn allow_role<S, I>(mut self, role: S, operations: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let ops: HashSet<String> = operations.into_iter().map(Into::into).collect();
        self.config.role_permissions.insert(role.into(), ops);
        self
    }

    /// Allows anonymous users to access specific operations.
    #[must_use]
    pub fn allow_anonymous_operations<I>(mut self, operations: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        for op in operations {
            self.config.anonymous_operations.insert(op.into());
        }
        self
    }

    /// Allows anonymous access to all operations.
    #[must_use]
    pub fn allow_anonymous(mut self) -> Self {
        self.config.allow_anonymous = true;
        self
    }

    /// Builds the authorization middleware.
    #[must_use]
    pub fn build(self) -> AuthorizationMiddleware {
        AuthorizationMiddleware {
            mode: AuthorizationMode::Rbac(Arc::new(self.config)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::MiddlewareContext;
    use bytes::Bytes;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;
    use themis_platform_types::identity::UserIdentity;

    fn make_test_request() -> Request {
        HttpRequest::builder()
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn success_response() -> Response {
        HttpResponse::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("success")))
            .unwrap()
    }

    fn create_handler() -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        |_ctx, _req| {
            Box::pin(async {
                success_response()
            })
        }
    }

    #[test]
    fn test_middleware_name() {
        let middleware = AuthorizationMiddleware::allow_all();
        assert_eq!(middleware.name(), "authorization");
    }

    #[tokio::test]
    async fn test_allow_all_permits_any_request() {
        let middleware = AuthorizationMiddleware::allow_all();
        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("testOp".to_string());
        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);

        // Check authorization result in context
        let auth_result = ctx.get_extension::<AuthorizationResult>().unwrap();
        assert!(auth_result.allowed);
    }

    #[tokio::test]
    async fn test_deny_all_rejects_any_request() {
        let middleware = AuthorizationMiddleware::deny_all();
        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("testOp".to_string());
        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Check authorization result in context
        let auth_result = ctx.get_extension::<AuthorizationResult>().unwrap();
        assert!(!auth_result.allowed);
    }

    #[tokio::test]
    async fn test_rbac_allows_role_with_permission() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_role("admin", vec!["getUser", "createUser", "deleteUser"])
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("getUser".to_string());
        ctx.set_identity(CallerIdentity::User(UserIdentity {
            user_id: "user123".to_string(),
            email: Some("admin@example.com".to_string()),
            name: Some("Admin".to_string()),
            roles: vec!["admin".to_string()],
            groups: vec![],
            tenant_id: None,
        }));

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_denies_role_without_permission() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_role("admin", vec!["deleteUser"])
            .allow_role("user", vec!["getUser", "updateUser"])
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("deleteUser".to_string());
        ctx.set_identity(CallerIdentity::User(UserIdentity {
            user_id: "user123".to_string(),
            email: Some("user@example.com".to_string()),
            name: Some("User".to_string()),
            roles: vec!["user".to_string()],
            groups: vec![],
            tenant_id: None,
        }));

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_rbac_wildcard_allows_any_operation() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_role("superadmin", vec!["*"])
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("anyOperation".to_string());
        ctx.set_identity(CallerIdentity::User(UserIdentity {
            user_id: "user123".to_string(),
            email: None,
            name: None,
            roles: vec!["superadmin".to_string()],
            groups: vec![],
            tenant_id: None,
        }));

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_anonymous_denied_by_default() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_role("user", vec!["getUser"])
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("getUser".to_string());
        // Default identity is Anonymous

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_rbac_allows_specific_anonymous_operations() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_anonymous_operations(vec!["health", "ready"])
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("health".to_string());

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_allow_anonymous_permits_all() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_anonymous()
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("anyOperation".to_string());

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_spiffe_identity_role_extraction() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_role("spiffe:example.com", vec!["serviceCall"])
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("serviceCall".to_string());
        ctx.set_identity(CallerIdentity::spiffe("spiffe://example.com/service"));

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_identity_role_extraction() {
        let middleware = AuthorizationMiddleware::rbac()
            .allow_role("api_key:key-12345", vec!["apiCall"])
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("apiCall".to_string());
        ctx.set_identity(CallerIdentity::api_key("key-12345", "Test Key"));

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[derive(Debug)]
    struct MockEvaluator {
        allow: bool,
    }

    impl PolicyEvaluator for MockEvaluator {
        fn evaluate(&self, _identity: &CallerIdentity, _operation_id: &str) -> PolicyDecision {
            if self.allow {
                PolicyDecision::Allow
            } else {
                PolicyDecision::Deny {
                    reason: "Mock evaluator denied".to_string(),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_custom_evaluator_allow() {
        let middleware = AuthorizationMiddleware::custom(MockEvaluator { allow: true });
        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("testOp".to_string());
        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_custom_evaluator_deny() {
        let middleware = AuthorizationMiddleware::custom(MockEvaluator { allow: false });
        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("testOp".to_string());
        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
