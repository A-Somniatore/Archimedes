//! Server configuration types.
//!
//! This module provides configuration types for the Archimedes server,
//! using the builder pattern for ergonomic construction.
//!
//! # Example
//!
//! ```rust
//! use archimedes_server::ServerConfig;
//! use std::time::Duration;
//!
//! let config = ServerConfig::builder()
//!     .http_addr("0.0.0.0:8080")
//!     .shutdown_timeout(Duration::from_secs(30))
//!     .build();
//!
//! assert_eq!(config.http_addr(), "0.0.0.0:8080");
//! ```

use std::net::SocketAddr;
use std::time::Duration;

/// Default HTTP bind address.
pub const DEFAULT_HTTP_ADDR: &str = "0.0.0.0:8080";

/// Default shutdown timeout in seconds.
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;

/// Default keep-alive timeout in seconds.
pub const DEFAULT_KEEP_ALIVE_SECS: u64 = 75;

/// Server configuration.
///
/// Contains all settings needed to configure the HTTP server.
/// Use [`ServerConfig::builder()`] to construct instances.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// HTTP server bind address (e.g., "0.0.0.0:8080")
    http_addr: String,

    /// Timeout for graceful shutdown (how long to wait for in-flight requests)
    shutdown_timeout: Duration,

    /// TCP keep-alive timeout
    keep_alive_timeout: Option<Duration>,

    /// Maximum concurrent connections (None = unlimited)
    max_connections: Option<usize>,

    /// Whether to enable HTTP/2 (default: true)
    http2_enabled: bool,
}

impl ServerConfig {
    /// Creates a new server configuration builder.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ServerConfig;
    ///
    /// let config = ServerConfig::builder()
    ///     .http_addr("127.0.0.1:3000")
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }

    /// Returns the HTTP bind address.
    #[must_use]
    pub fn http_addr(&self) -> &str {
        &self.http_addr
    }

    /// Parses and returns the HTTP address as a `SocketAddr`.
    ///
    /// # Errors
    ///
    /// Returns an error if the address cannot be parsed.
    pub fn socket_addr(&self) -> Result<SocketAddr, std::net::AddrParseError> {
        self.http_addr.parse()
    }

    /// Returns the graceful shutdown timeout.
    #[must_use]
    pub fn shutdown_timeout(&self) -> Duration {
        self.shutdown_timeout
    }

    /// Returns the TCP keep-alive timeout, if configured.
    #[must_use]
    pub fn keep_alive_timeout(&self) -> Option<Duration> {
        self.keep_alive_timeout
    }

    /// Returns the maximum number of concurrent connections, if configured.
    #[must_use]
    pub fn max_connections(&self) -> Option<usize> {
        self.max_connections
    }

    /// Returns whether HTTP/2 is enabled.
    #[must_use]
    pub fn http2_enabled(&self) -> bool {
        self.http2_enabled
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Builder for [`ServerConfig`].
///
/// Provides a fluent interface for constructing server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfigBuilder {
    http_addr: String,
    shutdown_timeout: Duration,
    keep_alive_timeout: Option<Duration>,
    max_connections: Option<usize>,
    http2_enabled: bool,
}

