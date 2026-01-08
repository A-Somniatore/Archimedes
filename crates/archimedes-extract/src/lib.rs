//! # Archimedes Extract
//!
//! Request extractors and response builders for the Archimedes HTTP framework.
//!
//! This crate provides type-safe extraction of data from HTTP requests and
//! convenient response building utilities. All extractors integrate with
//! Archimedes's contract validation system.
//!
//! ## Extractors
//!
//! Extractors parse data from different parts of an HTTP request:
//!
//! | Extractor | Source | Description |
//! |-----------|--------|-------------|
//! | [`Path<T>`] | URL path | Extract typed parameters from path segments |
//! | [`Query<T>`] | Query string | Parse URL query parameters |
//! | [`Json<T>`] | Request body | Deserialize JSON body |
//! | [`Form<T>`] | Request body | Parse URL-encoded form data |
//! | [`Header<T>`] | Headers | Extract a typed header value |
//! | [`Headers`] | Headers | Access all request headers |
//! | [`RawBody`] | Request body | Access raw request bytes |
//!
//! ## Example
//!
//! ```rust
//! use archimedes_extract::{Path, Query, Json, FromRequest, ExtractionContext};
//! use archimedes_router::Params;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct UserPath {
//!     user_id: u64,
//! }
//!
//! #[derive(Deserialize)]
//! struct ListParams {
//!     limit: Option<u32>,
//!     offset: Option<u32>,
//! }
//!
//! #[derive(Deserialize)]
//! struct CreateUser {
//!     name: String,
//!     email: String,
//! }
//!
//! // In a handler, extractors would be used like:
//! // async fn get_user(Path(path): Path<UserPath>) -> impl Response { ... }
//! // async fn list_users(Query(params): Query<ListParams>) -> impl Response { ... }
//! // async fn create_user(Json(body): Json<CreateUser>) -> impl Response { ... }
//! ```
//!
//! ## Response Builders
//!
//! Response builders construct HTTP responses with proper content types:
//!
//! ```rust
//! use archimedes_extract::response::{JsonResponse, HtmlResponse, Redirect};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! // Return JSON response
//! let json = JsonResponse::new(User { id: 1, name: "Alice".into() });
//!
//! // Return HTML response
//! let html = HtmlResponse::new("<h1>Hello</h1>");
//!
//! // Return redirect
//! let redirect = Redirect::to("/dashboard");
//! ```
//!
//! ## Error Handling
//!
//! All extractors return [`ExtractionError`] on failure, which includes:
//!
//! - Source location (path, query, body, header)
//! - Detailed error message
//! - Automatic conversion to HTTP 400/422 responses
//!
//! ```rust
//! use archimedes_extract::{ExtractionError, ExtractionSource};
//!
//! let err = ExtractionError::invalid_type(
//!     ExtractionSource::Path,
//!     "user_id",
//!     "expected integer, got string",
//! );
//!
//! // Converts to HTTP 400 Bad Request
//! assert_eq!(err.status_code(), http::StatusCode::BAD_REQUEST);
//! ```

#![doc(html_root_url = "https://docs.rs/archimedes-extract/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod body;
mod context;
mod error;
mod extractor;
mod form;
mod header;
mod inject;
mod json;
mod path;
mod query;
pub mod response;

// Re-export main types
pub use body::{BodyString, RawBody};
pub use context::ExtractionContext;
pub use error::{ExtractionError, ExtractionSource};
pub use extractor::FromRequest;
pub use form::{Form, FormWithLimit};
pub use header::{header, header_opt, ExtractTypedHeader, Header, Headers, TypedHeader};
pub use header::{Accept, Authorization, ContentType, UserAgent};
pub use inject::Inject;
pub use json::{Json, JsonWithLimit};
pub use path::{path_param, Path};
pub use query::{Query, RawQuery};

// Re-export useful types from dependencies
pub use archimedes_router::Params;
