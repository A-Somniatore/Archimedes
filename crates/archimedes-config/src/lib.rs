//! Typed configuration system for Archimedes.
//!
//! This crate provides a strongly-typed configuration system for Archimedes servers
//! with support for:
//! - TOML and JSON configuration files
//! - Environment variable overrides
//! - Strict validation (fails on unknown fields)
//! - Layered configuration (defaults → file → env)
//!
//! # Overview
//!
//! The configuration system is built around the [`ArchimedesConfig`] struct, which
//! contains all the configuration options for an Archimedes server:
//!
//! - [`ServerConfig`] - HTTP server settings (address, timeouts, etc.)
//! - [`TelemetryConfig`] - Observability settings (metrics, tracing, logging)
//! - [`AuthorizationConfig`] - Authorization settings (OPA endpoint, etc.)
//! - [`ContractConfig`] - Contract validation settings
//!
//! # Example
//!
//! ```no_run
//! use archimedes_config::{ArchimedesConfig, ConfigLoader};
//!
//! # fn main() -> Result<(), archimedes_config::ConfigError> {
//! // Load configuration with layered approach
//! let config = ConfigLoader::new()
//!     .with_defaults()
//!     .with_file("config.toml")?
//!     .with_env_prefix("ARCHIMEDES")
//!     .load()?;
//!
//! println!("Server will listen on: {}", config.server.http_addr);
//! # Ok(())
//! # }
//! ```
//!
//! # Configuration File Format
//!
//! ```toml
//! [server]
//! http_addr = "0.0.0.0:8080"
//! shutdown_timeout_secs = 30
//! max_connections = 10000
//! request_timeout_ms = 30000
//!
//! [telemetry]
//! service_name = "my-service"
//! service_version = "1.0.0"
//! environment = "production"
//!
//! [telemetry.metrics]
//! enabled = true
//! addr = "0.0.0.0:9090"
//!
//! [telemetry.tracing]
//! enabled = true
//! otlp_endpoint = "http://localhost:4317"
//! sampling_ratio = 1.0
//!
//! [telemetry.logging]
//! enabled = true
//! level = "info"
//! format = "json"
//!
//! [authorization]
//! enabled = true
//! mode = "rbac"
//!
//! [contract]
//! enabled = true
//! strict_validation = true
//! ```
//!
//! # Environment Variable Overrides
//!
//! All configuration values can be overridden via environment variables using
//! the format `PREFIX__SECTION__KEY`. For example:
//!
//! - `ARCHIMEDES__SERVER__HTTP_ADDR=0.0.0.0:9000`
//! - `ARCHIMEDES__TELEMETRY__SERVICE_NAME=my-service`
//! - `ARCHIMEDES__TELEMETRY__METRICS__ENABLED=false`

#![warn(missing_docs)]

mod config;
mod error;
mod loader;
mod schema;
mod watcher;

pub use config::*;
pub use error::ConfigError;
pub use loader::ConfigLoader;
pub use schema::*;
pub use watcher::{FileChangeEvent, FileChangeKind, FileWatcher, FileWatcherConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ArchimedesConfig::default();
        assert_eq!(config.server.http_addr, "0.0.0.0:8080");
        assert_eq!(config.server.shutdown_timeout_secs, 30);
    }

    #[test]
    fn test_config_builder() {
        let config = ArchimedesConfig::builder()
            .server(ServerConfig {
                http_addr: "127.0.0.1:3000".to_string(),
                ..Default::default()
            })
            .build();

        assert_eq!(config.server.http_addr, "127.0.0.1:3000");
    }
}
