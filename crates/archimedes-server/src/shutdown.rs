//! Graceful shutdown signal handling.
//!
//! This module provides utilities for handling shutdown signals
//! (SIGTERM, SIGINT) in a graceful manner, allowing in-flight
//! requests to complete before termination.
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_server::ShutdownSignal;
//!
//! // Wait for shutdown signal
//! let shutdown = ShutdownSignal::new();
//! shutdown.recv().await;
//!
//! // Or use with a timeout
//! use std::time::Duration;
//! tokio::select! {
//!     _ = shutdown.recv() => println!("Shutdown signal received"),
//!     _ = tokio::time::sleep(Duration::from_secs(60)) => println!("Timeout"),
//! }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::sync::broadcast;

/// A signal that can be used to trigger and await graceful shutdown.
///
/// `ShutdownSignal` provides a way to coordinate shutdown across
/// multiple tasks. It can be cloned and shared, and all clones
/// will receive the shutdown notification.
///
/// # Example
///
/// ```rust
/// use archimedes_server::ShutdownSignal;
///
/// let shutdown = ShutdownSignal::new();
///
/// // Clone for use in another task
/// let shutdown_clone = shutdown.clone();
///
/// // Trigger shutdown
/// shutdown.trigger();
///
/// // Check if shutdown was triggered
/// assert!(shutdown.is_shutdown());
/// ```
#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    /// Whether shutdown has been triggered
    triggered: Arc<AtomicBool>,

    /// Broadcast sender for notifying waiters
    sender: broadcast::Sender<()>,
}

impl ShutdownSignal {
    /// Creates a new shutdown signal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ShutdownSignal;
    ///
    /// let shutdown = ShutdownSignal::new();
    /// assert!(!shutdown.is_shutdown());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        Self {
            triggered: Arc::new(AtomicBool::new(false)),
            sender,
        }
    }

    /// Triggers the shutdown signal.
    ///
    /// This will notify all tasks waiting on this signal.
    /// Calling this multiple times is safe and idempotent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ShutdownSignal;
    ///
    /// let shutdown = ShutdownSignal::new();
    /// shutdown.trigger();
    /// assert!(shutdown.is_shutdown());
    /// ```
    pub fn trigger(&self) {
        // Only trigger once
        if self
            .triggered
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            // Ignore error if no receivers
            let _ = self.sender.send(());
        }
    }

    /// Returns `true` if shutdown has been triggered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ShutdownSignal;
    ///
    /// let shutdown = ShutdownSignal::new();
    /// assert!(!shutdown.is_shutdown());
    ///
    /// shutdown.trigger();
    /// assert!(shutdown.is_shutdown());
    /// ```
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    /// Returns a future that completes when shutdown is triggered.
    ///
    /// This can be awaited to wait for the shutdown signal.
    /// If shutdown has already been triggered, the future
    /// completes immediately.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_server::ShutdownSignal;
    ///
    /// let shutdown = ShutdownSignal::new();
    ///
    /// // In another task
    /// shutdown.trigger();
    ///
    /// // This will complete immediately
    /// shutdown.recv().await;
    /// ```
    pub fn recv(&self) -> ShutdownReceiver {
        ShutdownReceiver {
            triggered: Arc::clone(&self.triggered),
            receiver: self.sender.subscribe(),
        }
    }

    /// Creates a shutdown signal that listens for OS signals.
    ///
    /// This will trigger on SIGTERM or SIGINT (Ctrl+C).
    ///
    /// # Panics
    ///
    /// Panics if signal handlers cannot be registered.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_server::ShutdownSignal;
    ///
    /// let shutdown = ShutdownSignal::with_os_signals();
    ///
    /// // Will complete when SIGTERM or SIGINT is received
    /// shutdown.recv().await;
    /// ```
    #[must_use]
    pub fn with_os_signals() -> Self {
        let signal = Self::new();
        let signal_clone = signal.clone();

        tokio::spawn(async move {
            wait_for_os_signal().await;
            signal_clone.trigger();
        });

        signal
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// A future that completes when the shutdown signal is triggered.
///
/// Created by [`ShutdownSignal::recv()`].
pub struct ShutdownReceiver {
    triggered: Arc<AtomicBool>,
    receiver: broadcast::Receiver<()>,
}

impl Future for ShutdownReceiver {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Fast path: already triggered
        if self.triggered.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }

        // Wait for broadcast
        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Waits for an OS shutdown signal (SIGTERM or SIGINT).
///
/// On Unix systems, this waits for SIGTERM or SIGINT.
/// On other systems, this only waits for SIGINT (Ctrl+C).
async fn wait_for_os_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT, initiating graceful shutdown");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for Ctrl+C");
        tracing::info!("Received Ctrl+C, initiating graceful shutdown");
    }
}

/// A token that can be used to track active connections during shutdown.
///
/// When all `ConnectionToken` instances are dropped, the shutdown
/// process knows that all connections have been closed.
///
/// # Example
///
/// ```rust
/// use archimedes_server::shutdown::ConnectionTracker;
///
/// let tracker = ConnectionTracker::new();
///
/// // Acquire a token for each connection
/// let token = tracker.acquire();
/// assert_eq!(tracker.active_connections(), 1);
///
/// // Token is dropped, connection count decreases
/// drop(token);
/// assert_eq!(tracker.active_connections(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct ConnectionTracker {
    active: Arc<std::sync::atomic::AtomicUsize>,
    notify: Arc<tokio::sync::Notify>,
}

