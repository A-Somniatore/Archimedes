//! Error types for WebSocket operations.
//!
//! This module defines the error types that can occur during WebSocket
//! connection establishment, message handling, and connection management.

use std::fmt;
use thiserror::Error;

/// Result type for WebSocket operations.
pub type WsResult<T> = Result<T, WsError>;

/// Errors that can occur during WebSocket operations.
#[derive(Debug, Error)]
pub enum WsError {
    /// The HTTP request was not a valid WebSocket upgrade request.
    #[error("not a WebSocket upgrade request: {reason}")]
    NotWebSocketRequest {
        /// Reason why the request is not a valid WebSocket upgrade.
        reason: String,
    },

    /// The WebSocket handshake failed.
    #[error("WebSocket handshake failed: {0}")]
    HandshakeFailed(String),

    /// The WebSocket connection was closed.
    #[error("connection closed: {reason}")]
    ConnectionClosed {
        /// Optional close code from the peer.
        code: Option<u16>,
        /// Reason for closing.
        reason: String,
    },

    /// Failed to send a message.
    #[error("failed to send message: {0}")]
    SendFailed(String),

    /// Failed to receive a message.
    #[error("failed to receive message: {0}")]
    ReceiveFailed(String),

    /// Message validation failed against the contract schema.
    #[error("message validation failed: {0}")]
    ValidationFailed(String),

    /// The message payload could not be decoded.
    #[error("failed to decode message: {0}")]
    DecodeFailed(String),

    /// The message payload could not be encoded.
    #[error("failed to encode message: {0}")]
    EncodeFailed(String),

    /// Connection limit reached.
    #[error("connection limit reached: {0}")]
    ConnectionLimitReached(String),

    /// Connection not found.
    #[error("connection not found: {connection_id}")]
    ConnectionNotFound {
        /// The ID of the connection that was not found.
        connection_id: String,
    },

    /// Protocol error.
    #[error("protocol error: {0}")]
    ProtocolError(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Tungstenite error.
    #[error("tungstenite error: {0}")]
    Tungstenite(#[from] tungstenite::Error),
}

impl WsError {
    /// Create a new "not a WebSocket request" error.
    pub fn not_websocket(reason: impl Into<String>) -> Self {
        Self::NotWebSocketRequest {
            reason: reason.into(),
        }
    }

    /// Create a new handshake failed error.
    pub fn handshake_failed(reason: impl Into<String>) -> Self {
        Self::HandshakeFailed(reason.into())
    }

    /// Create a new connection closed error.
    pub fn connection_closed(code: Option<u16>, reason: impl Into<String>) -> Self {
        Self::ConnectionClosed {
            code,
            reason: reason.into(),
        }
    }

    /// Create a new send failed error.
    pub fn send_failed(reason: impl Into<String>) -> Self {
        Self::SendFailed(reason.into())
    }

    /// Create a new receive failed error.
    pub fn receive_failed(reason: impl Into<String>) -> Self {
        Self::ReceiveFailed(reason.into())
    }

    /// Create a new validation failed error.
    pub fn validation_failed(reason: impl Into<String>) -> Self {
        Self::ValidationFailed(reason.into())
    }

    /// Create a new connection limit reached error.
    pub fn connection_limit(reason: impl Into<String>) -> Self {
        Self::ConnectionLimitReached(reason.into())
    }

    /// Create a new connection not found error.
    pub fn connection_not_found(connection_id: impl Into<String>) -> Self {
        Self::ConnectionNotFound {
            connection_id: connection_id.into(),
        }
    }

    /// Create a new protocol error.
    pub fn protocol_error(reason: impl Into<String>) -> Self {
        Self::ProtocolError(reason.into())
    }

    /// Create a new internal error.
    pub fn internal(reason: impl Into<String>) -> Self {
        Self::Internal(reason.into())
    }

    /// Get the close code if this is a connection closed error.
    pub fn close_code(&self) -> Option<u16> {
        match self {
            Self::ConnectionClosed { code, .. } => *code,
            _ => None,
        }
    }

    /// Check if this error indicates the connection should be closed.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::HandshakeFailed(_)
                | Self::ConnectionClosed { .. }
                | Self::ConnectionLimitReached(_)
                | Self::ProtocolError(_)
                | Self::Internal(_)
        )
    }
}

