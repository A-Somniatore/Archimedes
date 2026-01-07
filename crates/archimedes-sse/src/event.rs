//! SSE event types.
//!
//! This module defines the event types used in Server-Sent Events communication.

use serde::Serialize;
use std::time::Duration;

use crate::error::{SseError, SseResult};

/// A Server-Sent Event.
///
/// SSE events consist of:
/// - `id` - Optional event ID for client-side tracking and reconnection
/// - `event` - Optional event type (default is "message")
/// - `data` - The event payload (required, can be multi-line)
/// - `retry` - Optional reconnection time hint for the client
///
/// # Example
///
/// ```
/// use archimedes_sse::SseEvent;
///
/// let event = SseEvent::new("Hello, World!")
///     .id("1")
///     .event("greeting");
///
/// assert_eq!(event.data(), "Hello, World!");
/// assert_eq!(event.id_value(), Some("1"));
/// assert_eq!(event.event_type(), Some("greeting"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// Optional event ID.
    id: Option<String>,
    /// Optional event type.
    event: Option<String>,
    /// Event data (required).
    data: String,
    /// Optional retry interval hint.
    retry: Option<Duration>,
}

impl SseEvent {
    /// Create a new SSE event with the given data.
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            id: None,
            event: None,
            data: data.into(),
            retry: None,
        }
    }

    /// Create an SSE event from a JSON-serializable value.
    pub fn json<T: Serialize>(value: &T) -> SseResult<Self> {
        let data =
            serde_json::to_string(value).map_err(|e| SseError::serialization_failed(e.to_string()))?;
        Ok(Self::new(data))
    }

    /// Set the event ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the event type.
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Set the retry interval.
    pub fn retry(mut self, retry: Duration) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Get the event ID.
    pub fn id_value(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the event type.
    pub fn event_type(&self) -> Option<&str> {
        self.event.as_deref()
    }

    /// Get the event data.
    pub fn data(&self) -> &str {
        &self.data
    }

    /// Get the retry interval.
    pub fn retry_interval(&self) -> Option<Duration> {
        self.retry
    }

    /// Format the event as an SSE text block.
    ///
    /// The format follows the SSE specification:
    /// ```text
    /// id: <id>
    /// event: <event>
    /// data: <data line 1>
    /// data: <data line 2>
    /// retry: <ms>
    ///
    /// ```
    pub fn to_sse_string(&self) -> String {
        let mut result = String::new();

        if let Some(id) = &self.id {
            result.push_str("id: ");
            result.push_str(id);
            result.push('\n');
        }

        if let Some(event) = &self.event {
            result.push_str("event: ");
            result.push_str(event);
            result.push('\n');
        }

        // Data can be multi-line - each line needs "data: " prefix
        for line in self.data.lines() {
            result.push_str("data: ");
            result.push_str(line);
            result.push('\n');
        }

        // Handle case where data doesn't end with newline
        if !self.data.is_empty() && !self.data.ends_with('\n') {
            // Data already written above
        }

        if let Some(retry) = &self.retry {
            result.push_str("retry: ");
            result.push_str(&retry.as_millis().to_string());
            result.push('\n');
        }

        // Double newline to end the event
        result.push('\n');

        result
    }

    /// Convert to bytes for sending.
    pub fn to_bytes(&self) -> bytes::Bytes {
        bytes::Bytes::from(self.to_sse_string())
    }
}

impl Default for SseEvent {
    fn default() -> Self {
        Self::new("")
    }
}

impl From<String> for SseEvent {
    fn from(data: String) -> Self {
        Self::new(data)
    }
}

impl From<&str> for SseEvent {
    fn from(data: &str) -> Self {
        Self::new(data)
    }
}

/// A comment line in the SSE stream.
///
/// Comments start with a colon and are used for keepalive pings.
/// They are ignored by the client but keep the connection alive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseComment(String);

impl SseComment {
    /// Create a new comment.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Create an empty comment (keepalive).
    pub fn keepalive() -> Self {
        Self::new("")
    }

    /// Get the comment text.
    pub fn text(&self) -> &str {
        &self.0
    }

    /// Format as SSE comment.
    pub fn to_sse_string(&self) -> String {
        format!(": {}\n", self.0)
    }

    /// Convert to bytes.
    pub fn to_bytes(&self) -> bytes::Bytes {
        bytes::Bytes::from(self.to_sse_string())
    }
}

