//! Background tasks - demonstrates async task spawning and scheduling.
//!
//! ## Features Demonstrated
//! - Task spawning (fire-and-forget)
//! - Scheduled tasks (cron-like)
//! - Task cancellation
//! - Task status tracking

use archimedes_tasks::{JobScheduler, TaskSpawner};
use std::sync::Arc;
use tracing::info;

use crate::routes::AppState;

/// Set up background tasks for the application.
///
/// # Example
/// ```
/// let spawner = setup_background_tasks(state).await;
/// spawner.spawn(my_async_task());
/// ```
pub async fn setup_background_tasks(state: Arc<AppState>) -> TaskSpawner {
    let spawner = TaskSpawner::new();
    let scheduler = JobScheduler::new();

    // -------------------------------------------------------------------------
    // PERIODIC CLEANUP TASK - Runs every hour
    // -------------------------------------------------------------------------
    scheduler.schedule("cleanup", "0 * * * *", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move {
                info!("Running periodic cleanup task");
                // In a real app, clean up expired sessions, temp files, etc.
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                info!("Cleanup complete");
                Ok(())
            }
        }
    });

    // -------------------------------------------------------------------------
    // METRICS AGGREGATION - Runs every 5 minutes
    // -------------------------------------------------------------------------
    scheduler.schedule("metrics", "*/5 * * * *", || async {
        info!("Aggregating metrics");
        // In a real app, aggregate and flush metrics to monitoring system
        Ok(())
    });

    // -------------------------------------------------------------------------
    // HEALTH CHECK - Runs every minute
    // -------------------------------------------------------------------------
    scheduler.schedule("health_check", "* * * * *", || async {
        info!("Running health check");
        // In a real app, check database connections, external services, etc.
        Ok(())
    });

    // Start the scheduler
    spawner.spawn(scheduler.run());

    spawner
}

/// Example of a fire-and-forget task.
///
/// # Example
/// ```
/// spawner.spawn(send_email_task("user@example.com", "Hello"));
/// ```
pub async fn send_email_task(to: &str, subject: &str) {
    info!(to = %to, subject = %subject, "Sending email");
    // Simulate email sending
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    info!(to = %to, "Email sent successfully");
}

/// Example of a task that reports progress.
///
/// # Example
/// ```
/// let handle = spawner.spawn_tracked(process_batch_task(items));
/// let status = handle.status().await;
/// ```
pub async fn process_batch_task(items: Vec<String>) -> Result<usize, String> {
    let total = items.len();
    let mut processed = 0;

    for item in items {
        info!(item = %item, progress = %format!("{}/{}", processed, total), "Processing item");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        processed += 1;
    }

    info!(total = %processed, "Batch processing complete");
    Ok(processed)
}

/// Example of a task with cancellation support.
///
/// # Example
/// ```
/// let (handle, cancel) = spawner.spawn_cancellable(long_running_task());
/// // Later...
/// cancel.cancel();
/// ```
pub async fn long_running_task(cancel: tokio::sync::watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut count = 0;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                count += 1;
                info!(count = %count, "Long-running task tick");
            }
            _ = async {
                while !*cancel.borrow() {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            } => {
                info!("Long-running task cancelled");
                break;
            }
        }

        if count >= 60 {
            info!("Long-running task completed naturally");
            break;
        }
    }
}

/// Example of a retry task.
///
/// # Example
/// ```
/// spawner.spawn(retry_task(|| fetch_external_data(), 3));
/// ```
pub async fn retry_task<F, Fut, T, E>(
    operation: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut last_error = None;

    while attempts < max_retries {
        attempts += 1;
        info!(attempt = %attempts, max = %max_retries, "Attempting operation");

        match operation().await {
            Ok(result) => {
                info!(attempts = %attempts, "Operation succeeded");
                return Ok(result);
            }
            Err(e) => {
                info!(attempt = %attempts, error = %e, "Operation failed, retrying");
                last_error = Some(e);
                
                // Exponential backoff
                let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempts));
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(last_error.unwrap())
}

/// Task metadata for tracking.
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
}

/// Task status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_email_task() {
        send_email_task("test@example.com", "Test").await;
        // Task completes without error
    }

    #[tokio::test]
    async fn test_process_batch_task() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = process_batch_task(items).await;
        assert_eq!(result, Ok(3));
    }

    #[tokio::test]
    async fn test_retry_task_success() {
        let result = retry_task(|| async { Ok::<_, String>(42) }, 3).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_task_failure() {
        let mut attempts = 0;
        let result = retry_task(
            || {
                attempts += 1;
                async { Err::<i32, _>("always fails".to_string()) }
            },
            3,
        )
        .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::Running.to_string(), "running");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
    }

    #[tokio::test]
    async fn test_cancellable_task() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        
        let handle = tokio::spawn(async move {
            long_running_task(rx).await;
        });

        // Let it run for a bit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Cancel it
        tx.send(true).ok();
        
        // Wait for completion
        handle.await.ok();
    }
}
