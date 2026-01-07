//! SSE configuration.
//!
//! This module provides configuration types for SSE streams.

use std::time::Duration;

/// Configuration for SSE streams.
#[derive(Debug, Clone)]
pub struct SseConfig {
    /// Buffer size for the event channel.
    pub buffer_size: usize,
    /// Keep-alive interval (sends comment to keep connection alive).
    pub keep_alive_interval: Option<Duration>,
    /// Default retry interval to suggest to clients.
    pub default_retry: Option<Duration>,
    /// Maximum number of queued events before backpressure.
    pub max_queued_events: usize,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            buffer_size: 32,
            keep_alive_interval: Some(Duration::from_secs(15)),
            default_retry: Some(Duration::from_secs(3)),
            max_queued_events: 256,
        }
    }
}

impl SseConfig {
    /// Create a new SSE configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for configuration.
    pub fn builder() -> SseConfigBuilder {
        SseConfigBuilder::default()
    }

    /// Set the buffer size.
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set the keep-alive interval.
    pub fn with_keep_alive(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = Some(interval);
        self
    }

    /// Disable keep-alive.
    pub fn without_keep_alive(mut self) -> Self {
        self.keep_alive_interval = None;
        self
    }

    /// Set the default retry interval.
    pub fn with_default_retry(mut self, retry: Duration) -> Self {
        self.default_retry = Some(retry);
        self
    }

    /// Set the maximum queued events.
    pub fn with_max_queued_events(mut self, max: usize) -> Self {
        self.max_queued_events = max;
        self
    }
}

/// Builder for SSE configuration.
#[derive(Debug, Default)]
pub struct SseConfigBuilder {
    buffer_size: Option<usize>,
    keep_alive_interval: Option<Option<Duration>>,
    default_retry: Option<Option<Duration>>,
    max_queued_events: Option<usize>,
}

impl SseConfigBuilder {
    /// Set the buffer size.
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }

    /// Set the keep-alive interval.
    pub fn keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = Some(Some(interval));
        self
    }

    /// Disable keep-alive.
    pub fn no_keep_alive(mut self) -> Self {
        self.keep_alive_interval = Some(None);
        self
    }

    /// Set the default retry interval.
    pub fn default_retry(mut self, retry: Duration) -> Self {
        self.default_retry = Some(Some(retry));
        self
    }

    /// Set the maximum queued events.
    pub fn max_queued_events(mut self, max: usize) -> Self {
        self.max_queued_events = Some(max);
        self
    }

    /// Build the configuration.
    pub fn build(self) -> SseConfig {
        let mut config = SseConfig::default();

        if let Some(size) = self.buffer_size {
            config.buffer_size = size;
        }
        if let Some(interval) = self.keep_alive_interval {
            config.keep_alive_interval = interval;
        }
        if let Some(retry) = self.default_retry {
            config.default_retry = retry;
        }
        if let Some(max) = self.max_queued_events {
            config.max_queued_events = max;
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = SseConfig::default();
        assert_eq!(config.buffer_size, 32);
        assert!(config.keep_alive_interval.is_some());
        assert!(config.default_retry.is_some());
    }

    #[test]
    fn test_config_builder() {
        let config = SseConfig::builder()
            .buffer_size(64)
            .keep_alive_interval(Duration::from_secs(30))
            .default_retry(Duration::from_secs(5))
            .max_queued_events(512)
            .build();

        assert_eq!(config.buffer_size, 64);
        assert_eq!(config.keep_alive_interval, Some(Duration::from_secs(30)));
        assert_eq!(config.default_retry, Some(Duration::from_secs(5)));
        assert_eq!(config.max_queued_events, 512);
    }

    #[test]
    fn test_config_no_keep_alive() {
        let config = SseConfig::builder().no_keep_alive().build();
        assert!(config.keep_alive_interval.is_none());
    }

    #[test]
    fn test_config_fluent() {
        let config = SseConfig::new()
            .with_buffer_size(100)
            .with_keep_alive(Duration::from_secs(20))
            .with_default_retry(Duration::from_secs(10));

        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.keep_alive_interval, Some(Duration::from_secs(20)));
        assert_eq!(config.default_retry, Some(Duration::from_secs(10)));
    }
}
