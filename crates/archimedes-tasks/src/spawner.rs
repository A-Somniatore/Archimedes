//! Task spawner for background execution.

use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::error::{TaskError, TaskResult};
use crate::task::{TaskId, TaskInfo, TaskStats, TaskStatus};

/// Configuration for the task spawner.
#[derive(Debug, Clone)]
pub struct SpawnerConfig {
    /// Maximum number of concurrent tasks.
    pub max_concurrent: usize,
    /// Default timeout for tasks.
    pub default_timeout: Option<Duration>,
    /// Maximum task registry size.
    pub max_registry_size: usize,
    /// Whether to track task history.
    pub track_history: bool,
    /// How long to keep completed tasks in registry.
    pub history_retention: Duration,
}

impl Default for SpawnerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 1000,
            default_timeout: Some(Duration::from_secs(300)), // 5 minutes
            max_registry_size: 10000,
            track_history: true,
            history_retention: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl SpawnerConfig {
    /// Create a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum concurrent tasks.
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set default timeout.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = Some(timeout);
        self
    }

    /// Disable default timeout.
    pub fn without_timeout(mut self) -> Self {
        self.default_timeout = None;
        self
    }

    /// Set maximum registry size.
    pub fn with_max_registry_size(mut self, size: usize) -> Self {
        self.max_registry_size = size;
        self
    }

    /// Set history retention duration.
    pub fn with_history_retention(mut self, retention: Duration) -> Self {
        self.history_retention = retention;
        self
    }

    /// Disable history tracking.
    pub fn without_history(mut self) -> Self {
        self.track_history = false;
        self
    }
}

/// A handle to a spawned task.
#[derive(Debug)]
pub struct TaskHandle<T> {
    /// Task ID.
    id: TaskId,
    /// Join handle for the task.
    handle: JoinHandle<Option<T>>,
    /// Cancel sender.
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl<T> TaskHandle<T> {
    /// Get the task ID.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Check if the task is finished.
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// Cancel the task.
    pub fn cancel(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }
    }

    /// Abort the task immediately.
    pub fn abort(&self) {
        self.handle.abort();
    }

    /// Wait for the task to complete.
    ///
    /// Returns the task result if successful, or an error if the task
    /// was cancelled, timed out, or panicked.
    pub async fn join(self) -> TaskResult<T> {
        match self.handle.await {
            Ok(Some(result)) => Ok(result),
            Ok(None) => Err(TaskError::cancelled("task was cancelled or timed out")),
            Err(e) => {
                if e.is_cancelled() {
                    Err(TaskError::cancelled("task was aborted"))
                } else if e.is_panic() {
                    Err(TaskError::panicked("task panicked"))
                } else {
                    Err(TaskError::internal(e.to_string()))
                }
            }
        }
    }
}

/// Background task spawner with DI support.
#[derive(Debug)]
pub struct Spawner {
    /// Configuration.
    config: SpawnerConfig,
    /// Task registry.
    registry: DashMap<TaskId, Arc<RwLock<TaskInfo>>>,
    /// Statistics.
    stats: Arc<TaskStats>,
    /// Currently running count.
    running: Arc<AtomicU64>,
    /// Whether the spawner is shutdown.
    shutdown: AtomicBool,
}

impl Spawner {
    /// Create a new spawner with default configuration.
    pub fn new() -> Self {
        Self::with_config(SpawnerConfig::default())
    }

    /// Create a new spawner with custom configuration.
    pub fn with_config(config: SpawnerConfig) -> Self {
        Self {
            config,
            registry: DashMap::new(),
            stats: Arc::new(TaskStats::new()),
            running: Arc::new(AtomicU64::new(0)),
            shutdown: AtomicBool::new(false),
        }
    }

    /// Check if the spawner is shutdown.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Get the current number of running tasks.
    pub fn running_count(&self) -> u64 {
        self.running.load(Ordering::Relaxed)
    }

