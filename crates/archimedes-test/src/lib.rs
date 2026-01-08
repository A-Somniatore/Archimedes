//! # Archimedes Test
//!
//! Test utilities for the Archimedes framework, providing in-memory HTTP
//! testing without requiring actual network connections or port binding.
//!
//! This crate is a **P0 migration blocker** required for testing browser-facing
//! APIs and ensuring services can be properly tested before deployment.
//!
//! ## Key Features
//!
//! - **In-Memory Testing**: No real network connections or port binding
//! - **Request Builder**: Fluent API for building test requests
//! - **Response Assertions**: Helper methods for validating responses
//! - **JSON Support**: Automatic serialization/deserialization of JSON bodies
//! - **Full Middleware**: Requests go through the complete middleware pipeline
//!
//! ## Example
//!
//! ```ignore
//! use archimedes_test::{TestClient, TestRequest};
//! use serde_json::json;
//!
//! #[tokio::test]
//! async fn test_get_user() {
//!     // Create test client from your app
//!     let client = TestClient::new(app);
//!
//!     // Make a request
//!     let response = client
//!         .get("/users/123")
//!         .header("Authorization", "Bearer token")
//!         .send()
//!         .await;
//!
//!     // Assert response
//!     assert_eq!(response.status(), 200);
//!
//!     let user: User = response.json().await.unwrap();
//!     assert_eq!(user.id, "123");
//! }
//!
//! #[tokio::test]
//! async fn test_create_user() {
//!     let client = TestClient::new(app);
//!
//!     let response = client
//!         .post("/users")
//!         .json(&json!({
//!             "name": "Alice",
//!             "email": "alice@example.com"
//!         }))
//!         .send()
//!         .await;
//!
//!     assert_eq!(response.status(), 201);
//! }
//! ```
//!
//! ## Comparison with Other Test Frameworks
//!
//! | Framework  | Real Network | Middleware | JSON Helpers |
//! |------------|--------------|------------|--------------|
//! | Archimedes | ❌ In-memory | ✅ Full    | ✅           |
//! | reqwest    | ✅ Required  | N/A        | ✅           |
//! | axum::test | ❌ In-memory | ✅ Full    | ✅           |
//! | actix-test | ❌ In-memory | ✅ Full    | ⚠️ Manual    |

#![doc(html_root_url = "https://docs.rs/archimedes-test/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod client;
mod error;
mod request;
mod response;

pub use client::TestClient;
pub use error::TestError;
pub use request::{TestRequest, TestRequestBuilder};
pub use response::TestResponse;
