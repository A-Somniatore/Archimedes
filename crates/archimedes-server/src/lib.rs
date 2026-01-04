//! # Archimedes Server
//!
//! HTTP/gRPC server implementation for the Archimedes framework.
//!
//! This crate provides the server infrastructure for Archimedes:
//!
//! - HTTP/1.1 and HTTP/2 support via Hyper
//! - Request routing
//! - Graceful shutdown
//! - Health check endpoints
//!
//! ## Example
//!
//! ```rust,ignore
//! use archimedes_server::Server;
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

#![doc(html_root_url = "https://docs.rs/archimedes-server/0.1.0")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

// Server module will be implemented in Phase A2
// For now, we just expose the crate structure

/// Server placeholder - to be implemented in Week 5-8
pub struct Server;

impl Server {
    /// Creates a new server builder.
    #[must_use]
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}

/// Builder for configuring and creating a server.
pub struct ServerBuilder {
    bind_addr: String,
}

impl ServerBuilder {
    /// Creates a new server builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bind_addr: "0.0.0.0:8080".to_string(),
        }
    }

    /// Sets the address to bind the server to.
    #[must_use]
    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        self.bind_addr = addr.into();
        self
    }

    /// Returns the configured bind address.
    #[must_use]
    pub fn bind_addr(&self) -> &str {
        &self.bind_addr
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_builder_default() {
        let builder = ServerBuilder::new();
        assert_eq!(builder.bind_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn test_server_builder_custom_bind() {
        let builder = ServerBuilder::new().bind("127.0.0.1:3000");
        assert_eq!(builder.bind_addr(), "127.0.0.1:3000");
    }
}
