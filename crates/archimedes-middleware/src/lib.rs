//! # Archimedes Middleware
//!
//! Middleware pipeline implementation for the Archimedes framework.
//!
//! This crate provides the fixed-order middleware pipeline that processes
//! all requests in Archimedes. The middleware order is immutable and cannot
//! be changed by users, ensuring consistent behavior across all services.
//!
//! ## Pipeline Stages
//!
//! ```text
//! Request → RequestId → Tracing → Identity → AuthZ → Validation → Handler
//!                                                                    ↓
//! Response ← ErrorNorm ← Telemetry ← ResponseValidation ←───────────┘
//! ```
//!
//! The pipeline has 8 fixed stages:
//!
//! | Stage | Middleware          | Purpose                                 |
//! |-------|--------------------|-----------------------------------------|
//! | 1     | Request ID          | Generate/propagate request ID (UUID v7) |
//! | 2     | Tracing             | Initialize OpenTelemetry span           |
//! | 3     | Identity            | Extract caller identity (SPIFFE/JWT)    |
//! | 4     | Authorization       | OPA policy evaluation                   |
//! | 5     | Request Validation  | Validate against contract schema        |
//! | 6     | Response Validation | Validate response (configurable)        |
//! | 7     | Telemetry           | Emit metrics and structured logs        |
//! | 8     | Error Normalization | Convert errors to standard envelope     |
//!
//! ## Key Features
//!
//! - **Fixed Order**: Core middleware cannot be reordered or disabled
//! - **Extension Points**: Optional `pre_handler` and `post_handler` hooks
//! - **Type Safety**: Middleware receives strongly-typed context
//! - **Async**: All middleware is fully async using Tokio
//!
//! ## Example
//!
//! ```
//! use archimedes_middleware::pipeline::{Pipeline, Stage};
//! use archimedes_middleware::context::MiddlewareContext;
//!
//! // Pipeline stages are fixed
//! let stages = Stage::all();
//! assert_eq!(stages.len(), 8);
//! assert_eq!(stages[0].name(), "request_id");
//! assert_eq!(stages[7].name(), "error_normalization");
//! ```
//!
//! ## Extension Points
//!
//! While the core pipeline is fixed, users can add hooks at two points:
//!
//! - `pre_handler`: After identity extraction, before authorization
//! - `post_handler`: After handler execution, before response validation
//!
//! These hooks cannot modify the pipeline order or suppress core middleware.

#![doc(html_root_url = "https://docs.rs/archimedes-middleware/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod context;
pub mod middleware;
pub mod pipeline;
pub mod types;

// Re-export main types at crate root
pub use context::MiddlewareContext;
pub use middleware::{BoxFuture, FnMiddleware, Middleware, Next};
pub use pipeline::{HookError, Pipeline, PipelineBuilder, Stage};
pub use types::{Request, Response, ResponseExt};