impl ConnectionTracker {
    /// Creates a new connection tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Acquires a connection token.
    ///
    /// The token should be held for the duration of the connection.
    /// When dropped, it decrements the active connection count.
    #[must_use]
    pub fn acquire(&self) -> ConnectionToken {
        self.active.fetch_add(1, Ordering::SeqCst);
        ConnectionToken {
            active: Arc::clone(&self.active),
            notify: Arc::clone(&self.notify),
        }
    }

    /// Returns the number of active connections.
    #[must_use]
    pub fn active_connections(&self) -> usize {
        self.active.load(Ordering::SeqCst)
    }

    /// Waits until all connections are closed.
    ///
    /// This completes immediately if there are no active connections.
    pub async fn wait_for_shutdown(&self) {
        while self.active.load(Ordering::SeqCst) > 0 {
            self.notify.notified().await;
        }
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// A token representing an active connection.
///
/// When dropped, decrements the connection count in the tracker.
#[derive(Debug)]
pub struct ConnectionToken {
    active: Arc<std::sync::atomic::AtomicUsize>,
    notify: Arc<tokio::sync::Notify>,
}

impl Drop for ConnectionToken {
    fn drop(&mut self) {
        let prev = self.active.fetch_sub(1, Ordering::SeqCst);
        // Notify if we were the last connection
        if prev == 1 {
            self.notify.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_shutdown_signal_new() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_shutdown());
    }

    #[test]
    fn test_shutdown_signal_trigger() {
        let signal = ShutdownSignal::new();
        signal.trigger();
        assert!(signal.is_shutdown());
    }

    #[test]
    fn test_shutdown_signal_trigger_idempotent() {
        let signal = ShutdownSignal::new();
        signal.trigger();
        signal.trigger();
        signal.trigger();
        assert!(signal.is_shutdown());
    }

    #[test]
    fn test_shutdown_signal_clone() {
        let signal1 = ShutdownSignal::new();
        let signal2 = signal1.clone();

        signal1.trigger();

        assert!(signal1.is_shutdown());
        assert!(signal2.is_shutdown());
    }

    #[tokio::test]
    async fn test_shutdown_recv_completes_when_triggered() {
        let signal = ShutdownSignal::new();
        let signal_clone = signal.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            signal_clone.trigger();
        });

        // Should complete when triggered
        tokio::time::timeout(Duration::from_secs(1), signal.recv())
            .await
            .expect("recv should complete");
    }

    #[tokio::test]
    async fn test_shutdown_recv_completes_immediately_if_triggered() {
        let signal = ShutdownSignal::new();
        signal.trigger();

        // Should complete immediately since already triggered
        tokio::time::timeout(Duration::from_millis(10), signal.recv())
            .await
            .expect("recv should complete immediately");
    }

    #[test]
    fn test_connection_tracker_new() {
        let tracker = ConnectionTracker::new();
        assert_eq!(tracker.active_connections(), 0);
    }

    #[test]
    fn test_connection_tracker_acquire() {
        let tracker = ConnectionTracker::new();
        let _token = tracker.acquire();
        assert_eq!(tracker.active_connections(), 1);
    }

    #[test]
    fn test_connection_tracker_multiple() {
        let tracker = ConnectionTracker::new();
        let token1 = tracker.acquire();
        let token2 = tracker.acquire();
        let token3 = tracker.acquire();

        assert_eq!(tracker.active_connections(), 3);

        drop(token1);
        assert_eq!(tracker.active_connections(), 2);

        drop(token2);
        assert_eq!(tracker.active_connections(), 1);

        drop(token3);
        assert_eq!(tracker.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_connection_tracker_wait_for_shutdown_immediate() {
        let tracker = ConnectionTracker::new();

        // Should complete immediately with no connections
        tokio::time::timeout(Duration::from_millis(10), tracker.wait_for_shutdown())
            .await
            .expect("wait_for_shutdown should complete immediately");
    }

    #[tokio::test]
    async fn test_connection_tracker_wait_for_shutdown_delayed() {
        let tracker = ConnectionTracker::new();
        let token = tracker.acquire();

        let tracker_clone = tracker.clone();
        let wait_handle = tokio::spawn(async move {
            tracker_clone.wait_for_shutdown().await;
        });

        // Drop token after a delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            drop(token);
        });

        // Wait should complete after token is dropped
        tokio::time::timeout(Duration::from_secs(1), wait_handle)
            .await
            .expect("wait should complete")
            .expect("task should not panic");
    }

    #[test]
    fn test_shutdown_signal_default() {
        let signal = ShutdownSignal::default();
        assert!(!signal.is_shutdown());
    }

    #[test]
    fn test_connection_tracker_default() {
        let tracker = ConnectionTracker::default();
        assert_eq!(tracker.active_connections(), 0);
    }
}
