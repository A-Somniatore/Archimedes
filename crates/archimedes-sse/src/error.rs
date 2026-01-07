//! Error types for Server-Sent Events operations.

use thiserror::Error;

/// Result type for SSE operations.
pub type SseResult<T> = Result<T, SseError>;

/// Errors that can occur during SSE operations.
#[derive(Debug, Error)]
pub enum SseError {
    /// The event stream was closed.
    #[error("stream closed: {0}")]
    StreamClosed(String),

    /// Failed to send an event.
    #[error("failed to send event: {0}")]
    SendFailed(String),

    /// The channel is full (backpressure).
    #[error("channel full, backpressure limit reached")]
    ChannelFull,

    /// Failed to serialize event data.
    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    /// Invalid event format.
    #[error("invalid event format: {0}")]
    InvalidFormat(String),

    /// Connection limit reached.
    #[error("connection limit reached: {0}")]
    ConnectionLimitReached(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl SseError {
    /// Create a stream closed error.
    pub fn stream_closed(reason: impl Into<String>) -> Self {
        Self::StreamClosed(reason.into())
    }

    /// Create a send failed error.
    pub fn send_failed(reason: impl Into<String>) -> Self {
        Self::SendFailed(reason.into())
    }

    /// Create a serialization failed error.
    pub fn serialization_failed(reason: impl Into<String>) -> Self {
        Self::SerializationFailed(reason.into())
    }

    /// Create an invalid format error.
    pub fn invalid_format(reason: impl Into<String>) -> Self {
        Self::InvalidFormat(reason.into())
    }

    /// Create a connection limit error.
    pub fn connection_limit(reason: impl Into<String>) -> Self {
        Self::ConnectionLimitReached(reason.into())
    }

    /// Create an internal error.
    pub fn internal(reason: impl Into<String>) -> Self {
        Self::Internal(reason.into())
    }

    /// Create a channel full error.
    pub fn channel_full() -> Self {
        Self::ChannelFull
    }

    /// Check if this error is recoverable (can retry).
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::ChannelFull | Self::SendFailed(_))
    }

    /// Check if this error indicates the stream should be closed.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::StreamClosed(_) | Self::ConnectionLimitReached(_) | Self::Internal(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_stream_closed() {
        let err = SseError::stream_closed("client disconnected");
        assert!(err.to_string().contains("client disconnected"));
        assert!(err.is_fatal());
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_error_channel_full() {
        let err = SseError::ChannelFull;
        assert!(err.is_recoverable());
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_error_send_failed() {
        let err = SseError::send_failed("network error");
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_error_serialization() {
        let err = SseError::serialization_failed("invalid json");
        assert!(!err.is_recoverable());
    }
}
