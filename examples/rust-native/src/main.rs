//! Rust Example Service (Native Archimedes)
//!
//! This service demonstrates how to build a Rust microservice using the Archimedes
//! framework directly. It showcases native handler registration, routing, and
//! the contract-first approach of the Archimedes platform.
//!
//! ## Features Demonstrated
//!
//! - Native Archimedes server setup with `Server::builder()`
//! - Handler registration with `HandlerRegistry`
//! - Route configuration with `router_mut().add_route()`
//! - Typed request/response handlers with `RequestContext`
//! - Shared application state via `Arc`
//!
//! ## API Endpoints
//!
//! - `GET /users` - List all users
//! - `POST /users` - Create a new user
//! - `GET /users/{userId}` - Get a user by ID
//! - `PUT /users/{userId}` - Update a user
//! - `DELETE /users/{userId}` - Delete a user

use archimedes_core::{RequestContext, ThemisError};
use archimedes_server::{HandlerError, HandlerRegistry, Server};
use http::Method;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
};
use tracing::info;
use uuid::Uuid;

// =============================================================================
// Types
// =============================================================================

/// User model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique user identifier.
    pub id: String,
    /// User's display name.
    pub name: String,
    /// User's email address.
    pub email: String,
    /// ISO 8601 timestamp of when the user was created.
    pub created_at: String,
}

/// Request to list users (empty body, uses query params in real impl).
#[derive(Debug, Deserialize, Default)]
pub struct ListUsersRequest {}

/// Request to get a single user.
#[derive(Debug, Deserialize)]
pub struct GetUserRequest {
    /// User ID from path parameter.
    pub user_id: String,
}

/// Request to create a user.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// User's display name.
    pub name: String,
    /// User's email address.
    pub email: String,
}

/// Request to update a user.
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    /// User ID from path parameter.
    pub user_id: String,
    /// Updated name (optional).
    #[serde(default)]
    pub name: Option<String>,
    /// Updated email (optional).
    #[serde(default)]
    pub email: Option<String>,
}

/// Request to delete a user.
#[derive(Debug, Deserialize)]
pub struct DeleteUserRequest {
    /// User ID from path parameter.
    pub user_id: String,
}

/// List users response.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UsersResponse {
    /// List of users.
    pub users: Vec<User>,
    /// Total count of users.
    pub total: usize,
}

/// Delete user response (empty success).
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteUserResponse {
    /// Whether the deletion was successful.
    pub deleted: bool,
}

// =============================================================================
// Application State
// =============================================================================

/// Shared application state containing the user store.
#[derive(Clone)]
pub struct AppState {
    users: Arc<RwLock<HashMap<String, User>>>,
}

impl AppState {
    /// Creates a new `AppState` with seed data.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new empty `AppState` (for testing).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns a reference to the users store.
    #[must_use]
    pub fn users(&self) -> &Arc<RwLock<HashMap<String, User>>> {
        &self.users
    }
}

impl Default for AppState {
    fn default() -> Self {
        let mut users = HashMap::new();
        users.insert(
            "1".to_string(),
            User {
                id: "1".to_string(),
                name: "Alice Smith".to_string(),
                email: "alice@example.com".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
        );
        users.insert(
            "2".to_string(),
            User {
                id: "2".to_string(),
                name: "Bob Johnson".to_string(),
                email: "bob@example.com".to_string(),
                created_at: "2026-01-02T00:00:00Z".to_string(),
            },
        );
        Self {
            users: Arc::new(RwLock::new(users)),
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Returns the current time as an ISO 8601 string.
fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// =============================================================================
// Handlers
// =============================================================================

/// Handler for listing all users.
///
/// # Arguments
///
/// * `ctx` - Request context with identity and tracing info
/// * `_req` - Empty request body
///
/// # Returns
///
/// List of all users in the store.
async fn list_users_handler(
    ctx: RequestContext,
    _req: ListUsersRequest,
    state: Arc<AppState>,
) -> Result<UsersResponse, HandlerError> {
    info!(
        request_id = %ctx.request_id(),
        operation = %ctx.operation_id().unwrap_or("unknown"),
        "Listing users"
    );

    let users = state.users.read().map_err(|e| {
        HandlerError::Custom(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Lock poisoned: {}", e),
        )))
    })?;

    let users_vec: Vec<User> = users.values().cloned().collect();
    let total = users_vec.len();

    Ok(UsersResponse {
        users: users_vec,
        total,
    })
}

/// Handler for getting a user by ID.
///
/// # Arguments
///
/// * `ctx` - Request context
/// * `req` - Request containing user_id
///
/// # Returns
///
/// The user if found, or a not-found error.
async fn get_user_handler(
    ctx: RequestContext,
    req: GetUserRequest,
    state: Arc<AppState>,
) -> Result<User, HandlerError> {
    info!(
        request_id = %ctx.request_id(),
        user_id = %req.user_id,
        "Getting user"
    );

    let users = state.users.read().map_err(|e| {
        HandlerError::Custom(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Lock poisoned: {}", e),
        )))
    })?;

