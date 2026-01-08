//! SSE stream types.
//!
//! This module provides types for creating and managing SSE streams.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc;
use tokio::time::{interval, Interval};

use crate::config::SseConfig;
use crate::error::{SseError, SseResult};
use crate::event::{SseComment, SseEvent, SseItem};

/// A sender for SSE events.
///
/// This type can be cloned and shared across tasks to send events
/// to the SSE stream.
#[derive(Debug, Clone)]
pub struct SseSender {
    tx: mpsc::Sender<SseItem>,
    closed: Arc<AtomicBool>,
    events_sent: Arc<AtomicU64>,
}

impl SseSender {
    /// Create a new sender.
    fn new(tx: mpsc::Sender<SseItem>) -> Self {
        Self {
            tx,
            closed: Arc::new(AtomicBool::new(false)),
            events_sent: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Send an event.
    pub async fn send(&self, event: SseEvent) -> SseResult<()> {
        if self.closed.load(Ordering::Acquire) {
            return Err(SseError::stream_closed("stream is closed"));
        }

        self.tx
            .send(SseItem::Event(event))
            .await
            .map_err(|_| SseError::send_failed("receiver dropped"))?;

        self.events_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Send a text message as an event.
    pub async fn send_text(&self, data: impl Into<String>) -> SseResult<()> {
        self.send(SseEvent::new(data)).await
    }

    /// Send a JSON value as an event.
    pub async fn send_json<T: serde::Serialize>(&self, value: &T) -> SseResult<()> {
        let event = SseEvent::json(value)?;
        self.send(event).await
    }

    /// Send an event with a specific type.
    pub async fn send_event(
        &self,
        event_type: impl Into<String>,
        data: impl Into<String>,
    ) -> SseResult<()> {
        let event = SseEvent::new(data).event(event_type);
        self.send(event).await
    }

    /// Send an event with ID.
    pub async fn send_with_id(
        &self,
        id: impl Into<String>,
        data: impl Into<String>,
    ) -> SseResult<()> {
        let event = SseEvent::new(data).id(id);
        self.send(event).await
    }

    /// Send a comment (for keepalive or debugging).
    pub async fn send_comment(&self, text: impl Into<String>) -> SseResult<()> {
        if self.closed.load(Ordering::Acquire) {
            return Err(SseError::stream_closed("stream is closed"));
        }

        self.tx
            .send(SseItem::Comment(SseComment::new(text)))
            .await
            .map_err(|_| SseError::send_failed("receiver dropped"))
    }

    /// Try to send an event without blocking.
    pub fn try_send(&self, event: SseEvent) -> SseResult<()> {
        use tokio::sync::mpsc::error::TrySendError;

        if self.closed.load(Ordering::Acquire) {
            return Err(SseError::stream_closed("stream is closed"));
        }

        match self.tx.try_send(SseItem::Event(event)) {
            Ok(()) => {
                self.events_sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(TrySendError::Full(_)) => Err(SseError::channel_full()),
            Err(TrySendError::Closed(_)) => Err(SseError::send_failed("receiver dropped")),
        }
    }

    /// Check if the stream is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire) || self.tx.is_closed()
    }

    /// Get the number of events sent.
    pub fn events_sent(&self) -> u64 {
        self.events_sent.load(Ordering::Relaxed)
    }

    /// Close the sender.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }
}

/// An SSE stream that can be used as an HTTP response body.
///
/// This stream yields bytes that are properly formatted SSE messages.
pub struct SseStream {
    rx: mpsc::Receiver<SseItem>,
    keep_alive: Option<Interval>,
    closed: Arc<AtomicBool>,
    initial_retry: Option<Duration>,
    sent_initial: bool,
}

impl SseStream {
    /// Create a new SSE stream with sender.
    pub fn new() -> (SseSender, Self) {
        Self::with_config(SseConfig::default())
    }

    /// Create a new SSE stream with configuration.
    pub fn with_config(config: SseConfig) -> (SseSender, Self) {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        let closed = Arc::new(AtomicBool::new(false));

        let keep_alive = config
            .keep_alive_interval
            .map(|duration| interval(duration));

        let sender = SseSender {
            tx,
            closed: closed.clone(),
            events_sent: Arc::new(AtomicU64::new(0)),
        };

        let stream = Self {
            rx,
            keep_alive,
            closed,
            initial_retry: config.default_retry,
            sent_initial: false,
        };

        (sender, stream)
    }

    /// Create a stream from a futures Stream.
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = SseItem> + Send + 'static,
    {
        Self::from_stream_with_config(stream, SseConfig::default())
    }

    /// Create a stream from a futures Stream with configuration.
    pub fn from_stream_with_config<S>(stream: S, config: SseConfig) -> Self
    where
        S: Stream<Item = SseItem> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        let closed = Arc::new(AtomicBool::new(false));
        let closed_clone = closed.clone();

        // Spawn a task to forward items from the stream
        tokio::spawn(async move {
            use futures_util::StreamExt;
            tokio::pin!(stream);

            while let Some(item) = stream.next().await {
                if tx.send(item).await.is_err() {
                    break;
                }
            }

            closed_clone.store(true, Ordering::Release);
        });

        let keep_alive = config
            .keep_alive_interval
            .map(|duration| interval(duration));

        Self {
            rx,
            keep_alive,
            closed,
            initial_retry: config.default_retry,
            sent_initial: false,
        }
    }

    /// Check if the stream is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Get the retry comment for initial connection.
    fn initial_retry_bytes(&self) -> Option<Bytes> {
        self.initial_retry
            .map(|duration| Bytes::from(format!("retry: {}\n\n", duration.as_millis())))
    }
}

impl Default for SseStream {
    fn default() -> Self {
        Self::new().1
    }
}

impl Stream for SseStream {
    type Item = Result<Bytes, SseError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Send initial retry hint if configured
        if !self.sent_initial {
            self.sent_initial = true;
            if let Some(bytes) = self.initial_retry_bytes() {
                return Poll::Ready(Some(Ok(bytes)));
            }
        }

        // Try to receive an item
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(Ok(item.to_bytes()))),
            Poll::Ready(None) => {
                self.closed.store(true, Ordering::Release);
                Poll::Ready(None)
            }
            Poll::Pending => {
                // Check keepalive timer
                if let Some(ref mut keepalive) = self.keep_alive {
                    if keepalive.poll_tick(cx).is_ready() {
                        return Poll::Ready(Some(Ok(Bytes::from(": keepalive\n\n"))));
                    }
                }
                Poll::Pending
            }
        }
    }
}

