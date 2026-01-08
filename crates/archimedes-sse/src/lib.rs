//! # Archimedes SSE
//!
//! Server-Sent Events (SSE) support for the Archimedes framework.
//!
//! This crate provides types and utilities for implementing server-sent events
//! streams in HTTP handlers.
//!
//! ## Features
//!
//! - **Event Types**: Structured SSE events with ID, type, data, and retry fields
//! - **Async Streaming**: Tokio-based async event streaming
//! - **Keep-Alive**: Automatic keep-alive comments to maintain connections
//! - **Backpressure**: Channel-based flow control with configurable buffer sizes
//! - **Multiple Senders**: Clone-able sender for multi-producer scenarios
//!
//! ## Example
//!
//! ```rust,no_run
//! use archimedes_sse::{SseStream, SseEvent, SseConfig};
//! use std::time::Duration;
//!
//! async fn events_handler() -> (http::HeaderMap, SseStream) {
//!     let (sender, stream) = SseStream::with_config(
//!         SseConfig::new()
//!             .with_keep_alive(Duration::from_secs(15))
//!             .with_default_retry(Duration::from_secs(3))
//!     );
//!
//!     // Spawn a task to send events
//!     tokio::spawn(async move {
//!         let mut counter = 0;
//!         loop {
//!             counter += 1;
//!             let event = SseEvent::new(format!("Event {}", counter))
//!                 .id(counter.to_string())
//!                 .event("update");
//!
//!             if sender.send(event).await.is_err() {
//!                 break;
//!             }
//!
//!             tokio::time::sleep(Duration::from_secs(1)).await;
//!         }
//!     });
//!
//!     archimedes_sse::sse_response(stream)
//! }
//! ```
//!
//! ## SSE Protocol
//!
//! Server-Sent Events use a simple text-based protocol:
//!
//! ```text
//! id: 1
//! event: update
//! data: Hello, World!
//! retry: 3000
//!
//! ```
//!
//! - `id`: Event ID for reconnection tracking
//! - `event`: Event type (default: "message")
//! - `data`: Event payload (can span multiple lines)
//! - `retry`: Reconnection interval hint in milliseconds
//! - Comments start with `:` and are used for keep-alive

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod config;
mod error;
mod event;
mod stream;

pub use config::{SseConfig, SseConfigBuilder};
pub use error::{SseError, SseResult};
pub use event::{SseComment, SseEvent, SseItem};
pub use stream::{sse_response, SseSender, SseStream};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::config::SseConfig;
    pub use crate::error::{SseError, SseResult};
    pub use crate::event::{SseComment, SseEvent, SseItem};
    pub use crate::stream::{sse_response, SseSender, SseStream};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use futures_util::StreamExt;
    use std::time::Duration;

    #[tokio::test]
    async fn test_full_sse_workflow() {
        let config = SseConfig::new()
            .with_buffer_size(16)
            .with_keep_alive(Duration::from_secs(30));

        let (sender, mut stream) = SseStream::with_config(config);

        // Send various events
        sender.send(SseEvent::new("plain data")).await.unwrap();

        sender
            .send(SseEvent::new("typed event").event("notification").id("1"))
            .await
            .unwrap();

        #[derive(serde::Serialize)]
        struct Payload {
            message: String,
        }

        sender
            .send_json(&Payload {
                message: "json data".to_string(),
            })
            .await
            .unwrap();

        drop(sender);

        // Collect all output
        let mut output = String::new();

        // Skip initial retry
        let _ = stream.next().await;

        while let Some(Ok(bytes)) = stream.next().await {
            output.push_str(&String::from_utf8_lossy(&bytes));
        }

        assert!(output.contains("data: plain data"));
        assert!(output.contains("event: notification"));
        assert!(output.contains("id: 1"));
        assert!(output.contains("json data"));
    }

    #[tokio::test]
    async fn test_sse_response_returns_correct_headers() {
        let (_, stream) = SseStream::new();
        let (headers, _) = sse_response(stream);

        assert_eq!(
            headers.get(http::header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
        assert_eq!(
            headers.get(http::header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );
        assert_eq!(headers.get(http::header::CONNECTION).unwrap(), "keep-alive");
    }

    #[test]
    fn test_event_builder_pattern() {
        let event = SseEvent::new("data")
            .id("123")
            .event("custom")
            .retry(Duration::from_secs(5));

        assert_eq!(event.data(), "data");
        assert_eq!(event.id_value(), Some("123"));
        assert_eq!(event.event_type(), Some("custom"));
        assert_eq!(event.retry_interval(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_event_formatting() {
        let event = SseEvent::new("hello\nworld").id("1").event("test");

        let formatted = event.to_sse_string();

        assert!(formatted.contains("id: 1\n"));
        assert!(formatted.contains("event: test\n"));
        assert!(formatted.contains("data: hello\n"));
        assert!(formatted.contains("data: world\n"));
        assert!(formatted.ends_with("\n\n"));
    }
}
