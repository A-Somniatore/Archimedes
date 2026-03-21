//! Integrated Example: Archimedes + Themis + Eunomia
//!
//! This example demonstrates the full integration of:
//! - Archimedes (HTTP server framework)
//! - Themis (Contract validation via Sentinel)
//! - Eunomia (Authorization via OPA/Rego policies)
//!
//! ## Running the Example
//!
//! ```bash
//! cd archimedes/examples/integrated
//! cargo run
//! ```
//!
//! ## Testing the Integration
//!
//! Identity is extracted from HTTP headers using the server's middleware integration.
//! Supported headers (checked in order):
//! - `Authorization: Bearer <jwt>` - JWT token with user claims
//! - `X-Caller-Identity: {...}` - JSON-encoded identity object
//! - `X-User-Id` + `X-User-Roles` - Simple identity headers
//!
//! Health check (no auth required):
//! ```bash
//! curl http://localhost:8080/health
//! ```
//!
//! List users as admin (should succeed):
//! ```bash
//! curl -H "X-User-Id: admin-1" -H "X-User-Roles: admin" \
//!      http://localhost:8080/users
//! ```
//!
//! Create user as admin (should succeed):
//! ```bash
//! curl -X POST -H "Content-Type: application/json" \
//!      -H "X-User-Id: admin-1" -H "X-User-Roles: admin" \
//!      -d '{"name":"New User","email":"new@example.com"}' \
//!      http://localhost:8080/users
//! ```
//!
//! Create user as regular user (should fail - insufficient permissions):
//! ```bash
//! curl -X POST -H "Content-Type: application/json" \
//!      -H "X-User-Id: user-1" -H "X-User-Roles: user" \
//!      -d '{"name":"New User","email":"new@example.com"}' \
//!      http://localhost:8080/users
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use archimedes_authz::{EvaluatorConfig, PolicyEvaluator};
use archimedes_core::{RequestContext, ThemisError};
use archimedes_sentinel::{ArtifactLoader, Sentinel, SentinelConfig};
use archimedes_server::{HandlerError, HandlerRegistry, MiddlewareConfig, Server};
use http::Method;
use serde::{Deserialize, Serialize};
use themis_platform_types::{CallerIdentity, PolicyInput, RequestId};
use tracing::{info, warn};
use uuid::Uuid;

/// User data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
}

/// Create user request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "user".to_string()
}

/// Update user request (includes user_id from path params)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

/// Get/Delete user request (user_id from path params)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserIdRequest {
    pub user_id: String,
}

/// Empty request for list operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyRequest {}

/// In-memory user store
type UserStore = Arc<RwLock<HashMap<String, User>>>;

/// Integrated context containing Themis and Eunomia components
#[derive(Clone)]
pub struct IntegrationContext {
    /// Themis Sentinel for contract validation
    pub sentinel: Arc<Sentinel>,
    /// Eunomia Policy Evaluator for authorization
    pub policy_evaluator: Arc<std::sync::RwLock<PolicyEvaluator>>,
    /// User data store
    pub users: UserStore,
}

impl IntegrationContext {
    /// Get caller identity from the request context.
    /// 
    /// Identity is automatically extracted from HTTP headers by the server's
    /// middleware integration. Supported headers (checked in order):
    /// - `Authorization: Bearer <jwt>` - JWT token with user claims
    /// - `X-Caller-Identity: {...}` - JSON-encoded identity from sidecar/proxy
    /// - `X-User-Id` + `X-User-Roles` - Simple identity headers
    /// 
    /// If no identity headers are present, returns Anonymous.
    pub fn get_identity(&self, ctx: &RequestContext) -> CallerIdentity {
        ctx.identity().clone()
    }
    