/// Create an SSE response from a stream of events.
///
/// This function returns the appropriate headers and body stream
/// for an SSE response.
pub fn sse_response(stream: SseStream) -> (http::HeaderMap, SseStream) {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("no-cache"),
    );
    headers.insert(
        http::header::CONNECTION,
        http::HeaderValue::from_static("keep-alive"),
    );
    // Disable buffering
    headers.insert("X-Accel-Buffering", http::HeaderValue::from_static("no"));

    (headers, stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn test_sender_send() {
        let (sender, mut stream) = SseStream::new();

        sender.send(SseEvent::new("hello")).await.unwrap();

        // Skip initial retry
        let _ = stream.next().await;

        let item = stream.next().await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&item).contains("data: hello"));
    }

    #[tokio::test]
    async fn test_sender_send_text() {
        let (sender, mut stream) = SseStream::new();

        sender.send_text("world").await.unwrap();

        // Skip initial retry
        let _ = stream.next().await;

        let item = stream.next().await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&item).contains("data: world"));
    }

    #[tokio::test]
    async fn test_sender_send_json() {
        #[derive(serde::Serialize)]
        struct Data {
            value: i32,
        }

        let (sender, mut stream) = SseStream::new();

        sender.send_json(&Data { value: 42 }).await.unwrap();

        // Skip initial retry
        let _ = stream.next().await;

        let item = stream.next().await.unwrap().unwrap();
        let text = String::from_utf8_lossy(&item);
        assert!(text.contains("42"));
    }

    #[tokio::test]
    async fn test_sender_send_event() {
        let (sender, mut stream) = SseStream::new();

        sender
            .send_event("notification", "new message")
            .await
            .unwrap();

        // Skip initial retry
        let _ = stream.next().await;

        let item = stream.next().await.unwrap().unwrap();
        let text = String::from_utf8_lossy(&item);
        assert!(text.contains("event: notification"));
        assert!(text.contains("data: new message"));
    }

    #[tokio::test]
    async fn test_sender_send_with_id() {
        let (sender, mut stream) = SseStream::new();

        sender.send_with_id("123", "data").await.unwrap();

        // Skip initial retry
        let _ = stream.next().await;

        let item = stream.next().await.unwrap().unwrap();
        let text = String::from_utf8_lossy(&item);
        assert!(text.contains("id: 123"));
        assert!(text.contains("data: data"));
    }

    #[tokio::test]
    async fn test_sender_try_send() {
        let (sender, mut stream) = SseStream::new();

        sender.try_send(SseEvent::new("immediate")).unwrap();

        // Skip initial retry
        let _ = stream.next().await;

        let item = stream.next().await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&item).contains("data: immediate"));
    }

    #[tokio::test]
    async fn test_sender_closed() {
        let (sender, stream) = SseStream::new();

        drop(stream);

        // Wait for close to propagate
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(sender.is_closed());
        assert!(sender.send_text("test").await.is_err());
    }

    #[tokio::test]
    async fn test_stream_initial_retry() {
        let config = SseConfig::new().with_default_retry(Duration::from_secs(5));
        let (_sender, mut stream) = SseStream::with_config(config);

        let item = stream.next().await.unwrap().unwrap();
        let text = String::from_utf8_lossy(&item);
        assert!(text.contains("retry: 5000"));
    }

    #[tokio::test]
    async fn test_stream_no_initial_retry() {
        let config = SseConfig::builder()
            .default_retry(Duration::ZERO)
            .no_keep_alive()
            .build();
        let config = SseConfig {
            default_retry: None,
            keep_alive_interval: None,
            ..config
        };
        let (sender, mut stream) = SseStream::with_config(config);

        sender.send_text("hello").await.unwrap();
        drop(sender);

        let item = stream.next().await.unwrap().unwrap();
        let text = String::from_utf8_lossy(&item);
        // Should be the actual event, not a retry directive
        assert!(text.contains("data: hello"));
    }

    #[tokio::test]
    async fn test_sse_response_headers() {
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
    }

    #[tokio::test]
    async fn test_events_sent_counter() {
        let (sender, _stream) = SseStream::new();

        assert_eq!(sender.events_sent(), 0);

        sender.send_text("one").await.unwrap();
        assert_eq!(sender.events_sent(), 1);

        sender.send_text("two").await.unwrap();
        assert_eq!(sender.events_sent(), 2);
    }

    #[tokio::test]
    async fn test_sender_close() {
        let (sender, _stream) = SseStream::new();

        assert!(!sender.is_closed());

        sender.close();

        assert!(sender.is_closed());
        assert!(sender.send_text("test").await.is_err());
    }

    #[tokio::test]
    async fn test_from_stream() {
        let items = vec![
            SseItem::event(SseEvent::new("one")),
            SseItem::event(SseEvent::new("two")),
        ];
        let source = futures_util::stream::iter(items);

        let config = SseConfig::builder().no_keep_alive().build();
        let config = SseConfig {
            default_retry: None,
            ..config
        };
        let mut stream = SseStream::from_stream_with_config(source, config);

        let item1 = stream.next().await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&item1).contains("data: one"));

        let item2 = stream.next().await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&item2).contains("data: two"));
    }
}