    users
        .get(&req.user_id)
        .cloned()
        .ok_or_else(|| {
            HandlerError::ThemisError(ThemisError::not_found(format!(
                "User with ID '{}' not found",
                req.user_id
            )))
        })
}

/// Handler for creating a new user.
///
/// # Arguments
///
/// * `ctx` - Request context
/// * `req` - Request containing name and email
///
/// # Returns
///
/// The created user with generated ID.
async fn create_user_handler(
    ctx: RequestContext,
    req: CreateUserRequest,
    state: Arc<AppState>,
) -> Result<User, HandlerError> {
    info!(
        request_id = %ctx.request_id(),
        email = %req.email,
        "Creating user"
    );

    let mut users = state.users.write().map_err(|e| {
        HandlerError::Custom(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Lock poisoned: {}", e),
        )))
    })?;

    // Check for duplicate email
    for user in users.values() {
        if user.email == req.email {
            return Err(HandlerError::ThemisError(ThemisError::validation(
                format!("User with email '{}' already exists", req.email),
            )));
        }
    }

    let user = User {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        email: req.email,
        created_at: now_iso(),
    };

    info!(
        request_id = %ctx.request_id(),
        user_id = %user.id,
        "Created user"
    );

    users.insert(user.id.clone(), user.clone());
    Ok(user)
}

/// Handler for updating an existing user.
///
/// # Arguments
///
/// * `ctx` - Request context
/// * `req` - Request containing user_id and optional updates
///
/// # Returns
///
/// The updated user.
async fn update_user_handler(
    ctx: RequestContext,
    req: UpdateUserRequest,
    state: Arc<AppState>,
) -> Result<User, HandlerError> {
    info!(
        request_id = %ctx.request_id(),
        user_id = %req.user_id,
        "Updating user"
    );

    let mut users = state.users.write().map_err(|e| {
        HandlerError::Custom(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Lock poisoned: {}", e),
        )))
    })?;

    let user = users.get_mut(&req.user_id).ok_or_else(|| {
        HandlerError::ThemisError(ThemisError::not_found(format!(
            "User with ID '{}' not found",
            req.user_id
        )))
    })?;

    if let Some(name) = req.name {
        user.name = name;
    }
    if let Some(email) = req.email {
        user.email = email;
    }

    info!(
        request_id = %ctx.request_id(),
        user_id = %req.user_id,
        "Updated user"
    );

    Ok(user.clone())
}

/// Handler for deleting a user.
///
/// # Arguments
///
/// * `ctx` - Request context
/// * `req` - Request containing user_id
///
/// # Returns
///
/// Success response if deleted.
async fn delete_user_handler(
    ctx: RequestContext,
    req: DeleteUserRequest,
    state: Arc<AppState>,
) -> Result<DeleteUserResponse, HandlerError> {
    info!(
        request_id = %ctx.request_id(),
        user_id = %req.user_id,
        "Deleting user"
    );

    let mut users = state.users.write().map_err(|e| {
        HandlerError::Custom(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Lock poisoned: {}", e),
        )))
    })?;

    if users.remove(&req.user_id).is_some() {
        info!(
            request_id = %ctx.request_id(),
            user_id = %req.user_id,
            "Deleted user"
        );
        Ok(DeleteUserResponse { deleted: true })
    } else {
        Err(HandlerError::ThemisError(ThemisError::not_found(format!(
            "User with ID '{}' not found",
            req.user_id
        ))))
    }
}

