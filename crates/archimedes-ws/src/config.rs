//! WebSocket configuration.
//!
//! This module defines configuration options for WebSocket connections
//! and the connection manager.

use std::time::Duration;

/// Configuration for a WebSocket connection.
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Maximum message size in bytes (default: 64 MB).
    pub max_message_size: usize,
    /// Maximum frame size in bytes (default: 16 MB).
    pub max_frame_size: usize,
    /// Heartbeat interval for ping frames (default: 30 seconds).
    pub heartbeat_interval: Duration,
    /// Connection timeout - close if no pong received (default: 60 seconds).
    pub connection_timeout: Duration,
    /// Write buffer size (default: 128 KB).
    pub write_buffer_size: usize,
    /// Read buffer size (default: 128 KB).
    pub read_buffer_size: usize,
    /// Whether to accept unmasked frames from clients (default: false).
    pub accept_unmasked_frames: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024 * 1024,  // 64 MB
            max_frame_size: 16 * 1024 * 1024,    // 16 MB
            heartbeat_interval: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(60),
            write_buffer_size: 128 * 1024,       // 128 KB
            read_buffer_size: 128 * 1024,        // 128 KB
            accept_unmasked_frames: false,
        }
    }
}

impl WebSocketConfig {
    /// Create a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum message size.
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Set the maximum frame size.
    pub fn max_frame_size(mut self, size: usize) -> Self {
        self.max_frame_size = size;
        self
    }

    /// Set the heartbeat interval.
    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// Set the connection timeout.
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set the write buffer size.
    pub fn write_buffer_size(mut self, size: usize) -> Self {
        self.write_buffer_size = size;
        self
    }

    /// Set the read buffer size.
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.read_buffer_size = size;
        self
    }

    /// Set whether to accept unmasked frames from clients.
    pub fn accept_unmasked_frames(mut self, accept: bool) -> Self {
        self.accept_unmasked_frames = accept;
        self
    }
}

/// Configuration for the connection manager.
#[derive(Debug, Clone)]
pub struct ConnectionManagerConfig {
    /// Maximum total connections (default: 10000).
    pub max_connections: usize,
    /// Maximum connections per client identifier (default: 100).
    pub max_per_client: usize,
    /// Idle connection timeout (default: 5 minutes).
    pub idle_timeout: Duration,
    /// How often to run the cleanup task (default: 30 seconds).
    pub cleanup_interval: Duration,
}

impl Default for ConnectionManagerConfig {
    fn default() -> Self {
        Self {
            max_connections: 10_000,
            max_per_client: 100,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(30),
        }
    }
}

impl ConnectionManagerConfig {
    /// Create a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum total connections.
    pub fn max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Set the maximum connections per client.
    pub fn max_per_client(mut self, max: usize) -> Self {
        self.max_per_client = max;
        self
    }

    /// Set the idle connection timeout.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set the cleanup interval.
    pub fn cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.max_message_size, 64 * 1024 * 1024);
        assert_eq!(config.max_frame_size, 16 * 1024 * 1024);
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.connection_timeout, Duration::from_secs(60));
        assert!(!config.accept_unmasked_frames);
    }

    #[test]
    fn test_websocket_config_builder() {
        let config = WebSocketConfig::new()
            .max_message_size(1024)
            .max_frame_size(512)
            .heartbeat_interval(Duration::from_secs(10))
            .connection_timeout(Duration::from_secs(20))
            .accept_unmasked_frames(true);

        assert_eq!(config.max_message_size, 1024);
        assert_eq!(config.max_frame_size, 512);
        assert_eq!(config.heartbeat_interval, Duration::from_secs(10));
        assert_eq!(config.connection_timeout, Duration::from_secs(20));
        assert!(config.accept_unmasked_frames);
    }

    #[test]
    fn test_connection_manager_config_default() {
        let config = ConnectionManagerConfig::default();
        assert_eq!(config.max_connections, 10_000);
        assert_eq!(config.max_per_client, 100);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.cleanup_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_connection_manager_config_builder() {
        let config = ConnectionManagerConfig::new()
            .max_connections(5000)
            .max_per_client(50)
            .idle_timeout(Duration::from_secs(600))
            .cleanup_interval(Duration::from_secs(60));

        assert_eq!(config.max_connections, 5000);
        assert_eq!(config.max_per_client, 50);
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
    }
}
