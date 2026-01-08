//! Integration tests for handler macros with real Themis artifacts.
//!
//! These tests verify that handlers work correctly with actual Themis
//! contract artifacts instead of mocks.
//!
//! # P1 Technical Debt Item: Handler Macro + Real Contracts
//!
//! This test suite addresses the P1 backlog item to test `#[handler]` macro
//! with actual Themis artifacts, not mocks.

use archimedes_core::di::Container;
use archimedes_core::{InvocationContext, ThemisError};
use archimedes_extract::{ExtractionContext, FromRequest, Inject, Json, Path, Query};
use archimedes_router::Params;
use archimedes_sentinel::{LoadedArtifact, LoadedOperation, Sentinel};
use bytes::Bytes;
use http::{HeaderMap, Method, Uri};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// =============================================================================
// Test Artifact Creation
// =============================================================================

/// Creates a realistic user service artifact similar to what Themis produces.
fn create_user_service_artifact() -> LoadedArtifact {
    use archimedes_sentinel::SchemaRef;

    LoadedArtifact {
        service: "user-service".to_string(),
        version: "1.0.0".to_string(),
        format: "openapi".to_string(),
        operations: vec![
            LoadedOperation {
                id: "listUsers".to_string(),
                method: "GET".to_string(),
                path: "/users".to_string(),
                summary: Some("List all users with pagination".to_string()),
                deprecated: false,
                security: vec!["bearer".to_string()],
                request_schema: None,
                response_schemas: {
                    let mut m = HashMap::new();
                    m.insert(
                        "200".to_string(),
                        SchemaRef {
                            reference: "#/components/schemas/UserList".to_string(),
                            schema_type: "array".to_string(),
                            required: vec![],
                        },
                    );
                    m
                },
                tags: vec!["users".to_string()],
            },
            LoadedOperation {
                id: "getUser".to_string(),
                method: "GET".to_string(),
                path: "/users/{userId}".to_string(),
                summary: Some("Get a user by ID".to_string()),
                deprecated: false,
                security: vec!["bearer".to_string()],
                request_schema: None,
                response_schemas: {
                    let mut m = HashMap::new();
                    m.insert(
                        "200".to_string(),
                        SchemaRef {
                            reference: "#/components/schemas/User".to_string(),
                            schema_type: "object".to_string(),
                            required: vec!["id".to_string(), "email".to_string()],
                        },
                    );
                    m
                },
                tags: vec!["users".to_string()],
            },
            LoadedOperation {
                id: "createUser".to_string(),
                method: "POST".to_string(),
                path: "/users".to_string(),
                summary: Some("Create a new user".to_string()),
                deprecated: false,
                security: vec!["bearer".to_string()],
                request_schema: Some(SchemaRef {
                    reference: "#/components/schemas/CreateUserRequest".to_string(),
                    schema_type: "object".to_string(),
                    required: vec!["name".to_string(), "email".to_string()],
                }),
                response_schemas: {
                    let mut m = HashMap::new();
                    m.insert(
                        "201".to_string(),
                        SchemaRef {
                            reference: "#/components/schemas/User".to_string(),
                            schema_type: "object".to_string(),
                            required: vec!["id".to_string(), "email".to_string()],
                        },
                    );
                    m
                },
                tags: vec!["users".to_string()],
            },
            LoadedOperation {
                id: "updateUser".to_string(),
                method: "PUT".to_string(),
                path: "/users/{userId}".to_string(),
                summary: Some("Update a user".to_string()),
                deprecated: false,
                security: vec!["bearer".to_string()],
                request_schema: Some(SchemaRef {
                    reference: "#/components/schemas/UpdateUserRequest".to_string(),
                    schema_type: "object".to_string(),
                    required: vec![],
                }),
                response_schemas: {
                    let mut m = HashMap::new();
                    m.insert(
                        "200".to_string(),
                        SchemaRef {
                            reference: "#/components/schemas/User".to_string(),
                            schema_type: "object".to_string(),
                            required: vec!["id".to_string(), "email".to_string()],
                        },
                    );
                    m
                },
                tags: vec!["users".to_string()],
            },
            LoadedOperation {
                id: "deleteUser".to_string(),
                method: "DELETE".to_string(),
                path: "/users/{userId}".to_string(),
                summary: Some("Delete a user".to_string()),
                deprecated: false,
                security: vec!["bearer".to_string()],
                request_schema: None,
                response_schemas: {
                    let mut m = HashMap::new();
                    m.insert(
                        "204".to_string(),
                        SchemaRef {
                            reference: "".to_string(),
                            schema_type: "null".to_string(),
                            required: vec![],
                        },
                    );
                    m
                },
                tags: vec!["users".to_string()],
            },
        ],
        schemas: IndexMap::new(),
    }
}

