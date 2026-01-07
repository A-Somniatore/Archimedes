//! WebSocket support for Archimedes framework.
//!
//! This crate provides WebSocket support for the Archimedes server framework,
//! including connection management, HTTP upgrade handling, and message
//! processing with optional contract-based validation.
//!
//! # Features
//!
//! - **RFC 6455 compliant** WebSocket implementation using `tokio-tungstenite`
//! - **Connection management** with configurable limits per client and globally
//! - **Automatic ping/pong** handling for connection health
//! - **Graceful shutdown** with connection notification
//! - **Message types** including Text, Binary, Ping, Pong, and Close
//! - **JSON serialization** support for typed messages
//!
//! # Example
//!
//! ```ignore
//! use archimedes_ws::{
//!     WebSocket, WebSocketConfig, Message,
//!     upgrade::{is_websocket_request, prepare_upgrade, complete_upgrade},
//!     manager::{ConnectionManager, ConnectionManagerConfig},
//! };
//!
//! // Create a connection manager
//! let manager = ConnectionManager::new(ConnectionManagerConfig::default());
//!
//! // In your HTTP handler, check for WebSocket upgrade
//! async fn handle_request(
//!     request: Request<Incoming>,
//!     manager: Arc<ConnectionManager>,
//! ) -> Response<Full<Bytes>> {
//!     if is_websocket_request(&request) {
//!         let upgrade = prepare_upgrade(&request, None);
//!         if upgrade.success {
//!             // Complete the upgrade after sending the response
//!             // ...
//!         }
//!         upgrade.response
//!     } else {
//!         // Handle normal HTTP request
//!         // ...
//!     }
//! }
//!
//! // WebSocket handler
//! async fn handle_websocket(mut ws: WebSocket) {
//!     while let Some(msg) = ws.recv().await {
//!         match msg {
//!             Ok(Message::Text(text)) => {
//!                 ws.send_text(format!("Echo: {}", text)).await.ok();
//!             }
//!             Ok(Message::Binary(data)) => {
//!                 ws.send_binary(data).await.ok();
//!             }
//!             Ok(Message::Close(_)) => break,
//!             Err(e) => {
//!                 eprintln!("WebSocket error: {}", e);
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     WebSocket Flow                          │
//! │                                                             │
//! │  HTTP Request ──► is_websocket_request() ──► prepare_upgrade()
//! │       │                                           │
//! │       ▼                                           ▼
//! │  Send 101 Response ◄────────────────────── WebSocketUpgrade
//! │       │
//! │       ▼
//! │  complete_upgrade() ──► WebSocket ──► recv()/send() loop
//! │       │
//! │       ▼
//! │  ConnectionManager.accept() ──► track connection
//! │       │
//! │       ▼
//! │  On close: ConnectionManager.remove()
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Message Handling
//!
//! The [`Message`] enum represents WebSocket messages:
//!
//! - [`Message::Text`] - UTF-8 text messages
//! - [`Message::Binary`] - Binary data
//! - [`Message::Ping`] - Ping frames (automatically responded with Pong)
//! - [`Message::Pong`] - Pong frames
//! - [`Message::Close`] - Close frames with optional code and reason
//!
//! # Connection Management
//!
//! The [`ConnectionManager`](manager::ConnectionManager) tracks active connections:
//!
//! - Enforces global and per-client connection limits
//! - Tracks connection metadata (client ID, connection time, etc.)
//! - Supports graceful shutdown with notification to all connections
//! - Automatically cleans up idle connections
//!
//! # Configuration
//!
//! - [`WebSocketConfig`](config::WebSocketConfig) - Per-connection settings
//! - [`ConnectionManagerConfig`](config::ConnectionManagerConfig) - Manager settings

pub mod config;
pub mod connection;
pub mod error;
pub mod manager;
pub mod message;
pub mod upgrade;

// Re-exports for convenience
pub use config::{ConnectionManagerConfig, WebSocketConfig};
pub use connection::{ConnectionId, WebSocket, WebSocketSender};
pub use error::{CloseCode, WsError, WsResult};
pub use manager::{ConnectionInfo, ConnectionManager, ConnectionStats, ConnectionType};
pub use message::{CloseFrame, Message};
pub use upgrade::{
    complete_upgrade, complete_upgrade_with_id, get_websocket_protocols, is_websocket_request,
    prepare_upgrade, validate_upgrade_request, WebSocketHandler, WebSocketUpgrade,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exports() {
        // Verify all public types are accessible
        let _config = WebSocketConfig::default();
        let _manager_config = ConnectionManagerConfig::default();
        let _id = ConnectionId::new();
        let _msg = Message::text("hello");
        let _close = CloseCode::Normal;
    }
}
