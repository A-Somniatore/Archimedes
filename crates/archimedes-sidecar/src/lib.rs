//! Archimedes Sidecar - Multi-language service proxy
//!
//! The Archimedes sidecar is a standalone binary that provides all Archimedes middleware
//! functionality to services written in any language (Python, Go, TypeScript, C++, etc.).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                           Kubernetes Pod                                 │
//! │                                                                          │
//! │  ┌──────────────────────┐          ┌──────────────────────────────────┐ │
//! │  │  Archimedes Sidecar  │          │      Application Service         │ │
//! │  │                      │          │   (Python/Go/TypeScript/C++)     │ │
//! │  │  ┌────────────────┐  │   HTTP   │                                  │ │
//! │  │  │ Request ID     │  │ ───────► │  - Business logic only           │ │
//! │  │  │ Identity       │  │          │  - No middleware concerns        │ │
//! │  │  │ Authorization  │  │ ◄─────── │  - No contract validation        │ │
//! │  │  │ Validation     │  │          │  - No telemetry setup            │ │
//! │  │  │ Telemetry      │  │          │                                  │ │
//! │  │  └────────────────┘  │          └──────────────────────────────────┘ │
//! │  └──────────────────────┘                                                │
//! │           ▲                                                              │
//! │           │ HTTPS/mTLS                                                   │
//! └───────────┼──────────────────────────────────────────────────────────────┘
//!             │
//!     ┌───────┴───────┐
//!     │   Ingress     │
//!     └───────────────┘
//! ```
//!
//! # Features
//!
//! - **Middleware Pipeline**: Full Archimedes middleware (request ID, tracing, identity,
//!   authorization, validation)
//! - **Contract Validation**: Request/response validation against Themis contracts
//! - **Policy Evaluation**: Authorization via embedded OPA with Eunomia policies
//! - **Telemetry**: Automatic metrics, traces, and structured logging
//! - **Hot Reload**: Configuration, contracts, and policies can be reloaded at runtime
//!
//! # Example Usage
//!
//! ```bash
//! # Run the sidecar with a configuration file
//! $ archimedes-sidecar --config /etc/archimedes/sidecar.toml
//!
//! # Run with environment variable overrides
//! $ ARCHIMEDES_SIDECAR_LISTEN_PORT=8080 \
//!   ARCHIMEDES_SIDECAR_UPSTREAM_URL=http://localhost:3000 \
//!   archimedes-sidecar
//! ```

#![doc(html_root_url = "https://docs.rs/archimedes-sidecar/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod health;
pub mod headers;
pub mod proxy;
pub mod server;

pub use config::{SidecarConfig, SidecarConfigBuilder};
pub use error::{SidecarError, SidecarResult};
pub use health::{HealthChecker, HealthStatus, ReadinessStatus};
pub use proxy::{ProxyClient, ProxyRequest, ProxyResponse};
pub use server::SidecarServer;

/// Sidecar version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_exports() {
        // Verify all public types are accessible
        let _config = SidecarConfig::default();
    }
}
