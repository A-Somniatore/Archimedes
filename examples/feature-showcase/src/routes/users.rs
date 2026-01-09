//! Users routes - demonstrates CRUD operations with extractors.
//!
//! ## Features Demonstrated
//! - JSON request/response bodies
//! - Path parameters
//! - Query parameters
//! - Header extraction
//! - State injection
//! - Pagination patterns

use archimedes_core::extract::{Headers, Inject, Json, Path, Query};
use archimedes_core::response::{json, Response};
use archimedes_router::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::AppState;

/// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Request to create a user
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub role: Option<String>,
}

/// Request to update a user
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// Query parameters for listing users
#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Filter by role
    #[serde(default)]
    pub role: Option<String>,
    /// Search by name
    #[serde(default)]
    pub search: Option<String>,
}

fn default_page() -> u32 {
    1
}
fn default_limit() -> u32 {
    10
}

/// Paginated response wrapper
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub limit: u32,
    pub total: usize,
    pub total_pages: u32,
}

/// Create the users router.
///
/// ## Endpoints
///
/// | Method | Path | Description |
/// |--------|------|-------------|
/// | GET | /users | List all users (paginated) |
/// | POST | /users | Create a new user |
/// | GET | /users/{id} | Get user by ID |
/// | PUT | /users/{id} | Update user |
/// | DELETE | /users/{id} | Delete user |
pub fn routes(state: Arc<AppState>) -> Router {
    let mut router = Router::new();

    // -------------------------------------------------------------------------
    // LIST USERS - Demonstrates Query extractor and pagination
    // -------------------------------------------------------------------------
    router.get("/", {
        let state = state.clone();
        move |Query(query): Query<ListUsersQuery>| {
            let state = state.clone();
            async move {
                let users = state.users.read().await;
                
                // Apply filters
                let mut filtered: Vec<&User> = users.values().collect();
                
                if let Some(ref role) = query.role {
                    filtered.retain(|u| u.role.as_ref() == Some(role));
                }
                
                if let Some(ref search) = query.search {
                    let search_lower = search.to_lowercase();
                    filtered.retain(|u| u.name.to_lowercase().contains(&search_lower));
                }
                
                let total = filtered.len();
                let total_pages = (total as f64 / query.limit as f64).ceil() as u32;
                
                // Apply pagination
                let start = ((query.page - 1) * query.limit) as usize;
                let items: Vec<User> = filtered
                    .into_iter()
                    .skip(start)
                    .take(query.limit as usize)
                    .cloned()
                    .collect();
                
                json(&PaginatedResponse {
                    items,
                    page: query.page,
                    limit: query.limit,
                    total,
                    total_pages,
                })
            }
        }
    });

    // -------------------------------------------------------------------------
    // CREATE USER - Demonstrates JSON body extractor
    // -------------------------------------------------------------------------
    router.post("/", {
        let state = state.clone();
        move |Json(request): Json<CreateUserRequest>| {
            let state = state.clone();
            async move {
                let now = chrono::Utc::now();
                let user = User {
                    id: Uuid::new_v4(),
                    name: request.name,
                    email: request.email,
                    role: request.role,
                    created_at: now,
                    updated_at: now,
                };
                
                let user_clone = user.clone();
                state.users.write().await.insert(user.id, user);
                
                Response::builder()
                    .status(201)
                    .header("location", format!("/api/v1/users/{}", user_clone.id))
                    .json(&user_clone)
            }
        }
    });

    // -------------------------------------------------------------------------
    // GET USER BY ID - Demonstrates Path extractor
    // -------------------------------------------------------------------------
    router.get("/:id", {
        let state = state.clone();
        move |Path(id): Path<Uuid>| {
            let state = state.clone();
            async move {
                let users = state.users.read().await;
                
                match users.get(&id) {
                    Some(user) => json(user),
                    None => Response::builder()
                        .status(404)
                        .json(&serde_json::json!({
                            "error": "User not found",
                            "id": id.to_string()
                        })),
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // UPDATE USER - Demonstrates Path + JSON extractors
    // -------------------------------------------------------------------------
    router.put("/:id", {
        let state = state.clone();
        move |Path(id): Path<Uuid>, Json(request): Json<UpdateUserRequest>| {
            let state = state.clone();
            async move {
                let mut users = state.users.write().await;
                
                match users.get_mut(&id) {
                    Some(user) => {
                        if let Some(name) = request.name {
                            user.name = name;
                        }
                        if let Some(email) = request.email {
                            user.email = email;
                        }
                        if let Some(role) = request.role {
                            user.role = Some(role);
                        }
                        user.updated_at = chrono::Utc::now();
                        
                        json(user)
                    }
                    None => Response::builder()
                        .status(404)
                        .json(&serde_json::json!({
                            "error": "User not found",
                            "id": id.to_string()
                        })),
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // DELETE USER - Demonstrates Path extractor with 204 response
    // -------------------------------------------------------------------------
    router.delete("/:id", {
        let state = state.clone();
        move |Path(id): Path<Uuid>| {
            let state = state.clone();
            async move {
                let mut users = state.users.write().await;
                
                match users.remove(&id) {
                    Some(_) => Response::builder().status(204).body(vec![]),
                    None => Response::builder()
                        .status(404)
                        .json(&serde_json::json!({
                            "error": "User not found",
                            "id": id.to_string()
                        })),
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // GET CURRENT USER - Demonstrates Headers extractor
    // -------------------------------------------------------------------------
    router.get("/me", move |Headers(headers): Headers| async move {
        // In a real app, this would decode a JWT or session token
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if auth_header.starts_with("Bearer ") {
            json(&serde_json::json!({
                "id": "current-user-id",
                "authenticated": true,
                "token_prefix": &auth_header[..20.min(auth_header.len())]
            }))
        } else {
            Response::builder()
                .status(401)
                .json(&serde_json::json!({
                    "error": "Authentication required",
                    "hint": "Provide Authorization: Bearer <token> header"
                }))
        }
    });

    router.tag("users");
    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
            role: Some("admin".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&user).expect("Should serialize");
        assert!(json.contains("Test"));
        assert!(json.contains("admin"));
    }

    #[test]
    fn test_create_request_deserialization() {
        let json = r#"{"name": "Test", "email": "test@example.com"}"#;
        let request: CreateUserRequest = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(request.name, "Test");
        assert_eq!(request.email, "test@example.com");
        assert!(request.role.is_none());
    }

    #[test]
    fn test_query_defaults() {
        let json = r#"{}"#;
        let query: ListUsersQuery = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(query.page, 1);
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_pagination_calculation() {
        let total = 25usize;
        let limit = 10u32;
        let total_pages = (total as f64 / limit as f64).ceil() as u32;
        assert_eq!(total_pages, 3);
    }
}