    /// Validate request against Themis contract
    pub fn validate_request(
        &self,
        operation_id: &str,
        body: &serde_json::Value,
    ) -> Result<(), ThemisError> {
        match self.sentinel.validate_request(operation_id, body) {
            Ok(result) if result.valid => Ok(()),
            Ok(result) => {
                let errors: Vec<String> = result.errors.iter()
                    .map(|e| format!("{}: {}", e.path, e.message))
                    .collect();
                Err(ThemisError::validation(format!(
                    "Request validation failed: {}",
                    errors.join(", ")
                )))
            }
            Err(e) => Err(ThemisError::validation(format!(
                "Validation error: {}",
                e
            ))),
        }
    }
    
    /// Authorize request using Eunomia policies
    pub fn authorize(
        &self,
        identity: &CallerIdentity,
        operation_id: &str,
        method: &str,
        path: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> Result<(), ThemisError> {
        let input = PolicyInput::builder()
            .caller(identity.clone())
            .service("users-api")
            .operation_id(operation_id)
            .method(method)
            .path(path)
            .request_id(RequestId::new())
            .context(context)
            .try_build()
            .map_err(|e| ThemisError::internal(format!(
                "Failed to build policy input: {}",
                e
            )))?;
        
        let evaluator = self.policy_evaluator.read().unwrap();
        match evaluator.evaluate(&input) {
            Ok(decision) if decision.allowed => {
                info!(
                    operation_id = operation_id,
                    policy_id = decision.policy_id,
                    "authorization granted"
                );
                Ok(())
            }
            Ok(decision) => {
                warn!(
                    operation_id = operation_id,
                    reason = ?decision.reason,
                    "authorization denied"
                );
                Err(ThemisError::authorization(
                    decision.reason.unwrap_or_else(|| "Access denied".to_string())
                ))
            }
            Err(e) => {
                warn!(error = %e, "authorization evaluation error");
                Err(ThemisError::internal(format!(
                    "Authorization error: {}",
                    e
                )))
            }
        }
    }
}

/// Initialize the integration context
async fn create_integration_context() -> Result<IntegrationContext, Box<dyn std::error::Error>> {
    info!("Loading Themis contract artifact...");
    
    // Load Themis artifact
    let artifact_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("contracts")
        .join("users-api.artifact.json");
    
    let artifact = ArtifactLoader::from_file(&artifact_path).await
        .map_err(|e| format!("Failed to load artifact: {}", e))?;
    
    info!(
        service = artifact.service,
        version = artifact.version,
        operations = artifact.operations.len(),
        "Themis artifact loaded"
    );
    
    let sentinel = Sentinel::new(artifact, SentinelConfig::default());
    
    info!("Loading Eunomia authorization policies...");
    
    // Create policy evaluator and load policies
    let mut evaluator = PolicyEvaluator::new(EvaluatorConfig {
        allow_query: "data.authz.users.allow".to_string(),
        strict_mode: true,
        ..EvaluatorConfig::default()
    })?;
    
    // Load the Rego policy directly
    let policy_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("policies")
        .join("authz.rego");
    
    let policy_content = tokio::fs::read_to_string(&policy_path).await
        .map_err(|e| format!("Failed to read policy file: {}", e))?;
    
    evaluator.add_policy("authz.rego", &policy_content)?;
    
    info!("Eunomia policies loaded");
    
    // Create sample users
    let mut users = HashMap::new();
    users.insert("1".to_string(), User {
        id: "1".to_string(),
        name: "Alice Admin".to_string(),
        email: "alice@example.com".to_string(),
        role: "admin".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    });
    users.insert("2".to_string(), User {
        id: "2".to_string(),
        name: "Bob User".to_string(),
        email: "bob@example.com".to_string(),
        role: "user".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    });
    
    Ok(IntegrationContext {
        sentinel: Arc::new(sentinel),
        policy_evaluator: Arc::new(std::sync::RwLock::new(evaluator)),
        users: Arc::new(RwLock::new(users)),
    })
}

// ============================================================================
// Request Handlers
// ============================================================================

/// List users handler
async fn list_users(
    ctx: RequestContext,
    integration: IntegrationContext,
    _req: EmptyRequest,
) -> Result<Vec<User>, HandlerError> {
    let identity = integration.get_identity(&ctx);
    
    // Authorize
    integration.authorize(&identity, "listUsers", "GET", "/users", HashMap::new())?;
    
    // Get users
    let users = integration.users.read().unwrap();
    let users_list: Vec<User> = users.values().cloned().collect();
    
    Ok(users_list)
}

/// Get user by ID handler
async fn get_user(
    ctx: RequestContext,
    integration: IntegrationContext,
    req: UserIdRequest,
) -> Result<User, HandlerError> {
    let identity = integration.get_identity(&ctx);
    
    // Add user_id to context for self-access check
    let mut authz_context = HashMap::new();
    authz_context.insert("user_id".to_string(), serde_json::json!(&req.user_id));
    
    // Authorize
    integration.authorize(
        &identity, 
        "getUser", 
        "GET", 
        &format!("/users/{}", req.user_id),
        authz_context,
    )?;
    
    // Get user
    let users = integration.users.read().unwrap();
    let user = users.get(&req.user_id).cloned().ok_or_else(|| {
        HandlerError::from(ThemisError::not_found(format!("User {} not found", req.user_id)))
    })?;
    
    Ok(user)
}

/// Create user handler
async fn create_user(
    ctx: RequestContext,
    integration: IntegrationContext,
    req: CreateUserRequest,
) -> Result<User, HandlerError> {
    let identity = integration.get_identity(&ctx);
    
    // Authorize (only admins can create users)
    integration.authorize(&identity, "createUser", "POST", "/users", HashMap::new())?;
    
    // Validate request
    let body_json = serde_json::to_value(&req)
        .map_err(|e| HandlerError::from(ThemisError::internal(e.to_string())))?;
    integration.validate_request("createUser", &body_json)?;
    
    // Create user
    let user = User {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        email: req.email,
        role: req.role,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    
    let mut users = integration.users.write().unwrap();
    users.insert(user.id.clone(), user.clone());
    
    info!(user_id = user.id, "user created");
    
    Ok(user)
}

/// Update user handler
async fn update_user(
    ctx: RequestContext,
    integration: IntegrationContext,
    req: UpdateUserRequest,
) -> Result<User, HandlerError> {
    let identity = integration.get_identity(&ctx);
    
    // Add user_id to context for self-access check
    let mut authz_context = HashMap::new();
    authz_context.insert("user_id".to_string(), serde_json::json!(&req.user_id));
    
    // Authorize
    integration.authorize(
        &identity,
        "updateUser",
        "PUT",
        &format!("/users/{}", req.user_id),
        authz_context,
    )?;
    
    // Validate request
    let body_json = serde_json::to_value(&req)
        .map_err(|e| HandlerError::from(ThemisError::internal(e.to_string())))?;
    integration.validate_request("updateUser", &body_json)?;
    
    // Apply update
    let mut users = integration.users.write().unwrap();
    let user = users.get_mut(&req.user_id).ok_or_else(|| {
        HandlerError::from(ThemisError::not_found(format!("User {} not found", req.user_id)))
    })?;
    
    if let Some(name) = req.name {
        user.name = name;
    }
    if let Some(email) = req.email {
        user.email = email;
    }
    if let Some(role) = req.role {
        user.role = role;
    }
    
    info!(user_id = req.user_id, "user updated");
    
    Ok(user.clone())
}

/// Delete user handler
async fn delete_user(
    ctx: RequestContext,
    integration: IntegrationContext,
    req: UserIdRequest,
) -> Result<serde_json::Value, HandlerError> {
    let identity = integration.get_identity(&ctx);
    
    // Authorize (only admins can delete)
    integration.authorize(
        &identity,
        "deleteUser",
        "DELETE",
        &format!("/users/{}", req.user_id),
        HashMap::new(),
    )?;
    
    // Delete user
    let mut users = integration.users.write().unwrap();
    if users.remove(&req.user_id).is_none() {
        return Err(HandlerError::from(ThemisError::not_found(format!(
            "User {} not found",
            req.user_id
        ))));
    }
    
    info!(user_id = req.user_id, "user deleted");
    
    Ok(serde_json::json!(null))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,integrated_example=debug".into()),
        )
        .init();
    
    info!("Starting Integrated Example Server");
    info!("{}", "=".repeat(50));
    info!("This example demonstrates:");
    info!("  - Archimedes: HTTP server framework");
    info!("  - Themis:     Contract validation (Sentinel)");
    info!("  - Eunomia:    Authorization (OPA/Rego policies)");
    info!("{}", "=".repeat(50));
    
    // Create integration context
    let integration = create_integration_context().await?;
    
    // Create handler registry
    let mut registry = HandlerRegistry::new();
    
    // Register handlers - we need to use closures to capture the integration context
    {
        let integration = integration.clone();
        registry.register("listUsers", move |ctx: RequestContext, req: EmptyRequest| {
            let integration = integration.clone();
            async move { list_users(ctx, integration, req).await }
        });
    }
    
    {
        let integration = integration.clone();
        registry.register("getUser", move |ctx: RequestContext, req: UserIdRequest| {
            let integration = integration.clone();
            async move { get_user(ctx, integration, req).await }
        });
    }
    
    {
        let integration = integration.clone();
        registry.register("createUser", move |ctx: RequestContext, req: CreateUserRequest| {
            let integration = integration.clone();
            async move { create_user(ctx, integration, req).await }
        });
    }
    
    {
        let integration = integration.clone();
        registry.register("updateUser", move |ctx: RequestContext, req: UpdateUserRequest| {
            let integration = integration.clone();
            async move { update_user(ctx, integration, req).await }
        });
    }
    
    {
        let integration = integration.clone();
        registry.register("deleteUser", move |ctx: RequestContext, req: UserIdRequest| {
            let integration = integration.clone();
            async move { delete_user(ctx, integration, req).await }
        });
    }
    
    // Build server with middleware for identity extraction from headers
    let mut server = Server::builder()
        .http_addr("127.0.0.1:8080")
        .handlers(registry)
        .middleware(
            MiddlewareConfig::builder()
                .enable_identity()           // Extract identity from HTTP headers
                .service_name("users-api")
                .build()
        )
        .build();
    
    // Add routes from contract
    {
        let router = server.router_mut();
        router.add_route(Method::GET, "/users", "listUsers");
        router.add_route(Method::POST, "/users", "createUser");
        router.add_route(Method::GET, "/users/{userId}", "getUser");
        router.add_route(Method::PUT, "/users/{userId}", "updateUser");
        router.add_route(Method::DELETE, "/users/{userId}", "deleteUser");
    }
    
    info!("");
    info!("Server starting at http://127.0.0.1:8080");
    info!("");
    info!("Identity is extracted from HTTP headers automatically.");
    info!("Supported: X-User-Id + X-User-Roles, X-Caller-Identity, Authorization Bearer");
    info!("");
    info!("Try these commands:");
    info!("  # Health check");
    info!("  curl http://localhost:8080/health");
    info!("");
    info!("  # List users (as admin) - will succeed");
    info!("  curl -H 'X-User-Id: admin-1' -H 'X-User-Roles: admin' \\");
    info!("       http://localhost:8080/users");
    info!("");
    info!("  # Create user (as admin) - will succeed");
    info!("  curl -X POST -H 'Content-Type: application/json' \\");
    info!("       -H 'X-User-Id: admin-1' -H 'X-User-Roles: admin' \\");
    info!("       -d '{{\"name\":\"Test\",\"email\":\"test@example.com\"}}' \\");
    info!("       http://localhost:8080/users");
    info!("");
    info!("  # Create user (as regular user) - will fail (forbidden)");
    info!("  curl -X POST -H 'Content-Type: application/json' \\");
    info!("       -H 'X-User-Id: user-1' -H 'X-User-Roles: user' \\");
    info!("       -d '{{\"name\":\"Test\",\"email\":\"test@example.com\"}}' \\");
    info!("       http://localhost:8080/users");
    info!("");
    server.run().await?;
    
    Ok(())
}
