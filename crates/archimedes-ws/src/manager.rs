//! WebSocket connection management.
//!
//! This module provides a connection manager that tracks active WebSocket
//! connections, enforces connection limits, and handles graceful shutdown.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::config::ConnectionManagerConfig;
use crate::connection::ConnectionId;
use crate::error::{WsError, WsResult};

/// The type of WebSocket connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// A standard WebSocket connection.
    WebSocket,
    /// A Server-Sent Events connection.
    ServerSentEvents,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WebSocket => write!(f, "WebSocket"),
            Self::ServerSentEvents => write!(f, "SSE"),
        }
    }
}

/// Information about a tracked connection.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// The unique connection ID.
    pub id: ConnectionId,
    /// The client identifier (e.g., user ID, IP address).
    pub client_id: Option<String>,
    /// When the connection was established.
    pub connected_at: Instant,
    /// Last activity time.
    pub last_activity: Instant,
    /// The type of connection.
    pub connection_type: ConnectionType,
    /// Optional metadata.
    pub metadata: Option<String>,
}

impl ConnectionInfo {
    /// Create a new connection info.
    pub fn new(id: ConnectionId, connection_type: ConnectionType) -> Self {
        let now = Instant::now();
        Self {
            id,
            client_id: None,
            connected_at: now,
            last_activity: now,
            connection_type,
            metadata: None,
        }
    }

    /// Create a new WebSocket connection info.
    pub fn websocket(id: ConnectionId) -> Self {
        Self::new(id, ConnectionType::WebSocket)
    }

    /// Create a new SSE connection info.
    pub fn sse(id: ConnectionId) -> Self {
        Self::new(id, ConnectionType::ServerSentEvents)
    }

    /// Set the client identifier.
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Update the last activity time.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get the connection duration.
    pub fn duration(&self) -> std::time::Duration {
        self.connected_at.elapsed()
    }

    /// Get the idle duration.
    pub fn idle_duration(&self) -> std::time::Duration {
        self.last_activity.elapsed()
    }
}

/// Statistics about the connection manager.
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Total number of active connections.
    pub active_connections: usize,
    /// Number of WebSocket connections.
    pub websocket_connections: usize,
    /// Number of SSE connections.
    pub sse_connections: usize,
    /// Total connections ever accepted.
    pub total_accepted: usize,
    /// Total connections rejected due to limits.
    pub total_rejected: usize,
    /// Total connections closed.
    pub total_closed: usize,
}

/// A manager for tracking WebSocket and SSE connections.
///
/// The connection manager enforces connection limits, tracks active
/// connections, and supports graceful shutdown by notifying all
/// connections to close.
///
/// # Example
///
/// ```
/// use archimedes_ws::{ConnectionManager, ConnectionManagerConfig, ConnectionType};
///
/// let config = ConnectionManagerConfig::default();
/// let manager = ConnectionManager::new(config);
///
/// // Track a new connection
/// let id = manager.accept(ConnectionType::WebSocket, None)?;
///
/// // Update activity
/// manager.touch(&id);
///
/// // Remove when done
/// manager.remove(&id);
/// # Ok::<(), archimedes_ws::WsError>(())
/// ```
pub struct ConnectionManager {
    /// Active connections.
    connections: DashMap<ConnectionId, ConnectionInfo>,
    /// Configuration.
    config: ConnectionManagerConfig,
    /// Total connections accepted.
    total_accepted: AtomicUsize,
    /// Total connections rejected.
    total_rejected: AtomicUsize,
    /// Total connections closed.
    total_closed: AtomicUsize,
    /// Shutdown signal.
    shutdown_tx: broadcast::Sender<()>,
    /// Whether shutdown has been triggered.
    is_shutdown: AtomicBool,
}

impl ConnectionManager {
    /// Create a new connection manager.
    pub fn new(config: ConnectionManagerConfig) -> Arc<Self> {
        let (shutdown_tx, _) = broadcast::channel(1);
        Arc::new(Self {
            connections: DashMap::new(),
            config,
            total_accepted: AtomicUsize::new(0),
            total_rejected: AtomicUsize::new(0),
            total_closed: AtomicUsize::new(0),
            shutdown_tx,
            is_shutdown: AtomicBool::new(false),
        })
    }

