//! Core middleware stages.
//!
//! This module contains the implementations of the 8 mandatory middleware
//! stages in Archimedes. These stages execute in a fixed order and cannot
//! be disabled or reordered.
//!
//! ## Pre-Handler Stages (1-5)
//!
//! 1. [`request_id`] - Generate/propagate request ID
//! 2. [`tracing`] - Initialize OpenTelemetry span
//! 3. [`identity`] - Extract caller identity
//! 4. [`authorization`] - OPA policy evaluation
//! 5. [`validation`] - Request validation
//!
//! ## Post-Handler Stages (6-8)
//!
//! 6. [`validation`] - Response validation (via `ResponseValidationMiddleware`)
//! 7. `telemetry` - Emit metrics and logs (Week 12)
//! 8. `error_normalization` - Error envelope conversion (Week 12)

pub mod authorization;
pub mod identity;
pub mod request_id;
pub mod tracing;
pub mod validation;

// Re-export main types
pub use authorization::{
    AuthorizationMiddleware, AuthorizationResult, PolicyDecision, PolicyEvaluator, RbacBuilder,
};
pub use identity::IdentityMiddleware;
pub use request_id::RequestIdMiddleware;
pub use tracing::{SpanInfo, TraceContext, TracingMiddleware};
pub use validation::{
    FieldType, MockSchema, MockSchemaBuilder, RequestBody, ResponseValidationMiddleware,
    ResponseValidationResult, ValidationBuilder, ValidationError, ValidationMiddleware,
    ValidationResult,
};