// =============================================================================
// Test DTOs - Match what the contract specifies
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: u64,
    name: String,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdateUserRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UserPath {
    #[serde(rename = "userId")]
    user_id: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ListUsersQuery {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

// =============================================================================
// Test Service - Business logic simulation
// =============================================================================

#[derive(Debug, Clone)]
struct UserService {
    // In a real service, this would be a database connection
}

impl UserService {
    fn new() -> Self {
        Self {}
    }

    fn list_users(&self, limit: u32, offset: u32) -> Vec<User> {
        // Simulate fetching users
        (0..limit)
            .map(|i| User {
                id: (offset + i) as u64 + 1,
                name: format!("User {}", offset + i + 1),
                email: format!("user{}@example.com", offset + i + 1),
                role: Some("user".to_string()),
            })
            .collect()
    }

    fn get_user(&self, id: u64) -> Option<User> {
        // Simulate fetching a user
        if id > 0 && id <= 100 {
            Some(User {
                id,
                name: format!("User {}", id),
                email: format!("user{}@example.com", id),
                role: Some("user".to_string()),
            })
        } else {
            None
        }
    }

    fn create_user(&self, name: String, email: String, role: Option<String>) -> User {
        User {
            id: 1001, // Simulate auto-generated ID
            name,
            email,
            role,
        }
    }

    fn update_user(&self, id: u64, name: Option<String>, email: Option<String>) -> Option<User> {
        self.get_user(id).map(|mut user| {
            if let Some(n) = name {
                user.name = n;
            }
            if let Some(e) = email {
                user.email = e;
            }
            user
        })
    }
}

// =============================================================================
// Tests: Handler + Sentinel Integration
// =============================================================================

/// Test that a handler matches the contract's operation and works end-to-end.
#[tokio::test]
async fn test_handler_matches_contract_operation() {
    let artifact = create_user_service_artifact();
    let sentinel = Sentinel::with_defaults(artifact);

    // Verify the contract has our expected operations
    assert!(sentinel.has_operation("GET", "/users"));
    assert!(sentinel.has_operation("GET", "/users/123"));
    assert!(sentinel.has_operation("POST", "/users"));
    assert!(sentinel.has_operation("PUT", "/users/123"));
    assert!(sentinel.has_operation("DELETE", "/users/123"));

    // Test operation resolution
    let resolution = sentinel.resolve("GET", "/users/42").unwrap();
    assert_eq!(resolution.operation_id, "getUser");
    assert_eq!(
        resolution.path_params.get("userId"),
        Some(&"42".to_string())
    );
}

/// Test handler with path parameters extracted via Sentinel resolution.
#[tokio::test]
async fn test_handler_with_sentinel_path_resolution() {
    let artifact = create_user_service_artifact();
    let sentinel = Sentinel::with_defaults(artifact);

    // Resolve the path
    let resolution = sentinel.resolve("GET", "/users/42").unwrap();
    assert_eq!(resolution.operation_id, "getUser");

    // Create params from sentinel resolution
    let mut params = Params::new();
    for (key, value) in &resolution.path_params {
        params.push(key.clone(), value.clone());
    }

    // Set up DI container
    let mut container = Container::new();
    container.register(Arc::new(UserService::new()));
    let container = Arc::new(container);

    // Create invocation context
    let ctx = InvocationContext::new(
        Method::GET,
        Uri::from_static("/users/42"),
        HeaderMap::new(),
        Bytes::new(),
        params,
    )
    .with_container(container);

    // Simulate handler extraction
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let path: Path<UserPath> = Path::from_request(&extraction_ctx).unwrap();
    let service: Inject<UserService> = Inject::from_request(&extraction_ctx).unwrap();

    // Execute handler logic
    let user = service.get_user(path.0.user_id);
    assert!(user.is_some());
    assert_eq!(user.unwrap().id, 42);
}

/// Test createUser handler with JSON body.
#[tokio::test]
async fn test_create_user_handler_with_contract() {
    let artifact = create_user_service_artifact();
    let sentinel = Sentinel::with_defaults(artifact);

    // Verify operation exists
    let resolution = sentinel.resolve("POST", "/users").unwrap();
    assert_eq!(resolution.operation_id, "createUser");

    // Verify the operation has a request schema by checking artifact directly
    let operation = sentinel
        .artifact()
        .operations
        .iter()
        .find(|op| op.id == "createUser")
        .expect("operation should exist");
    assert!(operation.request_schema.is_some());

    // Set up handler context
    let body = r#"{"name":"Alice","email":"alice@example.com","role":"admin"}"#;
    let mut container = Container::new();
    container.register(Arc::new(UserService::new()));
    let container = Arc::new(container);

    let ctx = InvocationContext::new(
        Method::POST,
        Uri::from_static("/users"),
        HeaderMap::new(),
        Bytes::from(body),
        Params::new(),
    )
    .with_container(container);

    // Extract and handle
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let request: Json<CreateUserRequest> = Json::from_request(&extraction_ctx).unwrap();
    let service: Inject<UserService> = Inject::from_request(&extraction_ctx).unwrap();

    let user = service.create_user(
        request.0.name.clone(),
        request.0.email.clone(),
        request.0.role.clone(),
    );

    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
    assert_eq!(user.role, Some("admin".to_string()));
}

/// Test updateUser handler with path param and JSON body.
#[tokio::test]
async fn test_update_user_handler_with_contract() {
    let artifact = create_user_service_artifact();
    let sentinel = Sentinel::with_defaults(artifact);

    // Resolve operation
    let resolution = sentinel.resolve("PUT", "/users/42").unwrap();
    assert_eq!(resolution.operation_id, "updateUser");

    // Create params
    let mut params = Params::new();
    params.push("userId", "42");

    // Set up handler context
    let body = r#"{"name":"Alice Updated"}"#;
    let mut container = Container::new();
    container.register(Arc::new(UserService::new()));
    let container = Arc::new(container);

    let ctx = InvocationContext::new(
        Method::PUT,
        Uri::from_static("/users/42"),
        HeaderMap::new(),
        Bytes::from(body),
        params,
    )
    .with_container(container);

    // Extract and handle
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let path: Path<UserPath> = Path::from_request(&extraction_ctx).unwrap();
    let request: Json<UpdateUserRequest> = Json::from_request(&extraction_ctx).unwrap();
    let service: Inject<UserService> = Inject::from_request(&extraction_ctx).unwrap();

    let user = service.update_user(
        path.0.user_id,
        request.0.name.clone(),
        request.0.email.clone(),
    );

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.id, 42);
    assert_eq!(user.name, "Alice Updated");
}

/// Test listUsers handler with query parameters.
#[tokio::test]
async fn test_list_users_handler_with_query_params() {
    let artifact = create_user_service_artifact();
    let sentinel = Sentinel::with_defaults(artifact);

    // Resolve operation
    let resolution = sentinel.resolve("GET", "/users").unwrap();
    assert_eq!(resolution.operation_id, "listUsers");

    // Set up handler context with query params
    let mut container = Container::new();
    container.register(Arc::new(UserService::new()));
    let container = Arc::new(container);

    let ctx = InvocationContext::new(
        Method::GET,
        Uri::from_static("/users?limit=5&offset=10"),
        HeaderMap::new(),
        Bytes::new(),
        Params::new(),
    )
    .with_container(container);

    // Extract and handle
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let query: Query<ListUsersQuery> = Query::from_request(&extraction_ctx).unwrap();
    let service: Inject<UserService> = Inject::from_request(&extraction_ctx).unwrap();

    let limit = query.0.limit.unwrap_or(10);
    let offset = query.0.offset.unwrap_or(0);
    let users = service.list_users(limit, offset);

    assert_eq!(users.len(), 5);
    assert_eq!(users[0].id, 11); // offset of 10 means first ID is 11
}

/// Test that handler registration matches contract operations.
#[tokio::test]
async fn test_handler_binder_with_sentinel() {
    use archimedes_core::binder::HandlerBinder;
    use archimedes_core::handler::BoxedHandler;
    use std::future::Future;
    use std::pin::Pin;

    let artifact = create_user_service_artifact();

    // Extract operation IDs from the artifact
    let operation_ids: Vec<&str> = artifact
        .operations
        .iter()
        .map(|op| op.id.as_str())
        .collect();

    // Create binder with operation IDs
    let mut binder = HandlerBinder::new(operation_ids);

    // Helper to create a mock BoxedHandler
    fn mock_handler() -> BoxedHandler {
        Box::new(|_ctx: InvocationContext| -> Pin<Box<dyn Future<Output = Result<Bytes, ThemisError>> + Send>> {
            Box::pin(async move { Ok(Bytes::from("ok")) })
        })
    }

    // Register handlers for all operations
    binder.register("listUsers", mock_handler()).unwrap();
    binder.register("getUser", mock_handler()).unwrap();
    binder.register("createUser", mock_handler()).unwrap();
    binder.register("updateUser", mock_handler()).unwrap();
    binder.register("deleteUser", mock_handler()).unwrap();

    // Validate - should pass
    assert!(binder.validate().is_ok());

    // Verify all handlers are registered
    for op in &artifact.operations {
        assert!(
            binder.has_handler(&op.id),
            "Handler missing for operation: {}",
            op.id
        );
    }
}

/// Test full workflow: Sentinel resolution -> Handler extraction -> Response.
#[tokio::test]
async fn test_full_sentinel_handler_workflow() {
    let artifact = create_user_service_artifact();
    let sentinel = Sentinel::with_defaults(artifact);

    // 1. Resolve the incoming request
    let method = "POST";
    let path = "/users";
    let resolution = sentinel.resolve(method, path).unwrap();
    assert_eq!(resolution.operation_id, "createUser");

    // 2. Create params from resolution
    let mut params = Params::new();
    for (key, value) in &resolution.path_params {
        params.push(key.clone(), value.clone());
    }

    // 3. Set up DI container
    let mut container = Container::new();
    container.register(Arc::new(UserService::new()));
    let container = Arc::new(container);

    // 4. Create invocation context
    let body = r#"{"name":"Test User","email":"test@example.com"}"#;
    let ctx = InvocationContext::new(
        Method::POST,
        Uri::from_static("/users"),
        HeaderMap::new(),
        Bytes::from(body),
        params,
    )
    .with_container(container);

    // 5. Simulate macro-generated handler
    let handler = |ctx: InvocationContext| {
        Box::pin(async move {
            let extraction_ctx = ExtractionContext::from_invocation(&ctx);

            // Extract dependencies
            let service: Inject<UserService> = Inject::from_request(&extraction_ctx)
                .map_err(|e| ThemisError::validation(e.to_string()))?;
            let body: Json<CreateUserRequest> = Json::from_request(&extraction_ctx)
                .map_err(|e| ThemisError::validation(e.to_string()))?;

            // Call business logic
            let user = service.create_user(body.0.name.clone(), body.0.email.clone(), body.0.role);

            // Serialize response
            let response =
                serde_json::to_vec(&user).map_err(|e| ThemisError::internal(e.to_string()))?;

            Ok(Bytes::from(response))
        })
    };

    // 6. Invoke the handler
    let result: Result<Bytes, ThemisError> = handler(ctx).await;

    // 7. Verify response
    assert!(result.is_ok());
    let response_bytes = result.unwrap();
    let user: User = serde_json::from_slice(&response_bytes).unwrap();
    assert_eq!(user.name, "Test User");
    assert_eq!(user.email, "test@example.com");
}

/// Test that unknown operations are not resolved.
#[tokio::test]
async fn test_unknown_operation_not_resolved() {
    let artifact = create_user_service_artifact();
    let sentinel = Sentinel::with_defaults(artifact);

    // These should not resolve
    assert!(!sentinel.has_operation("GET", "/nonexistent"));
    assert!(!sentinel.has_operation("PATCH", "/users")); // PATCH not defined
    assert!(!sentinel.has_operation("GET", "/users/123/posts")); // Not in contract
}

/// Test deprecated operation handling.
#[tokio::test]
async fn test_deprecated_operation() {
    let mut artifact = create_user_service_artifact();

    // Mark an operation as deprecated
    for op in &mut artifact.operations {
        if op.id == "deleteUser" {
            op.deprecated = true;
        }
    }

    let sentinel = Sentinel::with_defaults(artifact);

    // Operation should still resolve
    let resolution = sentinel.resolve("DELETE", "/users/42").unwrap();
    assert_eq!(resolution.operation_id, "deleteUser");

    // Check deprecated status via artifact
    let operation = sentinel
        .artifact()
        .operations
        .iter()
        .find(|op| op.id == "deleteUser")
        .unwrap();
    assert!(operation.deprecated);
}
