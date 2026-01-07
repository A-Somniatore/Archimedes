//! # Archimedes Tasks
//!
//! Background task execution and scheduling for the Archimedes framework.
//!
//! This crate provides two main capabilities:
//!
//! 1. **Task Spawner**: Spawn background tasks with timeout, cancellation, and tracking
//! 2. **Cron Scheduler**: Schedule recurring jobs using cron expressions
//!
//! ## Task Spawner
//!
//! The spawner allows you to run background tasks with proper lifecycle management:
//!
//! ```rust,no_run
//! use archimedes_tasks::{Spawner, SpawnerConfig};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let spawner = Spawner::with_config(
//!         SpawnerConfig::new()
//!             .with_max_concurrent(100)
//!             .with_default_timeout(Duration::from_secs(60))
//!     );
//!
//!     // Spawn a tracked task
//!     let handle = spawner.spawn("process-data", async {
//!         // Do work
//!         42
//!     }).unwrap();
//!
//!     // Wait for result
//!     let result = handle.join().await.unwrap();
//!     assert_eq!(result, 42);
//!
//!     // Or spawn fire-and-forget
//!     spawner.spawn_detached("send-email", async {
//!         // Send email
//!     }).unwrap();
//! }
//! ```
//!
//! ## Cron Scheduler
//!
//! Schedule recurring jobs using standard cron expressions:
//!
//! ```rust,no_run
//! use archimedes_tasks::{Scheduler, SchedulerConfig};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let scheduler = Scheduler::new();
//!
//!     // Run every minute
//!     scheduler.register("cleanup", "0 * * * * *", || async {
//!         println!("Running cleanup");
//!     }).unwrap();
//!
//!     // Run at midnight
//!     scheduler.register("daily-report", "0 0 0 * * *", || async {
//!         println!("Generating daily report");
//!     }).unwrap();
//!
//!     // Start the scheduler
//!     scheduler.start().unwrap();
//!
//!     // ... run your application ...
//!
//!     // Stop gracefully
//!     scheduler.stop().await;
//! }
//! ```
//!
//! ## Cron Expression Format
//!
//! The cron format follows standard 6-field syntax:
//!
//! ```text
//! ┌───────────── second (0 - 59)
//! │ ┌───────────── minute (0 - 59)
//! │ │ ┌───────────── hour (0 - 23)
//! │ │ │ ┌───────────── day of month (1 - 31)
//! │ │ │ │ ┌───────────── month (1 - 12)
//! │ │ │ │ │ ┌───────────── day of week (0 - 6)
//! │ │ │ │ │ │
//! * * * * * *
//! ```
//!
//! Examples:
//! - `0 * * * * *` - Every minute
//! - `0 0 * * * *` - Every hour
//! - `0 0 0 * * *` - Every day at midnight
//! - `0 30 9 * * 1-5` - 9:30 AM on weekdays

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod error;
mod scheduler;
mod spawner;
mod task;

pub use error::{TaskError, TaskResult};
pub use scheduler::{JobFn, JobId, JobInfo, Scheduler, SchedulerConfig};
pub use spawner::{SharedSpawner, Spawner, SpawnerConfig, TaskHandle};
pub use task::{TaskId, TaskInfo, TaskStats, TaskStatus};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::error::{TaskError, TaskResult};
    pub use crate::scheduler::{JobId, JobInfo, Scheduler, SchedulerConfig};
    pub use crate::spawner::{SharedSpawner, Spawner, SpawnerConfig, TaskHandle};
    pub use crate::task::{TaskId, TaskInfo, TaskStats, TaskStatus};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_spawner_basic_workflow() {
        let spawner = Spawner::new();

        // Spawn and wait for result
        let handle = spawner.spawn("compute", async { 1 + 1 }).unwrap();
        let result = handle.join().await.unwrap();
        assert_eq!(result, 2);

        // Check stats
        assert_eq!(spawner.stats().total_completed(), 1);
    }

    #[tokio::test]
    async fn test_spawner_with_cancellation() {
        let spawner = Spawner::new();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut handle = spawner
            .spawn("long-task", async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                counter_clone.fetch_add(1, Ordering::Relaxed);
            })
            .unwrap();

        // Cancel before completion
        handle.cancel();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Counter should not have been incremented
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        assert_eq!(spawner.stats().total_cancelled(), 1);
    }

    #[tokio::test]
    async fn test_shared_spawner() {
        let spawner = SharedSpawner::new();

        let tasks: Vec<_> = (0..5)
            .map(|i| {
                let s = spawner.clone();
                tokio::spawn(async move { s.spawn(format!("task-{}", i), async move { i }).unwrap() })
            })
            .collect();

        for (i, task) in tasks.into_iter().enumerate() {
            let handle = task.await.unwrap();
            let result = handle.join().await.unwrap();
            assert_eq!(result, i);
        }
    }

    #[tokio::test]
    async fn test_scheduler_basic() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Register a job
        let id = scheduler
            .register("test", "* * * * * *", move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            })
            .unwrap();

        assert!(scheduler.get_job(id).is_some());

        // Run immediately
        scheduler.run_now(id).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(counter.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn test_task_status_transitions() {
        let mut info = TaskInfo::new(TaskId::new(), "test");

        assert!(info.status.is_pending());
        assert!(!info.status.is_terminal());

        info.mark_started();
        assert!(info.status.is_running());
        assert!(!info.status.is_terminal());

        info.mark_completed();
        assert!(info.status.is_success());
        assert!(info.status.is_terminal());
    }

    #[test]
    fn test_task_info_failure() {
        let mut info = TaskInfo::new(TaskId::new(), "failing");
        info.mark_started();
        info.mark_failed("error message");

        assert!(info.status.is_failure());
        assert!(info.status.is_terminal());
        assert_eq!(info.error, Some("error message".to_string()));
    }
}
