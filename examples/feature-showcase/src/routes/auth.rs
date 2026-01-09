//! Auth routes - demonstrates form handling and cookie management.
//!
//! ## Features Demonstrated
//! - Form extractor (URL-encoded bodies)
//! - Cookie extractor
//! - SetCookie response header
//! - Redirect responses
//! - Session management patterns

use archimedes_core::extract::{Cookies, Form};
use archimedes_core::response::{json, redirect, set_cookie, Response};
use archimedes_router::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::AppState;

/// Login form request
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    /// Remember me checkbox
    #[serde(default)]
    pub remember_me: bool,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Session info from cookies
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Create the auth router.
///
/// ## Endpoints
///
/// | Method | Path | Description |
/// |--------|------|-------------|
/// | POST | /login | Login with form data |
/// | GET | /logout | Logout (clears session cookie) |
/// | GET | /session | Get current session info |
/// | POST | /refresh | Refresh session token |
pub fn routes(_state: Arc<AppState>) -> Router {
    let mut router = Router::new();

    // -------------------------------------------------------------------------
    // LOGIN - Demonstrates Form extractor and SetCookie response
    // -------------------------------------------------------------------------
    router.post("/login", |Form(form): Form<LoginForm>| async move {
        // In a real app, verify credentials against database
        if form.username == "admin" && form.password == "secret" {
            let session_id = Uuid::new_v4().to_string();
            let max_age = if form.remember_me {
                60 * 60 * 24 * 30 // 30 days
            } else {
                60 * 60 * 24 // 1 day
            };

            Response::builder()
                .status(200)
                .set_cookie("session_id", &session_id, |cookie| {
                    cookie
                        .http_only(true)
                        .secure(true)
                        .same_site_strict()
                        .max_age(max_age)
                        .path("/")
                })
                .json(&LoginResponse {
                    success: true,
                    message: "Login successful".to_string(),
                    user_id: Some("user-123".to_string()),
                })
        } else {
            Response::builder()
                .status(401)
                .json(&LoginResponse {
                    success: false,
                    message: "Invalid username or password".to_string(),
                    user_id: None,
                })
        }
    });

    // -------------------------------------------------------------------------
    // LOGOUT - Demonstrates cookie clearing and redirect
    // -------------------------------------------------------------------------
    router.get("/logout", || async {
        Response::builder()
            .status(302)
            .set_cookie("session_id", "", |cookie| {
                cookie
                    .http_only(true)
                    .max_age(0) // Expire immediately
                    .path("/")
            })
            .header("location", "/")
            .body(vec![])
    });

    // -------------------------------------------------------------------------
    // SESSION - Demonstrates Cookies extractor
    // -------------------------------------------------------------------------
    router.get("/session", |Cookies(cookies): Cookies| async move {
        match cookies.get("session_id") {
            Some(session_id) => {
                let now = chrono::Utc::now();
                json(&SessionInfo {
                    session_id: session_id.to_string(),
                    created_at: now.to_rfc3339(),
                    expires_at: (now + chrono::Duration::hours(24)).to_rfc3339(),
                })
            }
            None => Response::builder()
                .status(401)
                .json(&serde_json::json!({
                    "error": "No session found",
                    "hint": "Please login first"
                })),
        }
    });

    // -------------------------------------------------------------------------
    // REFRESH - Demonstrates session token rotation
    // -------------------------------------------------------------------------
    router.post("/refresh", |Cookies(cookies): Cookies| async move {
        match cookies.get("session_id") {
            Some(_old_session) => {
                // Generate new session ID (token rotation)
                let new_session_id = Uuid::new_v4().to_string();

                Response::builder()
                    .status(200)
                    .set_cookie("session_id", &new_session_id, |cookie| {
                        cookie
                            .http_only(true)
                            .secure(true)
                            .same_site_strict()
                            .max_age(60 * 60 * 24) // 1 day
                            .path("/")
                    })
                    .json(&serde_json::json!({
                        "success": true,
                        "message": "Session refreshed"
                    }))
            }
            None => Response::builder()
                .status(401)
                .json(&serde_json::json!({
                    "error": "No session to refresh",
                    "hint": "Please login first"
                })),
        }
    });

    // -------------------------------------------------------------------------
    // API KEY - Demonstrates multiple cookie handling
    // -------------------------------------------------------------------------
    router.post("/api-key", |Cookies(cookies): Cookies| async move {
        // Verify session before generating API key
        match cookies.get("session_id") {
            Some(_session) => {
                let api_key = format!("ak_{}", Uuid::new_v4().to_string().replace("-", ""));

                Response::builder()
                    .status(200)
                    .set_cookie("api_key", &api_key, |cookie| {
                        cookie
                            .http_only(false) // Allow JS access for API calls
                            .secure(true)
                            .max_age(60 * 60 * 24 * 365) // 1 year
                            .path("/api")
                    })
                    .json(&serde_json::json!({
                        "api_key": api_key,
                        "expires_in": "1 year"
                    }))
            }
            None => Response::builder()
                .status(401)
                .json(&serde_json::json!({
                    "error": "Authentication required"
                })),
        }
    });

    router.tag("auth");
    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_form_deserialization() {
        // Standard form format
        let form_data = "username=admin&password=secret&remember_me=true";
        let form: LoginForm = serde_urlencoded::from_str(form_data).expect("Should deserialize");
        assert_eq!(form.username, "admin");
        assert_eq!(form.password, "secret");
        assert!(form.remember_me);
    }

    #[test]
    fn test_login_form_defaults() {
        let form_data = "username=user&password=pass";
        let form: LoginForm = serde_urlencoded::from_str(form_data).expect("Should deserialize");
        assert!(!form.remember_me); // Should default to false
    }

    #[test]
    fn test_login_response_serialization() {
        let response = LoginResponse {
            success: true,
            message: "OK".to_string(),
            user_id: Some("123".to_string()),
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(json.contains("success"));
        assert!(json.contains("user_id"));
    }

    #[test]
    fn test_login_response_skips_none() {
        let response = LoginResponse {
            success: false,
            message: "Failed".to_string(),
            user_id: None,
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(!json.contains("user_id"));
    }
}
