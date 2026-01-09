//! # Archimedes Feature Showcase
//!
//! This example demonstrates **ALL** features available in Archimedes.
//! It serves as the reference implementation for all language bindings.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p feature-showcase
//! ```

use std::sync::Arc;

mod config;
mod middleware;
mod routes;
mod tasks;

use archimedes_core::di::Container;
use archimedes_server::{Lifecycle, Server};
use config::AppConfig;
use routes::AppState;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ==========================================================================
    // 1. CONFIGURATION
    // ==========================================================================
    // Load configuration from file with environment overrides
    let config = AppConfig::load()?;
    
    // Initialize logging based on config
    config::init_logging(&config);
    
    info!("Starting Archimedes Feature Showcase");
    info!("Listen address: {}", config.listen_addr);

    // ==========================================================================
    // 2. APPLICATION STATE & DEPENDENCY INJECTION
    // ==========================================================================
    // Create shared application state
    let app_state = Arc::new(AppState::new());
    
    // Set up DI container
    let mut container = Container::new();
    container.register(app_state.clone());
    container.register(config.clone());

    // ==========================================================================
    // 3. LIFECYCLE HOOKS
    // ==========================================================================
    let lifecycle = Lifecycle::new()
        .on_startup("database_connect", {
            let state = app_state.clone();
            move || {
                let state = state.clone();
                async move {
                    info!("Startup hook: Initializing database connection...");
                    state.init_db().await;
                    Ok(())
                }
            }
        })
        .on_startup("cache_warmup", {
            let state = app_state.clone();
            move || {
                let state = state.clone();
                async move {
                    info!("Startup hook: Warming up cache...");
                    state.warmup_cache().await;
                    Ok(())
                }
            }
        })
        .on_shutdown("database_disconnect", {
            let state = app_state.clone();
            move || {
                let state = state.clone();
                async move {
                    info!("Shutdown hook: Closing database connection...");
                    state.close_db().await;
                    Ok(())
                }
            }
        })
        .on_shutdown("flush_metrics", || async {
            info!("Shutdown hook: Flushing metrics...");
            Ok(())
        });

    // ==========================================================================
    // 4. BACKGROUND TASKS
    // ==========================================================================
    let task_spawner = tasks::setup_background_tasks(app_state.clone()).await;
    container.register(task_spawner);

    // ==========================================================================
    // 5. BUILD SERVER WITH ALL FEATURES
    // ==========================================================================
    let mut server = Server::builder()
        .http_addr(&config.listen_addr)
        .service_name("feature-showcase")
        .lifecycle(lifecycle)
        .build();

    // ==========================================================================
    // 6. REGISTER ROUTES (Sub-routers with prefixes and tags)
    // ==========================================================================
    
    // API v1 routes - demonstrates sub-router nesting
    routes::register_routes(&mut server, app_state.clone());
    
    // Static file serving
    routes::register_static_files(&mut server);
    
    // Documentation endpoints
    routes::register_docs(&mut server);

    // ==========================================================================
    // 7. START SERVER
    // ==========================================================================
    info!("Server starting on {}", config.listen_addr);
    info!("API docs available at http://{}/docs", config.listen_addr);
    
    server.run().await?;
    
    info!("Server shutdown complete");
    Ok(())
}

// =============================================================================
// TESTS - Demonstrates TestClient usage
// =============================================================================
#[cfg(test)]
mod tests {
    use archimedes_test::TestClient;
    use serde_json::json;

    /// Test health endpoint
    #[tokio::test]
    async fn test_health_endpoint() {
        let client = TestClient::new();
        
        let response = client
            .get("/health")
            .send()
            .await
            .expect("Request failed");
        
        response.assert_status(200);
        response.assert_json_field("status", "healthy");
    }