impl ServerConfigBuilder {
    /// Creates a new builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            http_addr: DEFAULT_HTTP_ADDR.to_string(),
            shutdown_timeout: Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS),
            keep_alive_timeout: Some(Duration::from_secs(DEFAULT_KEEP_ALIVE_SECS)),
            max_connections: None,
            http2_enabled: true,
        }
    }

    /// Sets the HTTP bind address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to bind to (e.g., "0.0.0.0:8080", "127.0.0.1:3000")
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ServerConfigBuilder;
    ///
    /// let builder = ServerConfigBuilder::new()
    ///     .http_addr("0.0.0.0:9090");
    /// ```
    #[must_use]
    pub fn http_addr(mut self, addr: impl Into<String>) -> Self {
        self.http_addr = addr.into();
        self
    }

    /// Sets the graceful shutdown timeout.
    ///
    /// This is the maximum time the server will wait for in-flight
    /// requests to complete during shutdown.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Duration to wait for graceful shutdown
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ServerConfigBuilder;
    /// use std::time::Duration;
    ///
    /// let builder = ServerConfigBuilder::new()
    ///     .shutdown_timeout(Duration::from_secs(60));
    /// ```
    #[must_use]
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Sets the TCP keep-alive timeout.
    ///
    /// Set to `None` to disable keep-alive.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional duration for keep-alive, or None to disable
    #[must_use]
    pub fn keep_alive_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.keep_alive_timeout = timeout;
        self
    }

    /// Sets the maximum number of concurrent connections.
    ///
    /// Set to `None` for unlimited connections (default).
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum number of concurrent connections
    #[must_use]
    pub fn max_connections(mut self, max: Option<usize>) -> Self {
        self.max_connections = max;
        self
    }

    /// Enables or disables HTTP/2 support.
    ///
    /// HTTP/2 is enabled by default.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable HTTP/2
    #[must_use]
    pub fn http2_enabled(mut self, enabled: bool) -> Self {
        self.http2_enabled = enabled;
        self
    }

    /// Builds the [`ServerConfig`] with the configured values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ServerConfig;
    ///
    /// let config = ServerConfig::builder()
    ///     .http_addr("0.0.0.0:8080")
    ///     .build();
    /// ```
    #[must_use]
    pub fn build(self) -> ServerConfig {
        ServerConfig {
            http_addr: self.http_addr,
            shutdown_timeout: self.shutdown_timeout,
            keep_alive_timeout: self.keep_alive_timeout,
            max_connections: self.max_connections,
            http2_enabled: self.http2_enabled,
        }
    }
}

impl Default for ServerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();

        assert_eq!(config.http_addr(), DEFAULT_HTTP_ADDR);
        assert_eq!(
            config.shutdown_timeout(),
            Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS)
        );
        assert_eq!(
            config.keep_alive_timeout(),
            Some(Duration::from_secs(DEFAULT_KEEP_ALIVE_SECS))
        );
        assert!(config.max_connections().is_none());
        assert!(config.http2_enabled());
    }

    #[test]
    fn test_builder_http_addr() {
        let config = ServerConfig::builder()
            .http_addr("127.0.0.1:3000")
            .build();

        assert_eq!(config.http_addr(), "127.0.0.1:3000");
    }

    #[test]
    fn test_builder_shutdown_timeout() {
        let config = ServerConfig::builder()
            .shutdown_timeout(Duration::from_secs(60))
            .build();

        assert_eq!(config.shutdown_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_builder_keep_alive_disabled() {
        let config = ServerConfig::builder()
            .keep_alive_timeout(None)
            .build();

        assert!(config.keep_alive_timeout().is_none());
    }

    #[test]
    fn test_builder_max_connections() {
        let config = ServerConfig::builder()
            .max_connections(Some(1000))
            .build();

        assert_eq!(config.max_connections(), Some(1000));
    }

    #[test]
    fn test_builder_http2_disabled() {
        let config = ServerConfig::builder()
            .http2_enabled(false)
            .build();

        assert!(!config.http2_enabled());
    }

    #[test]
    fn test_socket_addr_parsing() {
        let config = ServerConfig::builder()
            .http_addr("127.0.0.1:8080")
            .build();

        let addr = config.socket_addr().unwrap();
        assert_eq!(addr.port(), 8080);
        assert!(addr.ip().is_loopback());
    }

    #[test]
    fn test_socket_addr_invalid() {
        let config = ServerConfig::builder()
            .http_addr("not-a-valid-address")
            .build();

        assert!(config.socket_addr().is_err());
    }

    #[test]
    fn test_builder_chaining() {
        let config = ServerConfig::builder()
            .http_addr("0.0.0.0:9090")
            .shutdown_timeout(Duration::from_secs(45))
            .keep_alive_timeout(Some(Duration::from_secs(120)))
            .max_connections(Some(500))
            .http2_enabled(true)
            .build();

        assert_eq!(config.http_addr(), "0.0.0.0:9090");
        assert_eq!(config.shutdown_timeout(), Duration::from_secs(45));
        assert_eq!(
            config.keep_alive_timeout(),
            Some(Duration::from_secs(120))
        );
        assert_eq!(config.max_connections(), Some(500));
        assert!(config.http2_enabled());
    }

    #[test]
    fn test_config_clone() {
        let config1 = ServerConfig::builder()
            .http_addr("192.168.1.1:8080")
            .build();

        let config2 = config1.clone();

        assert_eq!(config1.http_addr(), config2.http_addr());
        assert_eq!(config1.shutdown_timeout(), config2.shutdown_timeout());
    }
}
