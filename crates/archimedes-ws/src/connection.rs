//! WebSocket connection handling.
//!
//! This module provides the [`WebSocket`] type which wraps a WebSocket stream
//! and provides methods for sending and receiving messages with optional
//! contract-based validation.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, Stream, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

use crate::config::WebSocketConfig;
use crate::error::{CloseCode, WsError, WsResult};
use crate::message::Message;

/// A unique identifier for a WebSocket connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    /// Create a new random connection ID.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a connection ID from a UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ConnectionId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<ConnectionId> for Uuid {
    fn from(id: ConnectionId) -> Self {
        id.0
    }
}

/// A WebSocket connection.
///
/// This type wraps a WebSocket stream and provides methods for sending
/// and receiving messages. It tracks connection state and supports
/// automatic ping/pong handling.
///
/// # Example
///
/// ```ignore
/// use archimedes_ws::{WebSocket, Message};
///
/// async fn handle_ws(mut ws: WebSocket) {
///     while let Some(msg) = ws.recv().await {
///         match msg {
///             Ok(Message::Text(text)) => {
///                 ws.send(Message::text(format!("Echo: {}", text))).await?;
///             }
///             Ok(Message::Close(_)) => break,
///             Err(e) => {
///                 eprintln!("Error: {}", e);
///                 break;
///             }
///             _ => {}
///         }
///     }
/// }
/// ```
pub struct WebSocket<S = tokio::net::TcpStream> {
    /// The unique connection ID.
    connection_id: ConnectionId,
    /// The sender half of the WebSocket stream.
    sender: Arc<Mutex<SplitSink<WebSocketStream<S>, tungstenite::Message>>>,
    /// The receiver half of the WebSocket stream.
    receiver: SplitStream<WebSocketStream<S>>,
    /// Configuration for this connection.
    config: WebSocketConfig,
    /// When the connection was established.
    connected_at: Instant,
    /// Last time activity was seen on this connection.
    last_activity: Instant,
    /// Whether the connection has been closed.
    closed: bool,
}

impl<S> WebSocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Create a new WebSocket from an underlying stream.
    pub fn new(stream: WebSocketStream<S>, config: WebSocketConfig) -> Self {
        let (sender, receiver) = stream.split();
        let now = Instant::now();
        Self {
            connection_id: ConnectionId::new(),
            sender: Arc::new(Mutex::new(sender)),
            receiver,
            config,
            connected_at: now,
            last_activity: now,
            closed: false,
        }
    }

    /// Create a new WebSocket with a specific connection ID.
    pub fn with_id(
        stream: WebSocketStream<S>,
        config: WebSocketConfig,
        connection_id: ConnectionId,
    ) -> Self {
        let (sender, receiver) = stream.split();
        let now = Instant::now();
        Self {
            connection_id,
            sender: Arc::new(Mutex::new(sender)),
            receiver,
            config,
            connected_at: now,
            last_activity: now,
            closed: false,
        }
    }

    /// Get the connection ID.
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    /// Get the connection configuration.
    pub fn config(&self) -> &WebSocketConfig {
        &self.config
    }

    /// Get when the connection was established.
    pub fn connected_at(&self) -> Instant {
        self.connected_at
    }

    /// Get the last activity time.
    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }

    /// Check if the connection has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Get how long this connection has been open.
    pub fn connection_duration(&self) -> std::time::Duration {
        self.connected_at.elapsed()
    }

    /// Get how long since the last activity.
    pub fn idle_duration(&self) -> std::time::Duration {
        self.last_activity.elapsed()
    }

    /// Receive the next message from the WebSocket.
    ///
    /// Returns `None` when the connection is closed.
    #[instrument(skip(self), fields(connection_id = %self.connection_id))]
    pub async fn recv(&mut self) -> Option<WsResult<Message>> {
        if self.closed {
            return None;
        }

        match self.receiver.next().await {
            Some(Ok(msg)) => {
                self.last_activity = Instant::now();
                let msg = Message::from(msg);

                // Handle ping automatically
                if let Message::Ping(data) = &msg {
                    debug!("Received ping, sending pong");
                    if let Err(e) = self.send(Message::pong(data.clone())).await {
                        warn!("Failed to send pong: {}", e);
                    }
                }

                // Mark as closed if close frame received
                if msg.is_close() {
                    debug!("Received close frame");
                    self.closed = true;
                }

                Some(Ok(msg))
            }
            Some(Err(e)) => {
                self.closed = true;
                Some(Err(WsError::from(e)))
            }
            None => {
                self.closed = true;
                None
            }
        }
    }

    /// Send a message on the WebSocket.
    #[instrument(skip(self, msg), fields(connection_id = %self.connection_id, msg_type = ?msg_type(&msg)))]
    pub async fn send(&self, msg: Message) -> WsResult<()> {
        if self.closed {
            return Err(WsError::connection_closed(
                Some(CloseCode::Normal.as_u16()),
                "connection already closed",
            ));
        }

        let tungstenite_msg = tungstenite::Message::from(msg);
        let mut sender = self.sender.lock().await;
        sender
            .send(tungstenite_msg)
            .await
            .map_err(|e| WsError::send_failed(e.to_string()))
    }

    /// Send a text message.
    pub async fn send_text(&self, text: impl Into<String>) -> WsResult<()> {
        self.send(Message::text(text)).await
    }

    /// Send a binary message.
    pub async fn send_binary(&self, data: impl Into<Vec<u8>>) -> WsResult<()> {
        self.send(Message::binary(data)).await
    }

    /// Send a JSON message.
    pub async fn send_json<T: serde::Serialize>(&self, value: &T) -> WsResult<()> {
        let msg = Message::from_json(value)?;
        self.send(msg).await
    }

    /// Send a ping message.
    pub async fn ping(&self, data: impl Into<Vec<u8>>) -> WsResult<()> {
        self.send(Message::ping(data)).await
    }

    /// Close the WebSocket connection.
    pub async fn close(&mut self, code: CloseCode, reason: impl Into<String>) -> WsResult<()> {
        if self.closed {
            return Ok(());
        }

        let reason = reason.into();
        debug!(connection_id = %self.connection_id, code = code.as_u16(), reason = %reason, "Closing connection");

        let msg = Message::close(code, reason);
        self.send(msg).await?;
        self.closed = true;
        Ok(())
    }

    /// Close the WebSocket with a normal close code.
    pub async fn close_normal(&mut self, reason: impl Into<String>) -> WsResult<()> {
        self.close(CloseCode::Normal, reason).await
    }

    /// Get a handle that can be used to send messages from other tasks.
    pub fn sender(&self) -> WebSocketSender<S> {
        WebSocketSender {
            connection_id: self.connection_id,
            sender: Arc::clone(&self.sender),
        }
    }
}