/// Close code for WebSocket connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CloseCode {
    /// Normal closure (1000).
    Normal = 1000,
    /// Going away (1001).
    GoingAway = 1001,
    /// Protocol error (1002).
    Protocol = 1002,
    /// Unsupported data (1003).
    Unsupported = 1003,
    /// No status received (1005).
    NoStatus = 1005,
    /// Abnormal closure (1006).
    Abnormal = 1006,
    /// Invalid payload data (1007).
    InvalidPayload = 1007,
    /// Policy violation (1008).
    PolicyViolation = 1008,
    /// Message too big (1009).
    MessageTooBig = 1009,
    /// Extension required (1010).
    ExtensionRequired = 1010,
    /// Internal error (1011).
    InternalError = 1011,
    /// Service restart (1012).
    ServiceRestart = 1012,
    /// Try again later (1013).
    TryAgainLater = 1013,
    /// Bad gateway (1014).
    BadGateway = 1014,
    /// TLS handshake failure (1015).
    TlsHandshake = 1015,
}

impl CloseCode {
    /// Convert from a u16 code.
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            1000 => Some(Self::Normal),
            1001 => Some(Self::GoingAway),
            1002 => Some(Self::Protocol),
            1003 => Some(Self::Unsupported),
            1005 => Some(Self::NoStatus),
            1006 => Some(Self::Abnormal),
            1007 => Some(Self::InvalidPayload),
            1008 => Some(Self::PolicyViolation),
            1009 => Some(Self::MessageTooBig),
            1010 => Some(Self::ExtensionRequired),
            1011 => Some(Self::InternalError),
            1012 => Some(Self::ServiceRestart),
            1013 => Some(Self::TryAgainLater),
            1014 => Some(Self::BadGateway),
            1015 => Some(Self::TlsHandshake),
            _ => None,
        }
    }

    /// Get the u16 value of this close code.
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

impl fmt::Display for CloseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Normal => "Normal",
            Self::GoingAway => "GoingAway",
            Self::Protocol => "Protocol",
            Self::Unsupported => "Unsupported",
            Self::NoStatus => "NoStatus",
            Self::Abnormal => "Abnormal",
            Self::InvalidPayload => "InvalidPayload",
            Self::PolicyViolation => "PolicyViolation",
            Self::MessageTooBig => "MessageTooBig",
            Self::ExtensionRequired => "ExtensionRequired",
            Self::InternalError => "InternalError",
            Self::ServiceRestart => "ServiceRestart",
            Self::TryAgainLater => "TryAgainLater",
            Self::BadGateway => "BadGateway",
            Self::TlsHandshake => "TlsHandshake",
        };
        write!(f, "{} ({})", name, self.as_u16())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_error_not_websocket() {
        let err = WsError::not_websocket("missing upgrade header");
        assert!(matches!(err, WsError::NotWebSocketRequest { .. }));
        assert!(err.to_string().contains("missing upgrade header"));
    }

    #[test]
    fn test_ws_error_connection_closed() {
        let err = WsError::connection_closed(Some(1000), "normal closure");
        assert_eq!(err.close_code(), Some(1000));
        assert!(err.is_fatal());
    }

    #[test]
    fn test_ws_error_validation_failed_not_fatal() {
        let err = WsError::validation_failed("invalid schema");
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_close_code_from_u16() {
        assert_eq!(CloseCode::from_u16(1000), Some(CloseCode::Normal));
        assert_eq!(CloseCode::from_u16(1001), Some(CloseCode::GoingAway));
        assert_eq!(CloseCode::from_u16(9999), None);
    }

    #[test]
    fn test_close_code_as_u16() {
        assert_eq!(CloseCode::Normal.as_u16(), 1000);
        assert_eq!(CloseCode::GoingAway.as_u16(), 1001);
    }

    #[test]
    fn test_close_code_display() {
        assert_eq!(CloseCode::Normal.to_string(), "Normal (1000)");
        assert_eq!(CloseCode::Protocol.to_string(), "Protocol (1002)");
    }
}