    /// Test user CRUD operations
    #[tokio::test]
    async fn test_user_crud() {
        let client = TestClient::new();
        
        // Create user
        let create_response = client
            .post("/api/v1/users")
            .json(&json!({
                "name": "Test User",
                "email": "test@example.com"
            }))
            .send()
            .await
            .expect("Create failed");
        
        create_response.assert_status(201);
        let user_id = create_response
            .json::<serde_json::Value>()
            .await
            .expect("JSON parse failed")
            ["id"]
            .as_str()
            .expect("No id")
            .to_string();

        // Get user
        let get_response = client
            .get(&format!("/api/v1/users/{}", user_id))
            .send()
            .await
            .expect("Get failed");
        
        get_response.assert_status(200);
        get_response.assert_json_field("name", "Test User");

        // Update user
        let update_response = client
            .put(&format!("/api/v1/users/{}", user_id))
            .json(&json!({
                "name": "Updated User"
            }))
            .send()
            .await
            .expect("Update failed");
        
        update_response.assert_status(200);
        update_response.assert_json_field("name", "Updated User");

        // Delete user
        let delete_response = client
            .delete(&format!("/api/v1/users/{}", user_id))
            .send()
            .await
            .expect("Delete failed");
        
        delete_response.assert_status(204);
    }

    /// Test form submission
    #[tokio::test]
    async fn test_login_form() {
        let client = TestClient::new();
        
        let response = client
            .post("/api/v1/auth/login")
            .form(&[
                ("username", "admin"),
                ("password", "secret"),
            ])
            .send()
            .await
            .expect("Login failed");
        
        response.assert_status(200);
        response.assert_header("set-cookie", |v| v.contains("session_id="));
    }

    /// Test cookie extraction
    #[tokio::test]
    async fn test_session_cookies() {
        let client = TestClient::new();
        
        let response = client
            .get("/api/v1/auth/session")
            .header("cookie", "session_id=abc123")
            .send()
            .await
            .expect("Request failed");
        
        response.assert_status(200);
        response.assert_json_field("session_id", "abc123");
    }

    /// Test file upload (multipart)
    #[tokio::test]
    async fn test_file_upload() {
        let client = TestClient::new();
        
        let response = client
            .post("/api/v1/files/upload")
            .multipart()
            .file("document", "test.txt", b"Hello, World!", "text/plain")
            .send()
            .await
            .expect("Upload failed");
        
        response.assert_status(200);
        response.assert_json_field("filename", "test.txt");
    }

    /// Test SSE endpoint
    #[tokio::test]
    async fn test_sse_endpoint() {
        let client = TestClient::new();
        
        let response = client
            .get("/api/v1/realtime/events")
            .header("accept", "text/event-stream")
            .send()
            .await
            .expect("SSE failed");
        
        response.assert_status(200);
        response.assert_header("content-type", |v| v.contains("text/event-stream"));
    }

    /// Test rate limiting
    #[tokio::test]
    async fn test_rate_limiting() {
        let client = TestClient::new();
        
        // First request should succeed
        let response = client
            .get("/api/v1/users")
            .send()
            .await
            .expect("Request failed");
        
        response.assert_status(200);
        response.assert_header("x-ratelimit-remaining", |v| {
            v.parse::<u32>().map(|n| n > 0).unwrap_or(false)
        });
    }

    /// Test CORS headers
    #[tokio::test]
    async fn test_cors_preflight() {
        let client = TestClient::new();
        
        let response = client
            .options("/api/v1/users")
            .header("origin", "https://example.com")
            .header("access-control-request-method", "POST")
            .send()
            .await
            .expect("Preflight failed");
        
        response.assert_status(200);
        response.assert_header("access-control-allow-origin", |v| v == "https://example.com" || v == "*");
    }

    /// Test redirect response
    #[tokio::test]
    async fn test_redirect() {
        let client = TestClient::new();
        
        let response = client
            .get("/api/v1/auth/logout")
            .send()
            .await
            .expect("Request failed");
        
        response.assert_status(302);
        response.assert_header("location", |v| v == "/");
    }

    /// Test HTML response
    #[tokio::test]
    async fn test_html_response() {
        let client = TestClient::new();
        
        let response = client
            .get("/")
            .send()
            .await
            .expect("Request failed");
        
        response.assert_status(200);
        response.assert_header("content-type", |v| v.contains("text/html"));
    }
}