impl Default for SseComment {
    fn default() -> Self {
        Self::keepalive()
    }
}

/// An item that can be sent on an SSE stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseItem {
    /// An event with data.
    Event(SseEvent),
    /// A comment (keepalive).
    Comment(SseComment),
}

impl SseItem {
    /// Create an event item.
    pub fn event(event: SseEvent) -> Self {
        Self::Event(event)
    }

    /// Create a comment item.
    pub fn comment(comment: impl Into<String>) -> Self {
        Self::Comment(SseComment::new(comment))
    }

    /// Create a keepalive comment.
    pub fn keepalive() -> Self {
        Self::Comment(SseComment::keepalive())
    }

    /// Check if this is an event.
    pub fn is_event(&self) -> bool {
        matches!(self, Self::Event(_))
    }

    /// Check if this is a comment.
    pub fn is_comment(&self) -> bool {
        matches!(self, Self::Comment(_))
    }

    /// Get the event if this is one.
    pub fn as_event(&self) -> Option<&SseEvent> {
        match self {
            Self::Event(e) => Some(e),
            Self::Comment(_) => None,
        }
    }

    /// Format as SSE text.
    pub fn to_sse_string(&self) -> String {
        match self {
            Self::Event(e) => e.to_sse_string(),
            Self::Comment(c) => c.to_sse_string(),
        }
    }

    /// Convert to bytes.
    pub fn to_bytes(&self) -> bytes::Bytes {
        bytes::Bytes::from(self.to_sse_string())
    }
}

impl From<SseEvent> for SseItem {
    fn from(event: SseEvent) -> Self {
        Self::Event(event)
    }
}

impl From<SseComment> for SseItem {
    fn from(comment: SseComment) -> Self {
        Self::Comment(comment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_new() {
        let event = SseEvent::new("hello");
        assert_eq!(event.data(), "hello");
        assert_eq!(event.id_value(), None);
        assert_eq!(event.event_type(), None);
    }

    #[test]
    fn test_event_with_id() {
        let event = SseEvent::new("hello").id("123");
        assert_eq!(event.id_value(), Some("123"));
    }

    #[test]
    fn test_event_with_event_type() {
        let event = SseEvent::new("hello").event("greeting");
        assert_eq!(event.event_type(), Some("greeting"));
    }

    #[test]
    fn test_event_with_retry() {
        let event = SseEvent::new("hello").retry(Duration::from_secs(5));
        assert_eq!(event.retry_interval(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_event_json() {
        #[derive(Serialize)]
        struct Data {
            value: i32,
        }
        let event = SseEvent::json(&Data { value: 42 }).unwrap();
        assert!(event.data().contains("42"));
    }

    #[test]
    fn test_event_to_sse_string_simple() {
        let event = SseEvent::new("hello");
        let output = event.to_sse_string();
        assert_eq!(output, "data: hello\n\n");
    }

    #[test]
    fn test_event_to_sse_string_full() {
        let event = SseEvent::new("hello")
            .id("1")
            .event("greeting")
            .retry(Duration::from_secs(5));
        let output = event.to_sse_string();
        assert!(output.contains("id: 1\n"));
        assert!(output.contains("event: greeting\n"));
        assert!(output.contains("data: hello\n"));
        assert!(output.contains("retry: 5000\n"));
    }

    #[test]
    fn test_event_multiline_data() {
        let event = SseEvent::new("line1\nline2\nline3");
        let output = event.to_sse_string();
        assert!(output.contains("data: line1\n"));
        assert!(output.contains("data: line2\n"));
        assert!(output.contains("data: line3\n"));
    }

    #[test]
    fn test_comment_keepalive() {
        let comment = SseComment::keepalive();
        assert_eq!(comment.to_sse_string(), ": \n");
    }

    #[test]
    fn test_comment_with_text() {
        let comment = SseComment::new("ping");
        assert_eq!(comment.to_sse_string(), ": ping\n");
    }

    #[test]
    fn test_sse_item_event() {
        let item = SseItem::event(SseEvent::new("test"));
        assert!(item.is_event());
        assert!(!item.is_comment());
    }

    #[test]
    fn test_sse_item_comment() {
        let item = SseItem::keepalive();
        assert!(item.is_comment());
        assert!(!item.is_event());
    }

    #[test]
    fn test_event_from_string() {
        let event: SseEvent = "hello".into();
        assert_eq!(event.data(), "hello");
    }
}
