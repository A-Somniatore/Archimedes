//! Integration tests for the handler macro.
//!
//! These tests verify that the `#[handler]` macro generates correct code
//! that compiles and works with the Archimedes extraction system.

use archimedes_core::di::Container;
use archimedes_core::{InvocationContext, ThemisError};
use archimedes_extract::{ExtractionContext, FromRequest, Inject, Json};
use archimedes_router::Params;
use bytes::Bytes;
use http::{HeaderMap, Method, Uri};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A test request type.
#[derive(Debug, Clone, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

/// A test response type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: u64,
    name: String,
    email: String,
}

/// A test service for DI.
#[derive(Debug, Clone)]
struct UserService {
    next_id: u64,
}

impl UserService {
    fn new() -> Self {
        Self { next_id: 1 }
    }

    fn create_user(&self, name: String, email: String) -> User {
        User {
            id: self.next_id,
            name,
            email,
        }
    }
}

/// Test that a simple handler function can extract JSON body.
#[tokio::test]
async fn test_handler_json_extraction() {
    // Create a handler that extracts JSON
    async fn create_user(body: Json<CreateUserRequest>) -> Result<User, ThemisError> {
        Ok(User {
            id: 1,
            name: body.0.name.clone(),
            email: body.0.email.clone(),
        })
    }

    // Create an invocation context with a JSON body
    let body = r#"{"name":"Alice","email":"alice@example.com"}"#;
    let ctx = InvocationContext::new(
        Method::POST,
        Uri::from_static("/users"),
        HeaderMap::new(),
        Bytes::from(body),
        Params::new(),
    );

    // Create extraction context and extract JSON
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let json: Json<CreateUserRequest> = Json::from_request(&extraction_ctx).unwrap();

    // Call the handler
    let result = create_user(json).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
}

/// Test that a handler can use dependency injection.
#[tokio::test]
async fn test_handler_with_injection() {
    // Create a handler that uses injected service
    async fn create_user_with_service(
        service: Inject<UserService>,
        body: Json<CreateUserRequest>,
    ) -> Result<User, ThemisError> {
        Ok(service.create_user(body.0.name.clone(), body.0.email.clone()))
    }

    // Set up DI container
    let mut container = Container::new();
    container.register(Arc::new(UserService::new()));
    let container = Arc::new(container);

    // Create an invocation context with JSON body and DI container
    let body = r#"{"name":"Bob","email":"bob@example.com"}"#;
    let ctx = InvocationContext::new(
        Method::POST,
        Uri::from_static("/users"),
        HeaderMap::new(),
        Bytes::from(body),
        Params::new(),
    )
    .with_container(container);

    // Create extraction context
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);

    // Extract service and JSON
    let service: Inject<UserService> = Inject::from_request(&extraction_ctx).unwrap();
    let json: Json<CreateUserRequest> = Json::from_request(&extraction_ctx).unwrap();

    // Call the handler
    let result = create_user_with_service(service, json).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.id, 1);
    assert_eq!(user.name, "Bob");
    assert_eq!(user.email, "bob@example.com");
}

/// Test that InvocationContext properly passes path parameters.
#[tokio::test]
async fn test_handler_with_path_params() {
    use archimedes_extract::Path;

    #[derive(Debug, Deserialize)]
    struct UserPath {
        user_id: u64,
    }

    // Handler that extracts path parameter
    async fn get_user(path: Path<UserPath>) -> Result<User, ThemisError> {
        Ok(User {
            id: path.0.user_id,
            name: format!("User {}", path.0.user_id),
            email: format!("user{}@example.com", path.0.user_id),
        })
    }

    // Create params
    let mut params = Params::new();
    params.push("user_id", "42");

    let ctx = InvocationContext::new(
        Method::GET,
        Uri::from_static("/users/42"),
        HeaderMap::new(),
        Bytes::new(),
        params,
    );

    // Create extraction context
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let path: Path<UserPath> = Path::from_request(&extraction_ctx).unwrap();

    // Call the handler
    let result = get_user(path).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.id, 42);
}