// =============================================================================
// Handler Registration
// =============================================================================

/// Registers all handlers with the handler registry.
///
/// Each handler is wrapped in a closure that captures the shared state
/// and adapts the 3-argument handler to the 2-argument signature expected
/// by the Archimedes framework.
fn register_handlers(handlers: &mut HandlerRegistry, state: Arc<AppState>) {
    // List users
    let state_clone = Arc::clone(&state);
    handlers.register("listUsers", move |ctx: RequestContext, req: ListUsersRequest| {
        let state = Arc::clone(&state_clone);
        async move { list_users_handler(ctx, req, state).await }
    });

    // Get user
    let state_clone = Arc::clone(&state);
    handlers.register("getUser", move |ctx: RequestContext, req: GetUserRequest| {
        let state = Arc::clone(&state_clone);
        async move { get_user_handler(ctx, req, state).await }
    });

    // Create user
    let state_clone = Arc::clone(&state);
    handlers.register("createUser", move |ctx: RequestContext, req: CreateUserRequest| {
        let state = Arc::clone(&state_clone);
        async move { create_user_handler(ctx, req, state).await }
    });

    // Update user
    let state_clone = Arc::clone(&state);
    handlers.register("updateUser", move |ctx: RequestContext, req: UpdateUserRequest| {
        let state = Arc::clone(&state_clone);
        async move { update_user_handler(ctx, req, state).await }
    });

    // Delete user
    let state_clone = Arc::clone(&state);
    handlers.register("deleteUser", move |ctx: RequestContext, req: DeleteUserRequest| {
        let state = Arc::clone(&state_clone);
        async move { delete_user_handler(ctx, req, state).await }
    });
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    // Configuration from environment
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8001);
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{}:{}", host, port);

    // Application state
    let state = Arc::new(AppState::new());

    // Create handler registry and register handlers
    let mut handlers = HandlerRegistry::new();
    register_handlers(&mut handlers, Arc::clone(&state));

    // Build the Archimedes server
    let mut server = Server::builder()
        .http_addr(&addr)
        .service_name("example-rust-native")
        .service_version(env!("CARGO_PKG_VERSION"))
        .handlers(handlers)
        .build();

    // Configure routes (mapping paths to operation IDs)
    server.router_mut().add_route(Method::GET, "/users", "listUsers");
    server.router_mut().add_route(Method::POST, "/users", "createUser");
    server.router_mut().add_route(Method::GET, "/users/{userId}", "getUser");
    server.router_mut().add_route(Method::PUT, "/users/{userId}", "updateUser");
    server.router_mut().add_route(Method::DELETE, "/users/{userId}", "deleteUser");

    info!("Rust example service (native Archimedes) listening on {}", addr);
    info!("Endpoints:");
    info!("  GET    /health        - Health check (built-in)");
    info!("  GET    /ready         - Readiness check (built-in)");
    info!("  GET    /users         - List all users");
    info!("  POST   /users         - Create a new user");
    info!("  GET    /users/{{id}}    - Get user by ID");
    info!("  PUT    /users/{{id}}    - Update user");
    info!("  DELETE /users/{{id}}    - Delete user");

    // Run the server (blocks until shutdown signal)
    server.run().await?;

    Ok(())
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // State Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_app_state_default_has_seed_data() {
        let state = AppState::default();
        let users = state.users.read().unwrap();

        assert_eq!(users.len(), 2);
        assert!(users.contains_key("1"));
        assert!(users.contains_key("2"));
    }

    #[test]
    fn test_app_state_empty() {
        let state = AppState::empty();
        let users = state.users.read().unwrap();

        assert!(users.is_empty());
    }

    // -------------------------------------------------------------------------
    // Handler Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_users_handler() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = ListUsersRequest {};

        let result = list_users_handler(ctx, req, state).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total, 2);
        assert_eq!(response.users.len(), 2);
    }

    #[tokio::test]
    async fn test_list_users_empty_store() {
        let state = Arc::new(AppState::empty());
        let ctx = RequestContext::new();
        let req = ListUsersRequest {};

        let result = list_users_handler(ctx, req, state).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total, 0);
        assert!(response.users.is_empty());
    }

    #[tokio::test]
    async fn test_get_user_handler_found() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = GetUserRequest {
            user_id: "1".to_string(),
        };

        let result = get_user_handler(ctx, req, state).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id, "1");
        assert_eq!(user.name, "Alice Smith");
    }

    #[tokio::test]
    async fn test_get_user_handler_not_found() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = GetUserRequest {
            user_id: "nonexistent".to_string(),
        };

        let result = get_user_handler(ctx, req, state).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HandlerError::ThemisError(e) => {
                assert!(e.to_string().contains("not found"));
            }
            _ => panic!("Expected ThemisError"),
        }
    }

    #[tokio::test]
    async fn test_create_user_handler_success() {
        let state = Arc::new(AppState::empty());
        let ctx = RequestContext::new();
        let req = CreateUserRequest {
            name: "Charlie Brown".to_string(),
            email: "charlie@example.com".to_string(),
        };

        let result = create_user_handler(ctx, req, state.clone()).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.name, "Charlie Brown");
        assert_eq!(user.email, "charlie@example.com");

        // Verify user was added to store
        let users = state.users.read().unwrap();
        assert!(users.contains_key(&user.id));
    }

    #[tokio::test]
    async fn test_create_user_handler_duplicate_email() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = CreateUserRequest {
            name: "Another Alice".to_string(),
            email: "alice@example.com".to_string(), // Already exists
        };

        let result = create_user_handler(ctx, req, state).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HandlerError::ThemisError(e) => {
                assert!(e.to_string().contains("already exists"));
            }
            _ => panic!("Expected ThemisError"),
        }
    }

    #[tokio::test]
    async fn test_update_user_handler_success() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = UpdateUserRequest {
            user_id: "1".to_string(),
            name: Some("Alice Updated".to_string()),
            email: None,
        };

        let result = update_user_handler(ctx, req, state.clone()).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.name, "Alice Updated");
        assert_eq!(user.email, "alice@example.com"); // Unchanged

        // Verify change persisted
        let users = state.users.read().unwrap();
        assert_eq!(users.get("1").unwrap().name, "Alice Updated");
    }

    #[tokio::test]
    async fn test_update_user_handler_not_found() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = UpdateUserRequest {
            user_id: "nonexistent".to_string(),
            name: Some("Nobody".to_string()),
            email: None,
        };

        let result = update_user_handler(ctx, req, state).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_user_handler_success() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = DeleteUserRequest {
            user_id: "1".to_string(),
        };

        let result = delete_user_handler(ctx, req, state.clone()).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.deleted);

        // Verify user was removed
        let users = state.users.read().unwrap();
        assert!(!users.contains_key("1"));
    }

    #[tokio::test]
    async fn test_delete_user_handler_not_found() {
        let state = Arc::new(AppState::default());
        let ctx = RequestContext::new();
        let req = DeleteUserRequest {
            user_id: "nonexistent".to_string(),
        };

        let result = delete_user_handler(ctx, req, state).await;

        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Handler Registration Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_register_handlers() {
        let state = Arc::new(AppState::default());
        let mut handlers = HandlerRegistry::new();

        register_handlers(&mut handlers, state);

        assert!(handlers.contains("listUsers"));
        assert!(handlers.contains("getUser"));
        assert!(handlers.contains("createUser"));
        assert!(handlers.contains("updateUser"));
        assert!(handlers.contains("deleteUser"));
        assert_eq!(handlers.len(), 5);
    }

    // -------------------------------------------------------------------------
    // Server Builder Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_server_builder() {
        let state = Arc::new(AppState::default());
        let mut handlers = HandlerRegistry::new();
        register_handlers(&mut handlers, state);

        let mut server = Server::builder()
            .http_addr("127.0.0.1:0")
            .service_name("test-service")
            .handlers(handlers)
            .build();

        // Add routes
        server.router_mut().add_route(Method::GET, "/users", "listUsers");
        server.router_mut().add_route(Method::GET, "/users/{userId}", "getUser");

        // Verify routes are registered
        assert!(server.router().has_operation("listUsers"));
        assert!(server.router().has_operation("getUser"));
    }

    // -------------------------------------------------------------------------
    // User Model Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "test-id".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user, deserialized);
    }

    #[test]
    fn test_users_response_serialization() {
        let response = UsersResponse {
            users: vec![User {
                id: "1".to_string(),
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            }],
            total: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"users\""));
    }
}
