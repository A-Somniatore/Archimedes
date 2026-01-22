# archimedes-extract

[![crates.io](https://img.shields.io/crates/v/archimedes-extract.svg)](https://crates.io/crates/archimedes-extract)
[![docs.rs](https://docs.rs/archimedes-extract/badge.svg)](https://docs.rs/archimedes-extract)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Request extractors and response builders for the Archimedes HTTP framework. Provides type-safe extraction of data from HTTP requests and construction of responses.

## Extractors

### Body Extractors

```rust
use archimedes_extract::{Json, Form, Bytes, Text, Multipart};
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

// JSON body
async fn create_user(Json(user): Json<CreateUser>) -> Response {
    Response::json(&user)
}

// Form data (application/x-www-form-urlencoded)
async fn login(Form(creds): Form<LoginForm>) -> Response {
    // Process form data
}

// Raw bytes
async fn upload(body: Bytes) -> Response {
    println!("Received {} bytes", body.len());
    Response::ok()
}

// UTF-8 text
async fn process_text(text: Text) -> Response {
    println!("Text: {}", text.0);
    Response::ok()
}
```

### Multipart File Uploads

```rust
use archimedes_extract::{Multipart, MultipartField};

async fn upload_file(mut multipart: Multipart) -> Response {
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("unknown");
        let filename = field.file_name();
        let content_type = field.content_type();
        let data = field.bytes().await?;

        println!("Field: {}, Size: {} bytes", name, data.len());
    }
    Response::ok()
}
```

### Parameter Extractors

```rust
use archimedes_extract::{Path, Query, Headers};
use serde::Deserialize;

// Path parameters
async fn get_user(Path(user_id): Path<i64>) -> Response {
    // user_id extracted from /users/{user_id}
}

// Multiple path parameters
async fn get_post(Path((user_id, post_id)): Path<(i64, i64)>) -> Response {
    // From /users/{user_id}/posts/{post_id}
}

// Query parameters
#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    limit: Option<u32>,
}

async fn list_users(Query(pagination): Query<Pagination>) -> Response {
    let page = pagination.page.unwrap_or(1);
    let limit = pagination.limit.unwrap_or(20);
    // ...
}

// Headers
async fn check_auth(headers: Headers) -> Response {
    if let Some(auth) = headers.get("Authorization") {
        // Process auth header
    }
    Response::unauthorized()
}
```

### Cookie Extractor

```rust
use archimedes_extract::Cookies;

async fn get_session(cookies: Cookies) -> Response {
    if let Some(session_id) = cookies.get("session_id") {
        // Validate session
    }
    Response::unauthorized()
}
```

### Context Extractors

```rust
use archimedes_extract::{Inject, State};
use std::sync::Arc;

// Dependency injection
async fn get_data(db: Inject<Database>) -> Response {
    let data = db.query().await?;
    Response::json(&data)
}

// Shared application state
async fn counter(State(state): State<Arc<AppState>>) -> Response {
    let count = state.counter.fetch_add(1, Ordering::SeqCst);
    Response::json(&json!({"count": count}))
}
```

## Response Builders

### Standard Responses

```rust
use archimedes_extract::Response;

// JSON response
Response::json(&json!({"status": "ok"}))

// Text response
Response::text("Hello, World!")

// HTML response
Response::html("<h1>Hello</h1>")

// No content (204)
Response::no_content()

// Redirects
Response::redirect("/new-location")           // 302 Found
Response::redirect_permanent("/new-location") // 301 Moved Permanently
Response::redirect_see_other("/new-location") // 303 See Other
```

### File Responses

```rust
use archimedes_extract::FileResponse;

// Download file as attachment
FileResponse::attachment("report.pdf", pdf_bytes)

// Display inline (e.g., images)
FileResponse::inline("image.png", image_bytes)

// With custom content type
FileResponse::new("data.csv", csv_bytes)
    .content_type("text/csv")
```

### Cookie Responses

```rust
use archimedes_extract::SetCookie;

// Set a cookie
let cookie = SetCookie::new("session_id", "abc123")
    .http_only(true)
    .secure(true)
    .same_site(SameSite::Strict)
    .max_age(Duration::from_secs(3600));

Response::ok().with_cookie(cookie)

// Delete a cookie
let delete = SetCookie::delete("session_id");
Response::ok().with_cookie(delete)
```

## FromRequest Trait

Implement `FromRequest` for custom extractors:

```rust
use archimedes_extract::{FromRequest, ExtractionContext};
use async_trait::async_trait;

struct CurrentUser(User);

#[async_trait]
impl FromRequest for CurrentUser {
    type Rejection = ThemisError;

    async fn from_request(ctx: &mut ExtractionContext) -> Result<Self, Self::Rejection> {
        // Extract user from request context
        let user_id = ctx.request_context()
            .caller()
            .user_id()
            .ok_or(ThemisError::Unauthorized)?;

        let db = ctx.inject::<Database>()?;
        let user = db.get_user(user_id).await?;

        Ok(CurrentUser(user))
    }
}

// Use in handlers
async fn profile(CurrentUser(user): CurrentUser) -> Response {
    Response::json(&user)
}
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Part of the Themis Platform

This crate is part of the [Archimedes](https://github.com/themis-platform/archimedes) server framework.
