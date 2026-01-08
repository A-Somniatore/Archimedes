//! Cron-based task scheduler.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use cron::Schedule;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::error::{TaskError, TaskResult};
use crate::spawner::{SharedSpawner, SpawnerConfig};

/// Type alias for async job functions.
pub type JobFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Unique identifier for a scheduled job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(Uuid);

impl JobId {
    /// Generate a new unique job ID.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about a scheduled job.
#[derive(Debug, Clone)]
pub struct JobInfo {
    /// Job ID.
    pub id: JobId,
    /// Job name.
    pub name: String,
    /// Cron expression.
    pub cron: String,
    /// Whether the job is enabled.
    pub enabled: bool,
    /// Last run time.
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled run time.
    pub next_run: Option<DateTime<Utc>>,
    /// Number of times the job has run.
    pub run_count: u64,
    /// Number of failed runs.
    pub fail_count: u64,
}

/// A scheduled job entry.
struct JobEntry {
    /// Job info.
    info: Arc<RwLock<JobInfo>>,
    /// Cron schedule.
    schedule: Schedule,
    /// Job function.
    func: JobFn,
}

/// Configuration for the scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Tick interval for checking scheduled jobs.
    pub tick_interval: Duration,
    /// Spawner configuration.
    pub spawner_config: SpawnerConfig,
    /// Whether to run missed jobs on startup.
    pub run_missed_on_startup: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(1),
            spawner_config: SpawnerConfig::default(),
            run_missed_on_startup: false,
        }
    }
}

impl SchedulerConfig {
    /// Create a new configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the tick interval.
    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = interval;
        self
    }

    /// Set the spawner configuration.
    pub fn with_spawner_config(mut self, config: SpawnerConfig) -> Self {
        self.spawner_config = config;
        self
    }

    /// Enable running missed jobs on startup.
    pub fn with_run_missed_on_startup(mut self) -> Self {
        self.run_missed_on_startup = true;
        self
    }
}

/// Cron-based job scheduler.
pub struct Scheduler {
    /// Configuration.
    config: SchedulerConfig,
    /// Registered jobs.
    jobs: DashMap<JobId, Arc<JobEntry>>,
    /// Task spawner.
    spawner: SharedSpawner,
    /// Whether the scheduler is running.
    running: AtomicBool,
    /// Shutdown signal sender.
    shutdown_tx: RwLock<Option<mpsc::Sender<()>>>,
    /// Scheduler loop handle.
    loop_handle: RwLock<Option<JoinHandle<()>>>,
    /// Total jobs executed.
    total_executed: Arc<AtomicU64>,
}

impl Scheduler {
    /// Create a new scheduler with default configuration.
    pub fn new() -> Self {
        Self::with_config(SchedulerConfig::default())
    }

