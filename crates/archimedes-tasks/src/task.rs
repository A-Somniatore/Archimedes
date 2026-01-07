//! Task identity and status types.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unique identifier for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Generate a new unique task ID.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a task ID from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TaskId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Current status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is queued and waiting to run.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed with an error.
    Failed,
    /// Task was cancelled.
    Cancelled,
    /// Task timed out.
    TimedOut,
}

impl TaskStatus {
    /// Check if the task is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut
        )
    }

    /// Check if the task is running.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if the task is pending.
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Check if the task succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Check if the task failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed | Self::TimedOut)
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Information about a task.
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Unique task identifier.
    pub id: TaskId,
    /// Human-readable task name.
    pub name: String,
    /// Current status.
    pub status: TaskStatus,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// When the task started running.
    pub started_at: Option<DateTime<Utc>>,
    /// When the task completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration of execution (if completed).
    pub duration: Option<Duration>,
    /// Number of retry attempts.
    pub retry_count: u32,
    /// Error message if failed.
    pub error: Option<String>,
}

impl TaskInfo {
    /// Create new task info.
    pub fn new(id: TaskId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            duration: None,
            retry_count: 0,
            error: None,
        }
    }

    /// Mark as started.
    pub fn mark_started(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark as completed.
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        if let Some(started) = self.started_at {
            let now = Utc::now();
            self.duration = Some(Duration::from_millis(
                (now - started).num_milliseconds().max(0) as u64,
            ));
        }
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error.into());
        if let Some(started) = self.started_at {
            let now = Utc::now();
            self.duration = Some(Duration::from_millis(
                (now - started).num_milliseconds().max(0) as u64,
            ));
        }
    }

    /// Mark as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Mark as timed out.
    pub fn mark_timed_out(&mut self) {
        self.status = TaskStatus::TimedOut;
        self.completed_at = Some(Utc::now());
    }

    /// Increment retry count.
    pub fn increment_retries(&mut self) {
        self.retry_count += 1;
    }
}

/// Task execution statistics.
#[derive(Debug, Default)]
pub struct TaskStats {
    /// Total tasks spawned.
    pub spawned: AtomicU64,
    /// Tasks completed successfully.
    pub completed: AtomicU64,
    /// Tasks that failed.
    pub failed: AtomicU64,
    /// Tasks that were cancelled.
    pub cancelled: AtomicU64,
    /// Tasks that timed out.
    pub timed_out: AtomicU64,
    /// Currently running tasks.
    pub running: AtomicU64,
}

impl TaskStats {
    /// Create new empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a task spawn.
    pub fn record_spawn(&self) {
        self.spawned.fetch_add(1, Ordering::Relaxed);
        self.running.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task completion.
    pub fn record_completed(&self) {
        self.completed.fetch_add(1, Ordering::Relaxed);
        self.running.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a task failure.
    pub fn record_failed(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.running.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a task cancellation.
    pub fn record_cancelled(&self) {
        self.cancelled.fetch_add(1, Ordering::Relaxed);
        self.running.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a task timeout.
    pub fn record_timed_out(&self) {
        self.timed_out.fetch_add(1, Ordering::Relaxed);
        self.running.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get total spawned count.
    pub fn total_spawned(&self) -> u64 {
        self.spawned.load(Ordering::Relaxed)
    }

    /// Get completed count.
    pub fn total_completed(&self) -> u64 {
        self.completed.load(Ordering::Relaxed)
    }

    /// Get failed count.
    pub fn total_failed(&self) -> u64 {
        self.failed.load(Ordering::Relaxed)
    }

    /// Get cancelled count.
    pub fn total_cancelled(&self) -> u64 {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Get timed out count.
    pub fn total_timed_out(&self) -> u64 {
        self.timed_out.load(Ordering::Relaxed)
    }

    /// Get currently running count.
    pub fn currently_running(&self) -> u64 {
        self.running.load(Ordering::Relaxed)
    }

    /// Get success rate (0.0 to 1.0).
    pub fn success_rate(&self) -> f64 {
        let completed = self.total_completed();
        let total = completed + self.total_failed() + self.total_timed_out();
        if total == 0 {
            1.0
        } else {
            completed as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_new() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskId::new();
        let s = id.to_string();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_task_status_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(TaskStatus::TimedOut.is_terminal());
    }

    #[test]
    fn test_task_status_running() {
        assert!(TaskStatus::Running.is_running());
        assert!(!TaskStatus::Pending.is_running());
    }

    #[test]
    fn test_task_status_success_failure() {
        assert!(TaskStatus::Completed.is_success());
        assert!(TaskStatus::Failed.is_failure());
        assert!(TaskStatus::TimedOut.is_failure());
        assert!(!TaskStatus::Cancelled.is_failure());
    }

    #[test]
    fn test_task_info_lifecycle() {
        let id = TaskId::new();
        let mut info = TaskInfo::new(id, "test-task");

        assert_eq!(info.status, TaskStatus::Pending);
        assert!(info.started_at.is_none());

        info.mark_started();
        assert_eq!(info.status, TaskStatus::Running);
        assert!(info.started_at.is_some());

        info.mark_completed();
        assert_eq!(info.status, TaskStatus::Completed);
        assert!(info.completed_at.is_some());
        assert!(info.duration.is_some());
    }

    #[test]
    fn test_task_info_failure() {
        let mut info = TaskInfo::new(TaskId::new(), "failing-task");
        info.mark_started();
        info.mark_failed("something went wrong");

        assert_eq!(info.status, TaskStatus::Failed);
        assert_eq!(info.error, Some("something went wrong".to_string()));
    }

    #[test]
    fn test_task_stats() {
        let stats = TaskStats::new();

        stats.record_spawn();
        assert_eq!(stats.total_spawned(), 1);
        assert_eq!(stats.currently_running(), 1);

        stats.record_completed();
        assert_eq!(stats.total_completed(), 1);
        assert_eq!(stats.currently_running(), 0);
    }

    #[test]
    fn test_task_stats_success_rate() {
        let stats = TaskStats::new();

        // No tasks - 100% success
        assert_eq!(stats.success_rate(), 1.0);

        // 1 completed, 1 failed - 50% success
        stats.record_spawn();
        stats.record_completed();
        stats.record_spawn();
        stats.record_failed();

        assert_eq!(stats.success_rate(), 0.5);
    }
}
