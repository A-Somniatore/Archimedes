# archimedes-core

[![crates.io](https://img.shields.io/crates/v/archimedes-core.svg)](https://crates.io/crates/archimedes-core)
[![docs.rs](https://docs.rs/archimedes-core/badge.svg)](https://docs.rs/archimedes-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Core types and traits for the Archimedes server framework. This crate provides the foundational types used throughout Archimedes.

## Key Types

### Request Context

```rust
use archimedes_core::{RequestContext, CallerIdentity};

// RequestContext contains request metadata
let ctx = RequestContext::new(request_id, caller, operation_id, trace_id);

// Access caller identity
match ctx.caller() {
    CallerIdentity::User(user) => println!("User: {}", user.id),
    CallerIdentity::Service(svc) => println!("Service: {}", svc.spiffe_id),
    CallerIdentity::ApiKey(key) => println!("API Key: {}", key.key_id),
    CallerIdentity::Anonymous => println!("Anonymous"),
}
```

### Handler Trait

```rust
use archimedes_core::{Handler, InvocationContext, Response};
use std::future::Future;

// Handlers are async functions that process requests
pub trait Handler: Send + Sync + 'static {
    fn call(&self, ctx: InvocationContext) -> impl Future<Output = Response> + Send;
}

// Most handlers are simple async functions
async fn my_handler(ctx: InvocationContext) -> Response {
    Response::json(&json!({"status": "ok"}))
}
```

### Error Types

```rust
use archimedes_core::{ThemisError, ErrorEnvelope};

// ThemisError is the standard error type
#[derive(thiserror::Error, Debug)]
pub enum ThemisError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Internal error: {0}")]
    Internal(#[source] anyhow::Error),
}

// ErrorEnvelope provides consistent error responses
let envelope = ErrorEnvelope::new(
    "VALIDATION_ERROR",
    "Invalid email format",
    request_id,
);
```

### Dependency Injection

```rust
use archimedes_core::{Container, Inject};

// Register services in the DI container
let mut container = Container::new();
container.register::<Database>(Database::connect(url).await?);
container.register::<Cache>(Cache::new());

// Inject services in handlers
async fn get_user(
    db: Inject<Database>,
    cache: Inject<Cache>,
    path: Path<UserId>,
) -> Result<Json<User>, ThemisError> {
    // Services are automatically injected
    let user = db.get_user(path.0).await?;
    Ok(Json(user))
}
```

### Contract Types

```rust
use archimedes_core::{Contract, Operation, Schema};

// Contracts define API operations
let contract = Contract::load("contract.json")?;

// Operations are individual API endpoints
for op in contract.operations() {
    println!("Operation: {} {} {}", op.method, op.path, op.operation_id);
}
```

## Features

- `serde` - Enable serialization/deserialization (enabled by default)
- `validation` - Enable request/response validation

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Part of the Themis Platform

This crate is part of the [Archimedes](https://github.com/themis-platform/archimedes) server framework.
