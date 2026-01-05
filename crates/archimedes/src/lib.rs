//! # Archimedes
//!
//! **Async HTTP/gRPC/GraphQL Server Framework for the Themis Platform**
//!
//! Archimedes is an opinionated Rust-based server framework that provides:
//!
//! - 🔒 **Contract-First Enforcement** – Validate all requests/responses against Themis contracts
//! - 🛡️ **Built-in Authorization** – Embedded OPA evaluator for Eunomia policies
//! - 📊 **First-Class Observability** – OpenTelemetry traces, metrics, and structured logs
//! - ⚡ **High Performance** – Async Rust with zero-cost abstractions
//! - 🔗 **Mandatory Middleware** – Core middleware cannot be disabled or reordered
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use archimedes::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let server = Server::builder()
//!         .bind("0.0.0.0:8080")
//!         .build()
//!         .await?;
//!
//!     server.serve().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! Archimedes enforces a fixed middleware pipeline that cannot be disabled or reordered:
//!
//! ```text
//! Request → RequestId → Tracing → Identity → AuthZ → Validation → Handler
//!                                                                    ↓
//! Response ← ErrorNorm ← Telemetry ← ResponseValidation ←───────────┘
//! ```

#![doc(html_root_url = "https://docs.rs/archimedes/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

// Re-export core types
pub use archimedes_core as core;

// Re-export server types
pub use archimedes_server as server;

// Re-export middleware types
pub use archimedes_middleware as middleware;

// Re-export router types
pub use archimedes_router as router;

// Re-export extraction types
pub use archimedes_extract as extract;

/// Prelude module for convenient imports.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes::prelude::*;
/// ```
pub mod prelude {
    pub use archimedes_core::{
        CallerIdentity, Handler, RequestContext, RequestId, ThemisError, ThemisResult,
    };

    // Re-export common extractors
    pub use archimedes_extract::{
        Form, Header, Headers, Json, JsonWithLimit, Path, Query, RawQuery,
    };

    // Re-export common response builders
    pub use archimedes_extract::response::{
        ErrorResponse, HtmlResponse, JsonResponse, NoContent, Redirect, TextResponse,
    };
}