    /// Get task statistics.
    pub fn stats(&self) -> &TaskStats {
        &self.stats
    }

    /// Get task info by ID.
    pub fn get_task(&self, id: TaskId) -> Option<TaskInfo> {
        self.registry.get(&id).map(|v| v.read().clone())
    }

    /// List all tasks.
    pub fn list_tasks(&self) -> Vec<TaskInfo> {
        self.registry
            .iter()
            .map(|entry| entry.value().read().clone())
            .collect()
    }

    /// List tasks by status.
    pub fn list_tasks_by_status(&self, status: TaskStatus) -> Vec<TaskInfo> {
        self.registry
            .iter()
            .filter_map(|entry| {
                let info = entry.value().read().clone();
                if info.status == status {
                    Some(info)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Spawn a background task.
    pub fn spawn<F, T>(&self, name: impl Into<String>, task: F) -> TaskResult<TaskHandle<T>>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.spawn_with_timeout(name, task, self.config.default_timeout)
    }

    /// Spawn a task with a specific timeout.
    pub fn spawn_with_timeout<F, T>(
        &self,
        name: impl Into<String>,
        task: F,
        timeout: Option<Duration>,
    ) -> TaskResult<TaskHandle<T>>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(TaskError::spawn_failed("spawner is shutdown"));
        }

        let current_running = self.running.load(Ordering::Relaxed);
        if current_running >= self.config.max_concurrent as u64 {
            return Err(TaskError::spawn_failed(format!(
                "max concurrent tasks ({}) reached",
                self.config.max_concurrent
            )));
        }

        if self.registry.len() >= self.config.max_registry_size {
            // Try to clean up old completed tasks
            self.cleanup_completed_tasks();

            if self.registry.len() >= self.config.max_registry_size {
                return Err(TaskError::registry_full(self.config.max_registry_size));
            }
        }

        let name = name.into();
        let id = TaskId::new();
        let info = Arc::new(RwLock::new(TaskInfo::new(id, name.clone())));

        // Create cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Clone for the task
        let info_clone = info.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();

        // Register the task
        if self.config.track_history {
            self.registry.insert(id, info);
        }

        self.running.fetch_add(1, Ordering::Relaxed);
        self.stats.record_spawn();

        debug!(task_id = %id, task_name = %name, "spawning background task");

        // Spawn the task
        let handle = tokio::spawn(async move {
            info_clone.write().mark_started();

            let result = if let Some(timeout_duration) = timeout {
                tokio::select! {
                    result = task => Some(result),
                    _ = tokio::time::sleep(timeout_duration) => {
                        warn!(task_id = %id, "task timed out");
                        info_clone.write().mark_timed_out();
                        stats.record_timed_out();
                        running.fetch_sub(1, Ordering::Relaxed);
                        return None;
                    }
                    _ = cancel_rx => {
                        info!(task_id = %id, "task cancelled");
                        info_clone.write().mark_cancelled();
                        stats.record_cancelled();
                        running.fetch_sub(1, Ordering::Relaxed);
                        return None;
                    }
                }
            } else {
                tokio::select! {
                    result = task => Some(result),
                    _ = cancel_rx => {
                        info!(task_id = %id, "task cancelled");
                        info_clone.write().mark_cancelled();
                        stats.record_cancelled();
                        running.fetch_sub(1, Ordering::Relaxed);
                        return None;
                    }
                }
            };

            if let Some(result) = result {
                info_clone.write().mark_completed();
                stats.record_completed();
                running.fetch_sub(1, Ordering::Relaxed);
                debug!(task_id = %id, "task completed");
                Some(result)
            } else {
                None
            }
        });

        Ok(TaskHandle {
            id,
            handle,
            cancel_tx: Some(cancel_tx),
        })
    }

    /// Spawn a fire-and-forget task (no result tracking).
    pub fn spawn_detached<F>(&self, name: impl Into<String>, task: F) -> TaskResult<TaskId>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(TaskError::spawn_failed("spawner is shutdown"));
        }

