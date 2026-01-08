//! Core middleware stages.
//!
//! This module contains the implementations of the middleware stages in
//! Archimedes. Core stages execute in a fixed order and cannot be disabled
//! or reordered.
//!
//! ## CORS Stage (Stage 0 - Optional)
//!
//! 0. [`cors`] - Handle CORS preflight and add headers
//!
//! ## Pre-Handler Stages (1-5)
//!
//! 1. [`request_id`] - Generate/propagate request ID
//! 2. [`tracing`] - Initialize OpenTelemetry span
//! 3. [`identity`] - Extract caller identity
//! 4. [`authorization`] - OPA policy evaluation
//! 5. [`validation`] - Request validation
//! 6. [`rate_limit`] - Rate limiting (optional)
//!
//! ## Post-Handler Stages (7-9)
//!
//! 7. [`validation`] - Response validation (via `ResponseValidationMiddleware`)
//! 8. [`telemetry`] - Emit metrics and logs
//! 9. [`error_normalization`] - Error envelope conversion

pub mod authorization;
pub mod cors;
pub mod error_normalization;
pub mod identity;
pub mod rate_limit;
pub mod request_id;
pub mod telemetry;
pub mod tracing;
pub mod validation;

// Re-export main types
pub use authorization::{
    AuthorizationMiddleware, AuthorizationResult, PolicyDecision, PolicyEvaluator, RbacBuilder,
};
pub use cors::{AllowedOrigins, CorsBuilder, CorsConfig, CorsMiddleware};
pub use error_normalization::{ErrorNormalizationMiddleware, NormalizedError};
pub use identity::IdentityMiddleware;
pub use rate_limit::{KeyExtractor, RateLimitBuilder, RateLimitConfig, RateLimitMiddleware};
pub use request_id::RequestIdMiddleware;
pub use telemetry::{TelemetryBuilder, TelemetryData, TelemetryMiddleware};
pub use tracing::{SpanInfo, TraceContext, TracingMiddleware};
pub use validation::{
    FieldType, MockSchema, MockSchemaBuilder, RequestBody, ResponseValidationMiddleware,
    ResponseValidationResult, ValidationBuilder, ValidationError, ValidationMiddleware,
    ValidationResult,
};
