//! Error types for background task operations.

use std::fmt;
use thiserror::Error;

/// Result type for task operations.
pub type TaskResult<T> = Result<T, TaskError>;

/// Errors that can occur during task operations.
#[derive(Debug, Error)]
pub enum TaskError {
    /// Task was cancelled before completion.
    #[error("task cancelled: {0}")]
    Cancelled(String),

    /// Task timed out.
    #[error("task timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Task panicked during execution.
    #[error("task panicked: {0}")]
    Panicked(String),

    /// Failed to spawn a task.
    #[error("failed to spawn task: {0}")]
    SpawnFailed(String),

    /// Task not found.
    #[error("task not found: {0}")]
    NotFound(String),

    /// Invalid task configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid cron expression.
    #[error("invalid cron expression: {0}")]
    InvalidCron(String),

    /// Task registry is full.
    #[error("task registry full, maximum {0} tasks")]
    RegistryFull(usize),

    /// Scheduler is not running.
    #[error("scheduler not running")]
    SchedulerNotRunning,

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl TaskError {
    /// Create a cancelled error.
    pub fn cancelled(reason: impl Into<String>) -> Self {
        Self::Cancelled(reason.into())
    }

    /// Create a timeout error.
    pub fn timeout(duration: std::time::Duration) -> Self {
        Self::Timeout(duration)
    }

    /// Create a panicked error.
    pub fn panicked(reason: impl Into<String>) -> Self {
        Self::Panicked(reason.into())
    }

    /// Create a spawn failed error.
    pub fn spawn_failed(reason: impl Into<String>) -> Self {
        Self::SpawnFailed(reason.into())
    }

    /// Create a not found error.
    pub fn not_found(id: impl fmt::Display) -> Self {
        Self::NotFound(id.to_string())
    }

    /// Create an invalid configuration error.
    pub fn invalid_config(reason: impl Into<String>) -> Self {
        Self::InvalidConfig(reason.into())
    }

    /// Create an invalid cron error.
    pub fn invalid_cron(reason: impl Into<String>) -> Self {
        Self::InvalidCron(reason.into())
    }

    /// Create a registry full error.
    pub fn registry_full(max: usize) -> Self {
        Self::RegistryFull(max)
    }

    /// Create an internal error.
    pub fn internal(reason: impl Into<String>) -> Self {
        Self::Internal(reason.into())
    }

    /// Check if the error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout(_) | Self::SpawnFailed(_) | Self::RegistryFull(_)
        )
    }

    /// Check if the error indicates the task should be retried.
    pub fn should_retry(&self) -> bool {
        matches!(self, Self::Timeout(_) | Self::SpawnFailed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_constructors() {
        let _ = TaskError::cancelled("user requested");
        let _ = TaskError::timeout(Duration::from_secs(30));
        let _ = TaskError::panicked("assertion failed");
        let _ = TaskError::spawn_failed("no capacity");
        let _ = TaskError::not_found("task-123");
        let _ = TaskError::invalid_config("missing name");
        let _ = TaskError::invalid_cron("* * * *");
        let _ = TaskError::registry_full(1000);
        let _ = TaskError::internal("unknown");
    }

    #[test]
    fn test_is_recoverable() {
        assert!(TaskError::timeout(Duration::from_secs(1)).is_recoverable());
        assert!(TaskError::spawn_failed("").is_recoverable());
        assert!(TaskError::registry_full(100).is_recoverable());
        assert!(!TaskError::cancelled("").is_recoverable());
        assert!(!TaskError::panicked("").is_recoverable());
    }

    #[test]
    fn test_should_retry() {
        assert!(TaskError::timeout(Duration::from_secs(1)).should_retry());
        assert!(TaskError::spawn_failed("").should_retry());
        assert!(!TaskError::registry_full(100).should_retry());
        assert!(!TaskError::cancelled("").should_retry());
    }

    #[test]
    fn test_error_display() {
        let err = TaskError::timeout(Duration::from_secs(30));
        assert!(err.to_string().contains("30"));
    }
}
