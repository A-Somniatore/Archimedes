//! # Archimedes Server
//!
//! HTTP/gRPC server implementation for the Archimedes framework.
//!
//! This crate provides the server infrastructure for Archimedes:
//!
//! - HTTP/1.1 and HTTP/2 support via Hyper
//! - Request routing with contract-based path resolution
//! - Graceful shutdown with configurable timeout
//! - Health check endpoints (`/health`, `/ready`)
//!
//! ## Example
//!
//! ```rust,ignore
//! use archimedes_server::{Server, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::builder()
//!         .http_addr("0.0.0.0:8080")
//!         .build();
//!
//!     let server = Server::new(config);
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Graceful Shutdown
//!
//! The server handles shutdown signals (SIGTERM, SIGINT) gracefully:
//!
//! 1. Stop accepting new connections
//! 2. Wait for in-flight requests to complete (with timeout)
//! 3. Close all connections
//!
//! ```rust,ignore
//! use archimedes_server::Server;
//! use std::time::Duration;
//!
//! let server = Server::builder()
//!     .shutdown_timeout(Duration::from_secs(30))
//!     .build();
//! ```

#![doc(html_root_url = "https://docs.rs/archimedes-server/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod config;
pub mod handler;
mod health;
mod lifecycle;
mod router;
mod server;
pub mod shutdown;

pub use config::{ServerConfig, ServerConfigBuilder};
pub use handler::{HandlerError, HandlerRegistry, InvokeError};
pub use health::{HealthCheck, HealthStatus, ReadinessCheck, ReadinessStatus};
pub use lifecycle::{Lifecycle, LifecycleError, LifecycleHook, LifecycleResult};
pub use router::{RouteMatch, Router};
pub use server::{Server, ServerBuilder, ServerError};
pub use shutdown::ShutdownSignal;
