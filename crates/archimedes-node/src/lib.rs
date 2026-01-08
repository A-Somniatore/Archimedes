//! # Archimedes Node.js/TypeScript Bindings
//!
//! Native Node.js bindings for the Archimedes HTTP server framework using NAPI-RS.
//!
//! ## Example (TypeScript)
//!
//! ```typescript
//! import { Archimedes, Request, Response, Config } from '@archimedes/node';
//!
//! const config = new Config({
//!   contractPath: 'contract.json',
//!   listenPort: 8080,
//! });
//!
//! const app = new Archimedes(config);
//!
//! app.operation('listUsers', async (request: Request): Promise<Response> => {
//!   const users = await db.getUsers();
//!   return Response.json({ users });
//! });
//!
//! app.operation('getUser', async (request: Request): Promise<Response> => {
//!   const userId = request.pathParams.userId;
//!   const user = await db.getUser(userId);
//!   if (!user) {
//!     return Response.notFound({ error: 'User not found' });
//!   }
//!   return Response.json(user);
//! });
//!
//! app.listen(8080);
//! ```
//!
//! ## Features
//!
//! - Full Rust parity with archimedes-py
//! - TypeScript-first API with full type definitions
//! - Async/Promise-based handler registration
//! - Built-in middleware (request ID, tracing, identity, validation, authorization)
//! - Contract-based request/response validation via Sentinel
//! - OPA policy evaluation via Authorizer
//! - Prometheus metrics and OpenTelemetry tracing

// NAPI-RS has specific patterns that conflict with some clippy lints
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_const_for_fn)] // napi methods can't be const
#![allow(clippy::unused_async)] // napi async is required for JS Promise
#![allow(clippy::unused_self)] // napi methods need &self even when unused
#![allow(clippy::use_self)] // napi derives don't work with Self
#![allow(clippy::return_self_not_must_use)] // napi builders
#![allow(clippy::uninlined_format_args)] // Keep for clarity
#![allow(clippy::option_if_let_else)] // More readable with if-let
#![allow(clippy::cast_possible_truncation)] // We handle array lengths safely
#![allow(clippy::unsafe_derive_deserialize)] // NAPI requires certain unsafe
#![allow(clippy::struct_field_names)] // operation_id is a clear name

use napi_derive::napi;

mod authz;
mod config;
mod context;
mod error;
mod handlers;
mod middleware;
mod response;
mod server;
mod telemetry;
mod validation;

pub use authz::{Authorizer, AuthzInput, PolicyDecision};
pub use config::Config;
pub use context::{Identity, RequestContext};
pub use error::ArchimedesError;
pub use handlers::HandlerRegistry;
pub use middleware::{
    apply_all_middleware, apply_identity, apply_request_id, apply_tracing,
    default_middleware_config, get_middleware_summary, normalize_error_response_header,
    MiddlewareConfig, MiddlewareResultJs,
};
pub use response::Response;
pub use server::Server;
pub use telemetry::{Telemetry, TelemetryConfig};
pub use validation::{OperationResolution, Sentinel, ValidationError, ValidationResult};

/// Package version
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let v = version();
        assert!(!v.is_empty());
        assert!(v.starts_with("0."));
    }
}