    /// Create a new connection manager with default configuration.
    pub fn default_manager() -> Arc<Self> {
        Self::new(ConnectionManagerConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &ConnectionManagerConfig {
        &self.config
    }

    /// Accept a new connection.
    ///
    /// This checks connection limits and registers the connection if allowed.
    ///
    /// # Arguments
    ///
    /// * `connection_type` - The type of connection.
    /// * `client_id` - Optional client identifier for per-client limits.
    ///
    /// # Returns
    ///
    /// The connection ID if accepted, or an error if limits are exceeded.
    pub fn accept(
        &self,
        connection_type: ConnectionType,
        client_id: Option<String>,
    ) -> WsResult<ConnectionId> {
        // Check if shutdown
        if self.is_shutdown.load(Ordering::SeqCst) {
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(WsError::connection_limit("server is shutting down"));
        }

        // Check global limit
        let current = self.connections.len();
        if current >= self.config.max_connections {
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            warn!(
                current = current,
                max = self.config.max_connections,
                "Connection limit reached"
            );
            return Err(WsError::connection_limit(format!(
                "maximum connections ({}) reached",
                self.config.max_connections
            )));
        }

        // Check per-client limit if client_id provided
        if let Some(ref client) = client_id {
            let client_count = self
                .connections
                .iter()
                .filter(|e| e.value().client_id.as_ref() == Some(client))
                .count();

            if client_count >= self.config.max_per_client {
                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                warn!(
                    client_id = %client,
                    count = client_count,
                    max = self.config.max_per_client,
                    "Per-client connection limit reached"
                );
                return Err(WsError::connection_limit(format!(
                    "maximum connections per client ({}) reached",
                    self.config.max_per_client
                )));
            }
        }

        // Create connection info
        let id = ConnectionId::new();
        let mut info = ConnectionInfo::new(id, connection_type);
        if let Some(client) = client_id {
            info.client_id = Some(client);
        }

        self.connections.insert(id, info);
        self.total_accepted.fetch_add(1, Ordering::Relaxed);

        debug!(
            connection_id = %id,
            connection_type = %connection_type,
            total = self.connections.len(),
            "Connection accepted"
        );

        Ok(id)
    }

    /// Accept a connection with a specific ID.
    ///
    /// This is useful when the connection ID is already known.
    pub fn accept_with_id(
        &self,
        id: ConnectionId,
        connection_type: ConnectionType,
        client_id: Option<String>,
    ) -> WsResult<()> {
        // Check if shutdown
        if self.is_shutdown.load(Ordering::SeqCst) {
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(WsError::connection_limit("server is shutting down"));
        }

        // Check global limit
        if self.connections.len() >= self.config.max_connections {
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(WsError::connection_limit("maximum connections reached"));
        }

        // Check per-client limit
        if let Some(ref client) = client_id {
            let client_count = self
                .connections
                .iter()
                .filter(|e| e.value().client_id.as_ref() == Some(client))
                .count();

            if client_count >= self.config.max_per_client {
                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                return Err(WsError::connection_limit(
                    "maximum connections per client reached",
                ));
            }
        }

        let mut info = ConnectionInfo::new(id, connection_type);
        if let Some(client) = client_id {
            info.client_id = Some(client);
        }

        self.connections.insert(id, info);
        self.total_accepted.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Remove a connection.
    pub fn remove(&self, id: &ConnectionId) -> Option<ConnectionInfo> {
        let removed = self.connections.remove(id).map(|(_, info)| info);
        if removed.is_some() {
            self.total_closed.fetch_add(1, Ordering::Relaxed);
            debug!(connection_id = %id, "Connection removed");
        }
        removed
    }

    /// Get information about a connection.
    pub fn get(&self, id: &ConnectionId) -> Option<ConnectionInfo> {
        self.connections.get(id).map(|e| e.value().clone())
    }

    /// Update the last activity time for a connection.
    pub fn touch(&self, id: &ConnectionId) {
        if let Some(mut entry) = self.connections.get_mut(id) {
            entry.touch();
        }
    }

    /// Check if a connection exists.
    pub fn contains(&self, id: &ConnectionId) -> bool {
        self.connections.contains_key(id)
    }

    /// Get the number of active connections.
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// Check if there are no active connections.
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }

    /// Get statistics about the connection manager.
    pub fn stats(&self) -> ConnectionStats {
        let mut ws_count = 0;
        let mut sse_count = 0;

        for entry in self.connections.iter() {
            match entry.value().connection_type {
                ConnectionType::WebSocket => ws_count += 1,
                ConnectionType::ServerSentEvents => sse_count += 1,
            }
        }

        ConnectionStats {
            active_connections: self.connections.len(),
            websocket_connections: ws_count,
            sse_connections: sse_count,
            total_accepted: self.total_accepted.load(Ordering::Relaxed),
            total_rejected: self.total_rejected.load(Ordering::Relaxed),
            total_closed: self.total_closed.load(Ordering::Relaxed),
        }
    }

    /// Get all connection IDs.
    pub fn connection_ids(&self) -> Vec<ConnectionId> {
        self.connections.iter().map(|e| *e.key()).collect()
    }

    /// Get all connections.
    pub fn connections(&self) -> Vec<ConnectionInfo> {
        self.connections.iter().map(|e| e.value().clone()).collect()
    }

    /// Get connections for a specific client.
    pub fn client_connections(&self, client_id: &str) -> Vec<ConnectionInfo> {
        self.connections
            .iter()
            .filter(|e| e.value().client_id.as_deref() == Some(client_id))
            .map(|e| e.value().clone())
            .collect()
    }

    /// Get a receiver for shutdown notifications.
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Check if shutdown has been triggered.
    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(Ordering::SeqCst)
    }

    /// Trigger shutdown and notify all connections.
    ///
    /// This will:
    /// 1. Set the shutdown flag to prevent new connections
    /// 2. Send a shutdown signal to all listeners
    /// 3. Return the number of connections that were notified
    pub fn shutdown(&self) -> usize {
        if self.is_shutdown.swap(true, Ordering::SeqCst) {
            // Already shutdown
            return 0;
        }

        let count = self.connections.len();
        info!(connections = count, "Initiating shutdown");

        // Send shutdown signal (ignore errors - receivers may have been dropped)
        let _ = self.shutdown_tx.send(());

        count
    }

    /// Remove idle connections that have exceeded the idle timeout.
    ///
    /// Returns the number of connections removed.
    pub fn cleanup_idle(&self) -> usize {
        let timeout = self.config.idle_timeout;
        let mut removed = 0;

        // Collect IDs to remove (can't remove while iterating)
        let to_remove: Vec<ConnectionId> = self
            .connections
            .iter()
            .filter(|e| e.value().idle_duration() > timeout)
            .map(|e| *e.key())
            .collect();

        for id in to_remove {
            if self.connections.remove(&id).is_some() {
                removed += 1;
                self.total_closed.fetch_add(1, Ordering::Relaxed);
                debug!(connection_id = %id, "Removed idle connection");
            }
        }

        if removed > 0 {
            info!(count = removed, "Cleaned up idle connections");
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_config() -> ConnectionManagerConfig {
        ConnectionManagerConfig {
            max_connections: 10,
            max_per_client: 3,
            idle_timeout: Duration::from_millis(100),
            cleanup_interval: Duration::from_millis(50),
        }
    }

    #[test]
    fn test_accept_connection() {
        let manager = ConnectionManager::new(test_config());

        let id = manager.accept(ConnectionType::WebSocket, None).unwrap();
        assert!(manager.contains(&id));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_accept_with_client_id() {
        let manager = ConnectionManager::new(test_config());

        let id = manager
            .accept(ConnectionType::WebSocket, Some("user1".to_string()))
            .unwrap();

        let info = manager.get(&id).unwrap();
        assert_eq!(info.client_id, Some("user1".to_string()));
    }

    #[test]
    fn test_global_connection_limit() {
        let config = ConnectionManagerConfig {
            max_connections: 2,
            ..test_config()
        };
        let manager = ConnectionManager::new(config);

        manager.accept(ConnectionType::WebSocket, None).unwrap();
        manager.accept(ConnectionType::WebSocket, None).unwrap();

        let result = manager.accept(ConnectionType::WebSocket, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_per_client_limit() {
        let manager = ConnectionManager::new(test_config());
        let client = "user1".to_string();

        manager
            .accept(ConnectionType::WebSocket, Some(client.clone()))
            .unwrap();
        manager
            .accept(ConnectionType::WebSocket, Some(client.clone()))
            .unwrap();
        manager
            .accept(ConnectionType::WebSocket, Some(client.clone()))
            .unwrap();

        let result = manager.accept(ConnectionType::WebSocket, Some(client));
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_connection() {
        let manager = ConnectionManager::new(test_config());

        let id = manager.accept(ConnectionType::WebSocket, None).unwrap();
        assert!(manager.contains(&id));

        let removed = manager.remove(&id);
        assert!(removed.is_some());
        assert!(!manager.contains(&id));
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_touch() {
        let manager = ConnectionManager::new(test_config());

        let id = manager.accept(ConnectionType::WebSocket, None).unwrap();
        let before = manager.get(&id).unwrap().last_activity;

        std::thread::sleep(Duration::from_millis(10));
        manager.touch(&id);

        let after = manager.get(&id).unwrap().last_activity;
        assert!(after > before);
    }

    #[test]
    fn test_stats() {
        let manager = ConnectionManager::new(test_config());

        manager.accept(ConnectionType::WebSocket, None).unwrap();
        manager.accept(ConnectionType::WebSocket, None).unwrap();
        manager
            .accept(ConnectionType::ServerSentEvents, None)
            .unwrap();

        let stats = manager.stats();
        assert_eq!(stats.active_connections, 3);
        assert_eq!(stats.websocket_connections, 2);
        assert_eq!(stats.sse_connections, 1);
        assert_eq!(stats.total_accepted, 3);
    }

    #[test]
    fn test_shutdown() {
        let manager = ConnectionManager::new(test_config());

        manager.accept(ConnectionType::WebSocket, None).unwrap();
        manager.accept(ConnectionType::WebSocket, None).unwrap();

        let notified = manager.shutdown();
        assert_eq!(notified, 2);
        assert!(manager.is_shutdown());

        // Can't accept new connections after shutdown
        let result = manager.accept(ConnectionType::WebSocket, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cleanup_idle() {
        let manager = ConnectionManager::new(test_config());

        let id = manager.accept(ConnectionType::WebSocket, None).unwrap();
        assert_eq!(manager.len(), 1);

        // Wait for idle timeout
        std::thread::sleep(Duration::from_millis(150));

        let removed = manager.cleanup_idle();
        assert_eq!(removed, 1);
        assert!(!manager.contains(&id));
    }

    #[test]
    fn test_client_connections() {
        let manager = ConnectionManager::new(test_config());

        manager
            .accept(ConnectionType::WebSocket, Some("user1".to_string()))
            .unwrap();
        manager
            .accept(ConnectionType::WebSocket, Some("user1".to_string()))
            .unwrap();
        manager
            .accept(ConnectionType::WebSocket, Some("user2".to_string()))
            .unwrap();

        let user1_conns = manager.client_connections("user1");
        assert_eq!(user1_conns.len(), 2);

        let user2_conns = manager.client_connections("user2");
        assert_eq!(user2_conns.len(), 1);
    }

    #[test]
    fn test_connection_type_display() {
        assert_eq!(ConnectionType::WebSocket.to_string(), "WebSocket");
        assert_eq!(ConnectionType::ServerSentEvents.to_string(), "SSE");
    }

    #[test]
    fn test_connection_info_with_builders() {
        let id = ConnectionId::new();
        let info = ConnectionInfo::websocket(id)
            .with_client_id("user1")
            .with_metadata("test metadata");

        assert_eq!(info.id, id);
        assert_eq!(info.client_id, Some("user1".to_string()));
        assert_eq!(info.metadata, Some("test metadata".to_string()));
        assert_eq!(info.connection_type, ConnectionType::WebSocket);
    }
}
