//! Procedural macros for Archimedes handlers.
//!
//! This crate provides attribute macros that enable FastAPI-style handler definitions,
//! reducing boilerplate while maintaining type safety and contract compliance.
//!
//! # Overview
//!
//! The `#[handler]` attribute macro transforms async functions into properly registered
//! handlers that integrate with Archimedes's request pipeline.
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes::prelude::*;
//!
//! #[archimedes::handler(operation = "createUser")]
//! async fn create_user(
//!     db: Inject<Database>,
//!     body: Json<CreateUserRequest>,
//! ) -> Result<Json<User>, AppError> {
//!     let user = db.create_user(body.0).await?;
//!     Ok(Json(user))
//! }
//! ```
//!
//! # Macro Expansion
//!
//! The `#[handler]` macro:
//!
//! 1. Parses the function signature to identify extractors
//! 2. Generates extraction code for each parameter
//! 3. Wraps the function body in proper error handling
//! 4. Creates a registration entry for the handler registry
//!
//! # Design Principles
//!
//! - **Type Safety**: All extractors are validated at compile time
//! - **Contract Binding**: Handlers are bound to specific operation IDs
//! - **Dependency Injection**: `Inject<T>` provides access to shared services
//! - **Automatic Extraction**: Parameters are extracted based on their types

mod handler;
mod parse;

use proc_macro::TokenStream;

/// Marks a function as an Archimedes handler.
///
/// This attribute macro transforms an async function into a handler that can be
/// registered with Archimedes's handler registry. The macro handles:
///
/// - Automatic parameter extraction using the `FromRequest` trait
/// - Dependency injection via `Inject<T>`
/// - Response serialization
/// - Error handling and conversion
///
/// # Attributes
///
/// - `operation`: The operation ID from the contract (required)
/// - `method`: HTTP method override (optional, defaults to contract)
/// - `path`: Path override (optional, defaults to contract)
///
/// # Example
///
/// ```rust,ignore
/// use archimedes::prelude::*;
///
/// #[archimedes::handler(operation = "getUser")]
/// async fn get_user(
///     Path(user_id): Path<UserId>,
///     db: Inject<Database>,
/// ) -> Result<Json<User>, AppError> {
///     let user = db.get_user(user_id).await?;
///     Ok(Json(user))
/// }
/// ```
///
/// # Generated Code
///
/// The macro generates approximately:
///
/// ```rust,ignore
/// fn __archimedes_handler_get_user() -> HandlerRegistration {
///     HandlerRegistration {
///         operation_id: "getUser",
///         handler: |ctx, body| Box::pin(async move {
///             let path: Path<UserId> = Path::from_request(&ctx)?;
///             let db: Inject<Database> = Inject::from_container(&ctx)?;
///             get_user(path, db).await
///         }),
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    handler::expand_handler(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Marks a struct as injectable via dependency injection.
///
/// Types marked with `#[injectable]` can be used with `Inject<T>` in handlers.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes::prelude::*;
///
/// #[injectable]
/// struct Database {
///     pool: PgPool,
/// }
///
/// #[archimedes::handler(operation = "getUser")]
/// async fn get_user(db: Inject<Database>) -> Result<Json<User>, AppError> {
///     // db is automatically injected
/// }
/// ```
#[proc_macro_attribute]
pub fn injectable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // For now, injectable is a marker - the actual DI logic is in archimedes-core
    item
}
