//! # Archimedes Core
//!
//! Core types and traits for the Archimedes server framework.
//!
//! This crate provides the foundational types used throughout Archimedes:
//!
//! - [`RequestContext`] - Per-request context carrying identity, tracing, and metadata
//! - [`RequestId`] - UUID v7 request identifier (from `themis-platform-types`)
//! - [`CallerIdentity`] - Authenticated caller identity (from `themis-platform-types`)
//! - [`ThemisError`] - Standard error types
//! - [`Handler`] - Core handler trait
//! - [`Contract`] - Mock contract type for parallel development
//! - [`Operation`] - API operation definition
//! - [`MockSchema`] - Request/response schema validation
//!
//! ## Shared Platform Types
//!
//! Archimedes uses types from `themis-platform-types` for cross-component compatibility:
//!
//! - `CallerIdentity` - Shared caller identity type
//! - `RequestId` - Shared request identifier
//! - `ThemisErrorEnvelope` - Shared error response format
//! - `PolicyInput` / `PolicyDecision` - Policy evaluation types (Phase A5)
//!
//! ## Mock Contracts for Parallel Development
//!
//! Archimedes can be developed in parallel with Themis using mock contracts.
//! The [`contract`] module provides types that simulate Themis contract behavior:
//!
//! ```rust
//! use archimedes_core::contract::{Contract, Operation, MockSchema};
//! use http::Method;
//!
//! // Define a mock contract for testing
//! let contract = Contract::builder("my-service")
//!     .version("1.0.0")
//!     .operation(
//!         Operation::builder("getUser")
//!             .method(Method::GET)
//!             .path("/users/{userId}")
//!             .response_schema(MockSchema::object(vec![
//!                 ("id", MockSchema::string().required()),
//!                 ("name", MockSchema::string().required()),
//!             ]))
//!             .build()
//!     )
//!     .build();
//!
//! // Route matching
//! let (op, params) = contract.match_operation(&Method::GET, "/users/123").unwrap();
//! assert_eq!(op.operation_id(), "getUser");
//! assert_eq!(params.get("userId").unwrap(), "123");
//! ```
//!
//! ## Test Fixtures
//!
//! The [`fixtures`] module provides pre-built contracts for testing:
//!
//! ```rust
//! use archimedes_core::fixtures;
//!
//! // Pre-built user service contract
//! let contract = fixtures::user_service_contract();
//!
//! // Pre-built health check contract (no auth required)
//! let health = fixtures::health_contract();
//! ```

#![doc(html_root_url = "https://docs.rs/archimedes-core/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod binder;
mod context;
pub mod contract;
pub mod di;
mod error;
pub mod fixtures;
pub mod handler;
mod identity;
mod invocation;

// Re-export shared types from themis-platform-types
pub use themis_platform_types::{
    // Re-export sub-types for convenience
    identity::{ApiKeyIdentity, SpiffeIdentity, UserIdentity},
    CallerIdentity,
    RequestId,
};

// Re-export local types
pub use binder::{BinderError, BinderResult, HandlerBinder};
pub use context::RequestContext;
pub use contract::{Contract, MockSchema, Operation, ValidationError};
pub use error::{ErrorCategory, ErrorDetail, ErrorEnvelope, ThemisError, ThemisResult};
pub use handler::Handler;
pub use invocation::{InvocationContext, InvocationContextBuilder};

// Keep local identity module for Archimedes-specific extensions
pub use identity::CallerIdentityExt;
