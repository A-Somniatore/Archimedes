# archimedes-test

[![crates.io](https://img.shields.io/crates/v/archimedes-test.svg)](https://crates.io/crates/archimedes-test)
[![docs.rs](https://docs.rs/archimedes-test/badge.svg)](https://docs.rs/archimedes-test)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Test utilities for the Archimedes HTTP framework. Provides in-memory HTTP testing without real network connections.

## Features

- **TestClient** - In-memory HTTP client for handler testing
- **TestRequest** - Fluent request builder with JSON, form, and headers support
- **TestResponse** - Response wrapper with assertion helpers
- **No network required** - Tests run without binding to ports

## Quick Start

```rust
use archimedes_test::{TestClient, TestRequest};
use archimedes_server::Router;
use archimedes_core::{Response, StatusCode};

async fn hello_handler() -> Response {
    Response::text("Hello, World!")
}

#[tokio::test]
async fn test_hello() {
    // Create router with handler
    let mut router = Router::new();
    router.get("/hello", hello_handler);

    // Create test client
    let client = TestClient::new(router);

    // Make request
    let response = client
        .get("/hello")
        .send()
        .await
        .unwrap();

    // Assert response
    response.assert_status(StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "Hello, World!");
}
```

## TestRequest Builder

```rust
// JSON body
let response = client
    .post("/users")
    .json(&json!({"name": "Alice", "email": "alice@example.com"}))
    .send()
    .await?;

// Form data
let response = client
    .post("/login")
    .form(&[("username", "alice"), ("password", "secret")])
    .send()
    .await?;

// Custom headers
let response = client
    .get("/api/data")
    .header("Authorization", "Bearer token123")
    .header("X-Request-Id", "req-001")
    .send()
    .await?;

// Query parameters
let response = client
    .get("/search")
    .query(&[("q", "rust"), ("limit", "10")])
    .send()
    .await?;
```

## TestResponse Assertions

```rust
// Status assertions
response.assert_status(StatusCode::OK);
response.assert_status(StatusCode::CREATED);

// Header assertions
response.assert_header("content-type", "application/json");

// JSON assertions
response.assert_json_field("id", 123);
response.assert_json_field("name", "Alice");

// Body access
let body = response.text().await?;
let json: serde_json::Value = response.json().await?;
let bytes = response.bytes().await?;
```

## Integration with Archimedes

The TestClient integrates seamlessly with Archimedes applications:

```rust
use archimedes::prelude::*;
use archimedes_test::TestClient;

#[tokio::test]
async fn test_full_application() {
    // Create your Archimedes app
    let app = Archimedes::new()
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .route("/users/{id}", get(get_user));

    // Test with TestClient
    let client = TestClient::from_app(app);

    // Test list users
    let response = client.get("/users").send().await?;
    response.assert_status(StatusCode::OK);

    // Test create user
    let response = client
        .post("/users")
        .json(&json!({"name": "Bob"}))
        .send()
        .await?;
    response.assert_status(StatusCode::CREATED);
}
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Part of the Themis Platform

This crate is part of the [Archimedes](https://github.com/themis-platform/archimedes) server framework.
