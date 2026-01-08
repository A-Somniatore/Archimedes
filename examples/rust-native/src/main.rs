//! Rust Example Service (Native Archimedes)
//!
//! This service demonstrates how to build a Rust microservice using the Archimedes
//! framework directly. In production, you would use the full Archimedes framework
//! for contract validation, authorization, and observability. This example uses
//! axum directly to show the basic patterns.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tracing::{info, warn};
use uuid::Uuid;

// =============================================================================
// Types
// =============================================================================

/// Caller identity from X-Caller-Identity header.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallerIdentity {
    #[serde(rename = "type")]
    pub identity_type: String,
    pub id: Option<String>,
    pub trust_domain: Option<String>,
    pub path: Option<String>,
    pub user_id: Option<String>,
    pub roles: Option<Vec<String>>,
    pub key_id: Option<String>,
}

/// User model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

/// Request to create a user.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

/// Request to update a user.
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub timestamp: String,
}

/// List users response.
#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub users: Vec<User>,
    pub total: usize,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Request context extracted from headers.
#[derive(Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub caller: Option<CallerIdentity>,
    pub operation_id: Option<String>,
}

// =============================================================================
// Application State
// =============================================================================

#[derive(Clone)]
pub struct AppState {
    users: Arc<RwLock<HashMap<String, User>>>,
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

fn parse_caller_identity(header_value: Option<&str>) -> Option<CallerIdentity> {
    header_value.and_then(|v| {
        serde_json::from_str(v)
            .map_err(|e| {
                warn!("Failed to parse caller identity: {}", e);
                e
            })
            .ok()
    })
}

fn get_request_context(headers: &HeaderMap) -> RequestContext {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let caller = parse_caller_identity(
        headers
            .get("x-caller-identity")
            .and_then(|v| v.to_str().ok()),
    );

    let operation_id = headers
        .get("x-operation-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    RequestContext {
        request_id,
        caller,
        operation_id,
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// =============================================================================
// Handlers
// =============================================================================

async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "example-rust".to_string(),
        timestamp: now_iso(),
    })
}

async fn list_users_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let ctx = get_request_context(&headers);
    info!(
        request_id = %ctx.request_id,
        caller = ?ctx.caller,
        "Listing users"
    );

    let users = state.users.read().unwrap();
    let users_vec: Vec<User> = users.values().cloned().collect();
    let total = users_vec.len();

    Json(UsersResponse {
        users: users_vec,
        total,
    })
}

async fn get_user_handler(
    headers: HeaderMap,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<User>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);
    info!(
        request_id = %ctx.request_id,
        user_id = %user_id,
        caller = ?ctx.caller,
        "Getting user"
    );

    let users = state.users.read().unwrap();
    match users.get(&user_id) {
        Some(user) => Ok(Json(user.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "USER_NOT_FOUND".to_string(),
                message: format!("User with ID '{}' not found", user_id),
                request_id: Some(ctx.request_id),
            }),
        )),
    }
}

async fn create_user_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);
    info!(
        request_id = %ctx.request_id,
        caller = ?ctx.caller,
        "Creating user"
    );

    let mut users = state.users.write().unwrap();

    // Check for duplicate email
    for user in users.values() {
        if user.email == body.email {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    code: "EMAIL_EXISTS".to_string(),
                    message: format!("User with email '{}' already exists", body.email),
                    request_id: Some(ctx.request_id),
                }),
            ));
        }
    }

    let user = User {
        id: Uuid::new_v4().to_string(),
        name: body.name,
        email: body.email,
        created_at: now_iso(),
    };

    info!(
        request_id = %ctx.request_id,
        user_id = %user.id,
        "Created user"
    );
    users.insert(user.id.clone(), user.clone());

    Ok((StatusCode::CREATED, Json(user)))
}

async fn update_user_handler(
    headers: HeaderMap,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<User>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);
    info!(
        request_id = %ctx.request_id,
        user_id = %user_id,
        caller = ?ctx.caller,
        "Updating user"
    );

    let mut users = state.users.write().unwrap();

    match users.get_mut(&user_id) {
        Some(user) => {
            if let Some(name) = body.name {
                user.name = name;
            }
            if let Some(email) = body.email {
                user.email = email;
            }
            info!(
                request_id = %ctx.request_id,
                user_id = %user_id,
                "Updated user"
            );
            Ok(Json(user.clone()))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "USER_NOT_FOUND".to_string(),
                message: format!("User with ID '{}' not found", user_id),
                request_id: Some(ctx.request_id),
            }),
        )),
    }
}

async fn delete_user_handler(
    headers: HeaderMap,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);
    info!(
        request_id = %ctx.request_id,
        user_id = %user_id,
        caller = ?ctx.caller,
        "Deleting user"
    );

    let mut users = state.users.write().unwrap();

    if users.remove(&user_id).is_some() {
        info!(
            request_id = %ctx.request_id,
            user_id = %user_id,
            "Deleted user"
        );
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "USER_NOT_FOUND".to_string(),
                message: format!("User with ID '{}' not found", user_id),
                request_id: Some(ctx.request_id),
            }),
        ))
    }
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() {
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

    // Application state
    let state = AppState::default();

    // Build router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/users", get(list_users_handler).post(create_user_handler))
        .route(
            "/users/:user_id",
            get(get_user_handler)
                .put(update_user_handler)
                .delete(delete_user_handler),
        )
        .with_state(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
    info!("Rust example service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
