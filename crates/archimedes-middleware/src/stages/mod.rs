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
//! 4. `authorization` - OPA policy evaluation (Week 11)
//! 5. `validation` - Request validation (Week 11)
//!
//! ## Post-Handler Stages (6-8)
//!
//! 6. `response_validation` - Response validation (Week 12)
//! 7. `telemetry` - Emit metrics and logs (Week 12)
//! 8. `error_normalization` - Error envelope conversion (Week 12)

pub mod identity;
pub mod request_id;
pub mod tracing;

// Re-export main types
pub use identity::IdentityMiddleware;
pub use request_id::RequestIdMiddleware;
pub use tracing::TracingMiddleware;
