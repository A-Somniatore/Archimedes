//! WebSocket message types.
//!
//! This module defines the message types used in WebSocket communication,
//! including text, binary, ping/pong, and close frames.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::error::{CloseCode, WsError, WsResult};

/// A WebSocket message.
///
/// Messages can be text, binary, ping, pong, or close frames.
/// Text messages are UTF-8 encoded strings, while binary messages
/// are raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// A text message (UTF-8 encoded).
    Text(String),
    /// A binary message.
    Binary(Vec<u8>),
    /// A ping frame with optional payload.
    Ping(Vec<u8>),
    /// A pong frame with optional payload.
    Pong(Vec<u8>),
    /// A close frame with optional code and reason.
    Close(Option<CloseFrame>),
}

impl Message {
    /// Create a new text message.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Create a new binary message.
    pub fn binary(data: impl Into<Vec<u8>>) -> Self {
        Self::Binary(data.into())
    }

    /// Create a new ping message.
    pub fn ping(data: impl Into<Vec<u8>>) -> Self {
        Self::Ping(data.into())
    }

    /// Create a new pong message.
    pub fn pong(data: impl Into<Vec<u8>>) -> Self {
        Self::Pong(data.into())
    }

    /// Create a close message with a code and reason.
    pub fn close(code: CloseCode, reason: impl Into<String>) -> Self {
        Self::Close(Some(CloseFrame {
            code: code.as_u16(),
            reason: Cow::Owned(reason.into()),
        }))
    }

    /// Create a close message with just a code.
    pub fn close_with_code(code: CloseCode) -> Self {
        Self::Close(Some(CloseFrame {
            code: code.as_u16(),
            reason: Cow::Borrowed(""),
        }))
    }

    /// Create an empty close message.
    pub fn close_empty() -> Self {
        Self::Close(None)
    }

    /// Check if this is a text message.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Check if this is a binary message.
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    /// Check if this is a ping message.
    pub fn is_ping(&self) -> bool {
        matches!(self, Self::Ping(_))
    }

    /// Check if this is a pong message.
    pub fn is_pong(&self) -> bool {
        matches!(self, Self::Pong(_))
    }

    /// Check if this is a close message.
    pub fn is_close(&self) -> bool {
        matches!(self, Self::Close(_))
    }

    /// Check if this is a data message (text or binary).
    pub fn is_data(&self) -> bool {
        matches!(self, Self::Text(_) | Self::Binary(_))
    }

    /// Check if this is a control message (ping, pong, or close).
    pub fn is_control(&self) -> bool {
        matches!(self, Self::Ping(_) | Self::Pong(_) | Self::Close(_))
    }

    /// Get the message payload as text.
    ///
    /// Returns `None` if this is not a text message.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Get the message payload as bytes.
    ///
    /// Returns `None` for close messages.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Text(s) => Some(s.as_bytes()),
            Self::Binary(b) | Self::Ping(b) | Self::Pong(b) => Some(b),
            Self::Close(_) => None,
        }
    }

    /// Get the close frame if this is a close message.
    pub fn close_frame(&self) -> Option<&CloseFrame> {
        match self {
            Self::Close(frame) => frame.as_ref(),
            _ => None,
        }
    }

    /// Convert the message into text.
    ///
    /// Returns `None` if this is not a text message.
    pub fn into_text(self) -> Option<String> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Convert the message into bytes.
    ///
    /// Returns `None` for close messages.
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Text(s) => Some(s.into_bytes()),
            Self::Binary(b) | Self::Ping(b) | Self::Pong(b) => Some(b),
            Self::Close(_) => None,
        }
    }

    /// Try to parse the text message as JSON.
    ///
    /// Returns an error if this is not a text message or if parsing fails.
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> WsResult<T> {
        let text = self
            .as_text()
            .ok_or_else(|| WsError::DecodeFailed("not a text message".to_string()))?;
        serde_json::from_str(text).map_err(|e| WsError::DecodeFailed(e.to_string()))
    }

    /// Create a text message from a JSON-serializable value.
    pub fn from_json<T: Serialize>(value: &T) -> WsResult<Self> {
        let text =
            serde_json::to_string(value).map_err(|e| WsError::EncodeFailed(e.to_string()))?;
        Ok(Self::Text(text))
    }

    /// Get the length of the message payload in bytes.
    pub fn len(&self) -> usize {
        match self {
            Self::Text(s) => s.len(),
            Self::Binary(b) | Self::Ping(b) | Self::Pong(b) => b.len(),
            Self::Close(Some(frame)) => 2 + frame.reason.len(),
            Self::Close(None) => 0,
        }
    }

    /// Check if the message payload is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<String> for Message {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for Message {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<Vec<u8>> for Message {
    fn from(b: Vec<u8>) -> Self {
        Self::Binary(b)
    }
}

impl From<Bytes> for Message {
    fn from(b: Bytes) -> Self {
        Self::Binary(b.to_vec())
    }
}

impl From<&[u8]> for Message {
    fn from(b: &[u8]) -> Self {
        Self::Binary(b.to_vec())
    }
}

/// Convert from tungstenite Message.
impl From<tungstenite::Message> for Message {
    fn from(msg: tungstenite::Message) -> Self {
        match msg {
            tungstenite::Message::Text(s) => Self::Text(s.to_string()),
            tungstenite::Message::Binary(b) => Self::Binary(b.to_vec()),
            tungstenite::Message::Ping(b) => Self::Ping(b.to_vec()),
            tungstenite::Message::Pong(b) => Self::Pong(b.to_vec()),
            tungstenite::Message::Close(frame) => Self::Close(frame.map(CloseFrame::from)),
            tungstenite::Message::Frame(_) => Self::Binary(vec![]),
        }
    }
}