    /// Create a new scheduler with custom configuration.
    pub fn with_config(config: SchedulerConfig) -> Self {
        let spawner = SharedSpawner::with_config(config.spawner_config.clone());
        Self {
            config,
            jobs: DashMap::new(),
            spawner,
            running: AtomicBool::new(false),
            shutdown_tx: RwLock::new(None),
            loop_handle: RwLock::new(None),
            total_executed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Check if the scheduler is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Get the number of registered jobs.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// Get total jobs executed.
    pub fn total_executed(&self) -> u64 {
        self.total_executed.load(Ordering::Relaxed)
    }

    /// Register a new scheduled job.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable job name
    /// * `cron_expr` - Cron expression (e.g., "0 0 * * * *" for every hour)
    /// * `func` - Async function to execute
    pub fn register<F, Fut>(
        &self,
        name: impl Into<String>,
        cron_expr: &str,
        func: F,
    ) -> TaskResult<JobId>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        let schedule: Schedule = cron_expr
            .parse()
            .map_err(|e: cron::error::Error| TaskError::invalid_cron(e.to_string()))?;

        let id = JobId::new();
        let next_run = schedule.upcoming(Utc).next();

        let info = JobInfo {
            id,
            name: name.clone(),
            cron: cron_expr.to_string(),
            enabled: true,
            last_run: None,
            next_run,
            run_count: 0,
            fail_count: 0,
        };

        let func: JobFn = Arc::new(move || Box::pin(func()));

        let entry = Arc::new(JobEntry {
            info: Arc::new(RwLock::new(info)),
            schedule,
            func,
        });

        self.jobs.insert(id, entry);
        info!(job_id = %id, job_name = %name, cron = %cron_expr, "registered scheduled job");

        Ok(id)
    }

    /// Unregister a job.
    pub fn unregister(&self, id: JobId) -> TaskResult<()> {
        self.jobs
            .remove(&id)
            .ok_or_else(|| TaskError::not_found(id))?;
        info!(job_id = %id, "unregistered scheduled job");
        Ok(())
    }

    /// Enable a job.
    pub fn enable(&self, id: JobId) -> TaskResult<()> {
        let entry = self.jobs.get(&id).ok_or_else(|| TaskError::not_found(id))?;
        entry.info.write().enabled = true;
        Ok(())
    }

    /// Disable a job.
    pub fn disable(&self, id: JobId) -> TaskResult<()> {
        let entry = self.jobs.get(&id).ok_or_else(|| TaskError::not_found(id))?;
        entry.info.write().enabled = false;
        Ok(())
    }

    /// Get job info.
    pub fn get_job(&self, id: JobId) -> Option<JobInfo> {
        self.jobs.get(&id).map(|e| e.info.read().clone())
    }

    /// List all jobs.
    pub fn list_jobs(&self) -> Vec<JobInfo> {
        self.jobs
            .iter()
            .map(|e| e.value().info.read().clone())
            .collect()
    }

    /// Run a job immediately (out of schedule).
    pub fn run_now(&self, id: JobId) -> TaskResult<()> {
        let entry = self.jobs.get(&id).ok_or_else(|| TaskError::not_found(id))?;

        let func = entry.func.clone();
        let info_lock = entry.value().info.clone();

        self.spawner
            .spawn_detached(format!("job-{}", id), async move {
                info_lock.write().last_run = Some(Utc::now());
                func().await;
                let mut info = info_lock.write();
                info.run_count += 1;
            })?;

        self.total_executed.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Start the scheduler.
    pub fn start(&self) -> TaskResult<()> {
        if self.running.swap(true, Ordering::AcqRel) {
            return Err(TaskError::invalid_config("scheduler already running"));
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.write() = Some(shutdown_tx);

        let jobs = self.jobs.clone();
        let spawner = self.spawner.clone();
        let tick_interval = self.config.tick_interval;
        let total_executed = self.total_executed.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let now = Utc::now();

                        for entry in jobs.iter() {
                            let job_entry = entry.value();
                            let info = job_entry.info.read();

                            if !info.enabled {
                                continue;
                            }

                            if let Some(next) = info.next_run {
                                if next <= now {
                                    drop(info);

                                    let id = entry.key();
                                    let func = job_entry.func.clone();
                                    let info_lock = job_entry.info.clone();

                                    debug!(job_id = %id, "executing scheduled job");

                                    if let Err(e) = spawner.spawn_detached(
                                        format!("job-{}", id),
                                        async move {
                                            func().await;
                                            let mut info = info_lock.write();
                                            info.run_count += 1;
                                        },
                                    ) {
                                        error!(job_id = %id, error = %e, "failed to spawn job");
                                        job_entry.info.write().fail_count += 1;
                                        continue;
                                    }

                                    total_executed.fetch_add(1, Ordering::Relaxed);

                                    // Update next run time
                                    let mut info = job_entry.info.write();
                                    info.last_run = Some(now);
                                    info.next_run = job_entry.schedule.upcoming(Utc).next();
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("scheduler received shutdown signal");
                        break;
                    }
                }
            }
        });

        *self.loop_handle.write() = Some(handle);
        info!("scheduler started");

        Ok(())
    }

    /// Stop the scheduler.
    pub async fn stop(&self) {
        if !self.running.swap(false, Ordering::AcqRel) {
            return;
        }

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().take() {
            let _ = tx.send(()).await;
        }

        // Wait for loop to finish
        if let Some(handle) = self.loop_handle.write().take() {
            let _ = handle.await;
        }

        // Shutdown spawner
        self.spawner.inner().shutdown(Duration::from_secs(30)).await;

        info!("scheduler stopped");
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        if self.running.load(Ordering::Acquire) {
            // Cancel the loop
            if let Some(tx) = self.shutdown_tx.write().take() {
                // Try to send, ignore if receiver dropped
                let _ = tx.try_send(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_job_id() {
        let id1 = JobId::new();
        let id2 = JobId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_scheduler_config() {
        let config = SchedulerConfig::new()
            .with_tick_interval(Duration::from_millis(500))
            .with_run_missed_on_startup();

        assert_eq!(config.tick_interval, Duration::from_millis(500));
        assert!(config.run_missed_on_startup);
    }

    #[test]
    fn test_register_job() {
        let scheduler = Scheduler::new();

        let id = scheduler
            .register("test-job", "0 * * * * *", || async {})
            .unwrap();

        let job = scheduler.get_job(id).unwrap();
        assert_eq!(job.name, "test-job");
        assert!(job.enabled);
        assert!(job.next_run.is_some());
    }

    #[test]
    fn test_register_invalid_cron() {
        let scheduler = Scheduler::new();

        let result = scheduler.register("bad-job", "invalid", || async {});
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_job() {
        let scheduler = Scheduler::new();

        let id = scheduler
            .register("temp-job", "0 * * * * *", || async {})
            .unwrap();

        assert!(scheduler.unregister(id).is_ok());
        assert!(scheduler.get_job(id).is_none());
    }

    #[test]
    fn test_enable_disable_job() {
        let scheduler = Scheduler::new();

        let id = scheduler
            .register("toggle-job", "0 * * * * *", || async {})
            .unwrap();

        scheduler.disable(id).unwrap();
        assert!(!scheduler.get_job(id).unwrap().enabled);

        scheduler.enable(id).unwrap();
        assert!(scheduler.get_job(id).unwrap().enabled);
    }

    #[test]
    fn test_list_jobs() {
        let scheduler = Scheduler::new();

        scheduler
            .register("job1", "0 * * * * *", || async {})
            .unwrap();
        scheduler
            .register("job2", "0 0 * * * *", || async {})
            .unwrap();

        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_run_now() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let id = scheduler
            .register("immediate", "0 0 0 1 1 *", move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            })
            .unwrap();

        scheduler.run_now(id).unwrap();

        // Wait for task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let scheduler = Scheduler::new();

        scheduler.start().unwrap();
        assert!(scheduler.is_running());

        scheduler.stop().await;
        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_double_start() {
        let scheduler = Scheduler::new();

        scheduler.start().unwrap();
        let result = scheduler.start();
        assert!(result.is_err());

        scheduler.stop().await;
    }

    #[tokio::test]
    async fn test_scheduled_execution() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let config = SchedulerConfig::new()
            .with_tick_interval(Duration::from_millis(100))
            .with_spawner_config(SpawnerConfig::new().without_timeout());
        let scheduler = Scheduler::with_config(config);

        // Register a job that runs every second
        scheduler
            .register("every-second", "* * * * * *", move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            })
            .unwrap();

        scheduler.start().unwrap();

        // Wait for at least one execution (2 seconds to be safe)
        tokio::time::sleep(Duration::from_millis(2500)).await;

        scheduler.stop().await;

        // Should have executed at least once
        assert!(counter.load(Ordering::Relaxed) >= 1);
    }
}
