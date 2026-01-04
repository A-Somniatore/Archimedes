//! # Archimedes Core
//!
//! Core types and traits for the Archimedes server framework.
//!
//! This crate provides the foundational types used throughout Archimedes:
//!
//! - [`RequestContext`] - Per-request context carrying identity, tracing, and metadata
//! - [`RequestId`] - UUID v7 request identifier
//! - [`CallerIdentity`] - Authenticated caller identity (SPIFFE, User, ApiKey, Anonymous)
//! - [`ThemisError`] - Standard error types
//! - [`Handler`] - Core handler trait

#![doc(html_root_url = "https://docs.rs/archimedes-core/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod contract;
mod context;
mod error;
pub mod fixtures;
mod handler;
mod identity;

pub use context::{RequestContext, RequestId};
pub use contract::{Contract, MockSchema, Operation, ValidationError};
pub use error::{ErrorCategory, ThemisError, ThemisResult};
pub use handler::Handler;
pub use identity::CallerIdentity;