        let current_running = self.running.load(Ordering::Relaxed);
        if current_running >= self.config.max_concurrent as u64 {
            return Err(TaskError::spawn_failed(format!(
                "max concurrent tasks ({}) reached",
                self.config.max_concurrent
            )));
        }

        if self.registry.len() >= self.config.max_registry_size {
            self.cleanup_completed_tasks();

            if self.registry.len() >= self.config.max_registry_size {
                return Err(TaskError::registry_full(self.config.max_registry_size));
            }
        }

        let name = name.into();
        let id = TaskId::new();
        let info = Arc::new(RwLock::new(TaskInfo::new(id, name.clone())));

        let info_clone = info.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();
        let timeout = self.config.default_timeout;

        if self.config.track_history {
            self.registry.insert(id, info);
        }

        self.running.fetch_add(1, Ordering::Relaxed);
        self.stats.record_spawn();

        debug!(task_id = %id, task_name = %name, "spawning detached background task");

        tokio::spawn(async move {
            info_clone.write().mark_started();

            let completed = if let Some(timeout_duration) = timeout {
                tokio::select! {
                    _ = task => true,
                    _ = tokio::time::sleep(timeout_duration) => {
                        warn!(task_id = %id, "detached task timed out");
                        info_clone.write().mark_timed_out();
                        stats.record_timed_out();
                        running.fetch_sub(1, Ordering::Relaxed);
                        false
                    }
                }
            } else {
                task.await;
                true
            };

            if completed {
                info_clone.write().mark_completed();
                stats.record_completed();
                running.fetch_sub(1, Ordering::Relaxed);
                debug!(task_id = %id, "detached task completed");
            }
        });

        Ok(id)
    }

    /// Clean up completed tasks older than retention period.
    fn cleanup_completed_tasks(&self) {
        let retention = self.config.history_retention;
        let now = chrono::Utc::now();

        self.registry.retain(|_, info| {
            let info = info.read();
            if info.status.is_terminal() {
                if let Some(completed_at) = info.completed_at {
                    let age = now - completed_at;
                    return age.num_seconds() < retention.as_secs() as i64;
                }
            }
            true
        });
    }

    /// Shutdown the spawner gracefully.
    pub async fn shutdown(&self, timeout: Duration) {
        info!("shutting down task spawner");
        self.shutdown.store(true, Ordering::Release);

        // Wait for running tasks to complete
        let deadline = tokio::time::Instant::now() + timeout;
        while self.running.load(Ordering::Relaxed) > 0 {
            if tokio::time::Instant::now() >= deadline {
                warn!(
                    running = self.running.load(Ordering::Relaxed),
                    "shutdown timeout reached, tasks still running"
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!("task spawner shutdown complete");
    }
}

impl Default for Spawner {
    fn default() -> Self {
        Self::new()
    }
}

/// A shared spawner that can be cloned.
#[derive(Debug, Clone)]
pub struct SharedSpawner(Arc<Spawner>);

impl SharedSpawner {
    /// Create a new shared spawner.
    pub fn new() -> Self {
        Self(Arc::new(Spawner::new()))
    }

    /// Create a shared spawner with configuration.
    pub fn with_config(config: SpawnerConfig) -> Self {
        Self(Arc::new(Spawner::with_config(config)))
    }

    /// Get the inner spawner.
    pub fn inner(&self) -> &Spawner {
        &self.0
    }

    /// Spawn a background task.
    pub fn spawn<F, T>(&self, name: impl Into<String>, task: F) -> TaskResult<TaskHandle<T>>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.0.spawn(name, task)
    }

    /// Spawn a fire-and-forget task.
    pub fn spawn_detached<F>(&self, name: impl Into<String>, task: F) -> TaskResult<TaskId>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.0.spawn_detached(name, task)
    }
}