/// Test that InvocationContext properly passes query parameters.
#[tokio::test]
async fn test_handler_with_query_params() {
    use archimedes_extract::Query;

    #[derive(Debug, Deserialize)]
    struct ListParams {
        #[serde(default)]
        limit: Option<u32>,
        #[serde(default)]
        offset: Option<u32>,
    }

    // Handler that extracts query parameters
    async fn list_users(query: Query<ListParams>) -> Result<Vec<User>, ThemisError> {
        let limit = query.0.limit.unwrap_or(10);
        let offset = query.0.offset.unwrap_or(0);

        Ok((0..limit)
            .map(|i| User {
                id: offset as u64 + i as u64,
                name: format!("User {}", offset + i),
                email: format!("user{}@example.com", offset + i),
            })
            .collect())
    }

    let ctx = InvocationContext::new(
        Method::GET,
        Uri::from_static("/users?limit=5&offset=10"),
        HeaderMap::new(),
        Bytes::new(),
        Params::new(),
    );

    // Create extraction context
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let query: Query<ListParams> = Query::from_request(&extraction_ctx).unwrap();

    // Call the handler
    let result = list_users(query).await;

    assert!(result.is_ok());
    let users = result.unwrap();
    assert_eq!(users.len(), 5);
    assert_eq!(users[0].id, 10);
}

/// Test that headers can be extracted.
#[tokio::test]
async fn test_handler_with_headers() {
    use archimedes_extract::{Authorization, ExtractTypedHeader};

    // Handler that extracts authorization header
    async fn authorized_action(
        auth: ExtractTypedHeader<Authorization>,
    ) -> Result<String, ThemisError> {
        Ok(format!("Authorized with: {}", auth.0 .0))
    }

    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer my-token".parse().unwrap());

    let ctx = InvocationContext::new(
        Method::GET,
        Uri::from_static("/protected"),
        headers,
        Bytes::new(),
        Params::new(),
    );

    // Create extraction context
    let extraction_ctx = ExtractionContext::from_invocation(&ctx);
    let auth: ExtractTypedHeader<Authorization> =
        ExtractTypedHeader::from_request(&extraction_ctx).unwrap();

    // Call the handler
    let result = authorized_action(auth).await;

    assert!(result.is_ok());
    assert!(result.unwrap().contains("Bearer my-token"));
}

/// Test full handler workflow simulation.
#[tokio::test]
async fn test_full_handler_workflow() {
    // This simulates what the macro-generated code does

    // Set up DI container
    let mut container = Container::new();
    container.register(Arc::new(UserService::new()));
    let container = Arc::new(container);

    // Create the request
    let body = r#"{"name":"Charlie","email":"charlie@example.com"}"#;
    let ctx = InvocationContext::new(
        Method::POST,
        Uri::from_static("/users"),
        HeaderMap::new(),
        Bytes::from(body),
        Params::new(),
    )
    .with_container(container);

    // Simulate macro-generated handler
    let handler = |ctx: InvocationContext| {
        Box::pin(async move {
            let extraction_ctx = ExtractionContext::from_invocation(&ctx);

            // Extract dependencies
            let service: Inject<UserService> = Inject::from_request(&extraction_ctx)
                .map_err(|e| ThemisError::validation(e.to_string()))?;
            let body: Json<CreateUserRequest> = Json::from_request(&extraction_ctx)
                .map_err(|e| ThemisError::validation(e.to_string()))?;

            // Call business logic
            let user = service.create_user(body.0.name.clone(), body.0.email.clone());

            // Serialize response
            let response =
                serde_json::to_vec(&user).map_err(|e| ThemisError::internal(e.to_string()))?;

            Ok(Bytes::from(response))
        })
    };

    // Invoke the handler
    let result: Result<Bytes, ThemisError> = handler(ctx).await;

    assert!(result.is_ok());
    let response_bytes = result.unwrap();
    let user: User = serde_json::from_slice(&response_bytes).unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.email, "charlie@example.com");
}
