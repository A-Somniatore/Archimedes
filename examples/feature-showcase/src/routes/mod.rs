//! Routes module - demonstrates sub-router organization.
//!
//! This module showcases the FastAPI-style router composition:
//! - `nest()` - Mount a sub-router at a path prefix
//! - `merge()` - Combine routers at the same level
//! - `prefix()` - Add a path prefix to all routes
//! - `tag()` - Add OpenAPI tags for documentation

pub mod auth;
pub mod files;
pub mod realtime;
pub mod users;

use archimedes_core::di::Container;
use archimedes_router::Router;
use archimedes_server::Server;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// Shared application state
pub struct AppState {
    /// In-memory user store (for demo purposes)
    pub users: RwLock<HashMap<Uuid, users::User>>,
    /// Database initialized flag
    pub db_initialized: RwLock<bool>,
    /// Cache warmed flag
    pub cache_warmed: RwLock<bool>,
}

impl AppState {
    /// Create new application state.
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
            db_initialized: RwLock::new(false),
            cache_warmed: RwLock::new(false),
        }
    }

    /// Initialize database connection (lifecycle hook).
    pub async fn init_db(&self) {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        *self.db_initialized.write().await = true;
    }

    /// Warm up cache (lifecycle hook).
    pub async fn warmup_cache(&self) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        *self.cache_warmed.write().await = true;
    }

    /// Close database connection (lifecycle hook).
    pub async fn close_db(&self) {
        *self.db_initialized.write().await = false;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Register all routes on the server.
///
/// This demonstrates the sub-router pattern for organizing routes.
pub fn register_routes(server: &mut Server, state: Arc<AppState>) {
    // ==========================================================================
    // HEALTH & READINESS PROBES
    // ==========================================================================
    server.get("/health", || async {
        archimedes_core::response::json(&serde_json::json!({
            "status": "healthy"
        }))
    });

    server.get("/ready", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move {
                let db = *state.db_initialized.read().await;
                let cache = *state.cache_warmed.read().await;
                
                if db && cache {
                    archimedes_core::response::json(&serde_json::json!({
                        "status": "ready",
                        "checks": {
                            "database": "connected",
                            "cache": "warmed"
                        }
                    }))
                } else {
                    archimedes_core::response::json(&serde_json::json!({
                        "status": "not_ready",
                        "checks": {
                            "database": if db { "connected" } else { "disconnected" },
                            "cache": if cache { "warmed" } else { "cold" }
                        }
                    }))
                }
            }
        }
    });

    // ==========================================================================
    // API V1 - Sub-router with prefix
    // ==========================================================================
    let mut api_v1 = Router::new();
    
    // Users routes
    let users_router = users::routes(state.clone());
    api_v1.nest("/users", users_router);
    
    // Auth routes
    let auth_router = auth::routes(state.clone());
    api_v1.nest("/auth", auth_router);
    
    // Files routes
    let files_router = files::routes(state.clone());
    api_v1.nest("/files", files_router);
    
    // Real-time routes (WebSocket, SSE)
    let realtime_router = realtime::routes(state.clone());
    api_v1.nest("/realtime", realtime_router);
    
    // Mount API v1 at /api/v1
    server.nest("/api/v1", api_v1);

    // ==========================================================================
    // ROOT - HTML welcome page
    // ==========================================================================
    server.get("/", || async {
        archimedes_core::response::html(include_str!("../static/index.html"))
    });
}

/// Register static file serving.
pub fn register_static_files(server: &mut Server) {
    // Serve static files from the static directory
    server.static_files("/static", "static");
}

/// Register documentation endpoints.
pub fn register_docs(server: &mut Server) {
    // OpenAPI spec (would be generated from contract.json)
    server.get("/openapi.json", || async {
        archimedes_core::response::json(&serde_json::json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Feature Showcase API",
                "version": "1.0.0"
            },
            "paths": {}
        }))
    });

    // Swagger UI redirect
    server.get("/docs", || async {
        archimedes_core::response::redirect("/static/swagger-ui.html")
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        assert!(state.users.blocking_read().is_empty());
    }

    #[tokio::test]
    async fn test_lifecycle_hooks() {
        let state = AppState::new();
        
        assert!(!*state.db_initialized.read().await);
        state.init_db().await;
        assert!(*state.db_initialized.read().await);
        
        assert!(!*state.cache_warmed.read().await);
        state.warmup_cache().await;
        assert!(*state.cache_warmed.read().await);
        
        state.close_db().await;
        assert!(!*state.db_initialized.read().await);
    }
}