impl<S> Stream for WebSocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Item = WsResult<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }

        match Pin::new(&mut self.receiver).poll_next(cx) {
            Poll::Ready(Some(Ok(msg))) => {
                self.last_activity = Instant::now();
                let msg = Message::from(msg);
                if msg.is_close() {
                    self.closed = true;
                }
                Poll::Ready(Some(Ok(msg)))
            }
            Poll::Ready(Some(Err(e))) => {
                self.closed = true;
                Poll::Ready(Some(Err(WsError::from(e))))
            }
            Poll::Ready(None) => {
                self.closed = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A handle for sending messages to a WebSocket from other tasks.
///
/// This is a cloneable handle that can be shared across tasks to send
/// messages to a WebSocket connection.
#[derive(Clone)]
pub struct WebSocketSender<S = tokio::net::TcpStream> {
    /// The connection ID.
    connection_id: ConnectionId,
    /// The sender half.
    sender: Arc<Mutex<SplitSink<WebSocketStream<S>, tungstenite::Message>>>,
}

impl<S> WebSocketSender<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Get the connection ID.
    pub fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    /// Send a message.
    pub async fn send(&self, msg: Message) -> WsResult<()> {
        let tungstenite_msg = tungstenite::Message::from(msg);
        let mut sender = self.sender.lock().await;
        sender
            .send(tungstenite_msg)
            .await
            .map_err(|e| WsError::send_failed(e.to_string()))
    }

    /// Send a text message.
    pub async fn send_text(&self, text: impl Into<String>) -> WsResult<()> {
        self.send(Message::text(text)).await
    }

    /// Send a binary message.
    pub async fn send_binary(&self, data: impl Into<Vec<u8>>) -> WsResult<()> {
        self.send(Message::binary(data)).await
    }

    /// Send a JSON message.
    pub async fn send_json<T: serde::Serialize>(&self, value: &T) -> WsResult<()> {
        let msg = Message::from_json(value)?;
        self.send(msg).await
    }
}

/// Helper function to get message type for logging.
fn msg_type(msg: &Message) -> &'static str {
    match msg {
        Message::Text(_) => "text",
        Message::Binary(_) => "binary",
        Message::Ping(_) => "ping",
        Message::Pong(_) => "pong",
        Message::Close(_) => "close",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_new() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_connection_id_from_uuid() {
        let uuid = Uuid::now_v7();
        let id = ConnectionId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_connection_id_display() {
        let uuid = Uuid::now_v7();
        let id = ConnectionId::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_connection_id_into_uuid() {
        let id = ConnectionId::new();
        let uuid: Uuid = id.into();
        assert_eq!(uuid, id.as_uuid());
    }
}