/// Convert to tungstenite Message.
impl From<Message> for tungstenite::Message {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Text(s) => Self::Text(s.into()),
            Message::Binary(b) => Self::Binary(b.into()),
            Message::Ping(b) => Self::Ping(b.into()),
            Message::Pong(b) => Self::Pong(b.into()),
            Message::Close(frame) => {
                Self::Close(frame.map(tungstenite::protocol::CloseFrame::from))
            }
        }
    }
}

/// A WebSocket close frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseFrame {
    /// The close code.
    pub code: u16,
    /// The close reason.
    pub reason: Cow<'static, str>,
}

impl CloseFrame {
    /// Create a new close frame.
    pub fn new(code: CloseCode, reason: impl Into<String>) -> Self {
        Self {
            code: code.as_u16(),
            reason: Cow::Owned(reason.into()),
        }
    }

    /// Create a normal close frame.
    pub fn normal(reason: impl Into<String>) -> Self {
        Self::new(CloseCode::Normal, reason)
    }

    /// Create a close frame for going away.
    pub fn going_away(reason: impl Into<String>) -> Self {
        Self::new(CloseCode::GoingAway, reason)
    }

    /// Create a close frame for a protocol error.
    pub fn protocol_error(reason: impl Into<String>) -> Self {
        Self::new(CloseCode::Protocol, reason)
    }

    /// Create a close frame for invalid payload.
    pub fn invalid_payload(reason: impl Into<String>) -> Self {
        Self::new(CloseCode::InvalidPayload, reason)
    }

    /// Create a close frame for internal error.
    pub fn internal_error(reason: impl Into<String>) -> Self {
        Self::new(CloseCode::InternalError, reason)
    }

    /// Get the close code enum value if it's a standard code.
    pub fn close_code(&self) -> Option<CloseCode> {
        CloseCode::from_u16(self.code)
    }
}

/// Convert from tungstenite CloseFrame.
impl From<tungstenite::protocol::CloseFrame> for CloseFrame {
    fn from(frame: tungstenite::protocol::CloseFrame) -> Self {
        Self {
            code: frame.code.into(),
            reason: Cow::Owned(frame.reason.to_string()),
        }
    }
}

/// Convert to tungstenite CloseFrame.
impl From<CloseFrame> for tungstenite::protocol::CloseFrame {
    fn from(frame: CloseFrame) -> Self {
        Self {
            code: frame.code.into(),
            reason: frame.reason.to_string().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_text() {
        let msg = Message::text("hello");
        assert!(msg.is_text());
        assert!(msg.is_data());
        assert!(!msg.is_control());
        assert_eq!(msg.as_text(), Some("hello"));
        assert_eq!(msg.len(), 5);
    }

    #[test]
    fn test_message_binary() {
        let msg = Message::binary(vec![1, 2, 3, 4]);
        assert!(msg.is_binary());
        assert!(msg.is_data());
        assert_eq!(msg.as_bytes(), Some(&[1, 2, 3, 4][..]));
        assert_eq!(msg.len(), 4);
    }

    #[test]
    fn test_message_ping_pong() {
        let ping = Message::ping(vec![1, 2]);
        assert!(ping.is_ping());
        assert!(ping.is_control());

        let pong = Message::pong(vec![1, 2]);
        assert!(pong.is_pong());
        assert!(pong.is_control());
    }

    #[test]
    fn test_message_close() {
        let msg = Message::close(CloseCode::Normal, "goodbye");
        assert!(msg.is_close());
        assert!(msg.is_control());
        let frame = msg.close_frame().unwrap();
        assert_eq!(frame.code, 1000);
        assert_eq!(frame.reason, "goodbye");
    }

    #[test]
    fn test_message_json() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Data {
            value: i32,
        }

        let data = Data { value: 42 };
        let msg = Message::from_json(&data).unwrap();
        assert!(msg.is_text());

        let parsed: Data = msg.json().unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn test_message_from_string() {
        let msg: Message = "hello".into();
        assert!(msg.is_text());
        assert_eq!(msg.as_text(), Some("hello"));
    }

    #[test]
    fn test_message_from_bytes() {
        let msg: Message = vec![1, 2, 3].into();
        assert!(msg.is_binary());
        assert_eq!(msg.as_bytes(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn test_close_frame_constructors() {
        let frame = CloseFrame::normal("done");
        assert_eq!(frame.code, 1000);

        let frame = CloseFrame::going_away("shutdown");
        assert_eq!(frame.code, 1001);

        let frame = CloseFrame::protocol_error("bad frame");
        assert_eq!(frame.code, 1002);
    }

    #[test]
    fn test_close_frame_close_code() {
        let frame = CloseFrame::new(CloseCode::Normal, "");
        assert_eq!(frame.close_code(), Some(CloseCode::Normal));

        let frame = CloseFrame {
            code: 9999,
            reason: Cow::Borrowed(""),
        };
        assert_eq!(frame.close_code(), None);
    }

    #[test]
    fn test_message_into_text() {
        let msg = Message::text("hello");
        assert_eq!(msg.into_text(), Some("hello".to_string()));

        let msg = Message::binary(vec![1, 2, 3]);
        assert_eq!(msg.into_text(), None);
    }

    #[test]
    fn test_message_into_bytes() {
        let msg = Message::text("hello");
        assert_eq!(msg.into_bytes(), Some(b"hello".to_vec()));

        let msg = Message::binary(vec![1, 2, 3]);
        assert_eq!(msg.into_bytes(), Some(vec![1, 2, 3]));

        let msg = Message::close_empty();
        assert_eq!(msg.into_bytes(), None);
    }
}