impl Default for SharedSpawner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawner_config_defaults() {
        let config = SpawnerConfig::default();
        assert_eq!(config.max_concurrent, 1000);
        assert!(config.default_timeout.is_some());
    }

    #[test]
    fn test_spawner_config_builder() {
        let config = SpawnerConfig::new()
            .with_max_concurrent(500)
            .with_default_timeout(Duration::from_secs(60))
            .with_max_registry_size(5000);

        assert_eq!(config.max_concurrent, 500);
        assert_eq!(config.default_timeout, Some(Duration::from_secs(60)));
        assert_eq!(config.max_registry_size, 5000);
    }

    #[tokio::test]
    async fn test_spawn_task() {
        let spawner = Spawner::new();

        let handle = spawner.spawn("test", async { 42 }).unwrap();
        let result = handle.join().await.unwrap();

        assert_eq!(result, 42);
        assert_eq!(spawner.stats().total_completed(), 1);
    }

    #[tokio::test]
    async fn test_spawn_detached() {
        let spawner = Spawner::new();

        let id = spawner.spawn_detached("detached", async {}).unwrap();

        // Wait for task to complete
        tokio::time::sleep(Duration::from_millis(50)).await;

        let info = spawner.get_task(id);
        assert!(info.is_some());
    }

    #[tokio::test]
    async fn test_task_cancel() {
        let spawner = Spawner::new();

        let mut handle = spawner
            .spawn("long-task", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                42
            })
            .unwrap();

        // Cancel immediately
        handle.cancel();

        // Wait a bit for cancellation to propagate
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(spawner.stats().total_cancelled(), 1);
    }

    #[tokio::test]
    async fn test_task_timeout() {
        let spawner = Spawner::with_config(
            SpawnerConfig::new().with_default_timeout(Duration::from_millis(50)),
        );

        let handle = spawner
            .spawn("timeout-task", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                42
            })
            .unwrap();

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(handle.is_finished());
        assert_eq!(spawner.stats().total_timed_out(), 1);
    }

    #[tokio::test]
    async fn test_spawner_max_concurrent() {
        let spawner = Spawner::with_config(SpawnerConfig::new().with_max_concurrent(2));

        // Spawn two tasks
        let _h1 = spawner
            .spawn("task1", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .unwrap();
        let _h2 = spawner
            .spawn("task2", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .unwrap();

        // Third should fail
        let result = spawner.spawn("task3", async {});
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spawner_shutdown() {
        let spawner = Spawner::new();

        spawner.spawn_detached("task", async {}).unwrap();

        spawner.shutdown(Duration::from_secs(1)).await;

        assert!(spawner.is_shutdown());
        assert!(spawner.spawn("new-task", async {}).is_err());
    }

    #[tokio::test]
    async fn test_shared_spawner() {
        let spawner = SharedSpawner::new();
        let spawner2 = spawner.clone();

        let h1 = spawner.spawn("task1", async { 1 }).unwrap();
        let h2 = spawner2.spawn("task2", async { 2 }).unwrap();

        assert_eq!(h1.join().await.unwrap(), 1);
        assert_eq!(h2.join().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let spawner = Spawner::new();

        spawner.spawn_detached("task1", async {}).unwrap();
        spawner.spawn_detached("task2", async {}).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let tasks = spawner.list_tasks();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_list_tasks_by_status() {
        let spawner = Spawner::with_config(SpawnerConfig::new().without_timeout());

        spawner.spawn_detached("completed", async {}).unwrap();
        let _long = spawner
            .spawn("running", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .unwrap();

        // Give more time for the short task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        let running = spawner.list_tasks_by_status(TaskStatus::Running);
        let completed = spawner.list_tasks_by_status(TaskStatus::Completed);

        assert_eq!(running.len(), 1);
        assert_eq!(completed.len(), 1);
    }
}
