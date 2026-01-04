//! Health check endpoints.
//!
//! This module provides health and readiness check functionality
//! for the Archimedes server. These endpoints are essential for
//! Kubernetes deployments and load balancer health probes.
//!
//! # Endpoints
//!
//! - `/health` - Liveness probe: Is the server running?
//! - `/ready` - Readiness probe: Is the server ready to accept traffic?
//!
//! # Example
//!
//! ```rust
//! use archimedes_server::{HealthCheck, HealthStatus, ReadinessCheck};
//! use std::time::Instant;
//!
//! // Create health check
//! let health = HealthCheck::new("my-service", "1.0.0");
//!
//! // Get health status
//! let status = health.status();
//! assert_eq!(status.service(), "my-service");
//! assert_eq!(status.version(), "1.0.0");
//!
//! // Create readiness check with custom checks
//! let readiness = ReadinessCheck::new()
//!     .add_check("database", || true)
//!     .add_check("cache", || true);
//!
//! assert!(readiness.is_ready());
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Health status response.
///
/// Returned by the `/health` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthStatus {
    /// Service status ("healthy", "degraded", "unhealthy")
    status: String,

    /// Service name
    service: String,

    /// Service version
    version: String,

    /// Server uptime in seconds
    uptime_seconds: u64,
}

impl HealthStatus {
    /// Creates a new health status.
    #[must_use]
    pub fn new(
        status: impl Into<String>,
        service: impl Into<String>,
        version: impl Into<String>,
        uptime: Duration,
    ) -> Self {
        Self {
            status: status.into(),
            service: service.into(),
            version: version.into(),
            uptime_seconds: uptime.as_secs(),
        }
    }

    /// Creates a healthy status.
    #[must_use]
    pub fn healthy(service: impl Into<String>, version: impl Into<String>, uptime: Duration) -> Self {
        Self::new("healthy", service, version, uptime)
    }

    /// Creates a degraded status.
    #[must_use]
    pub fn degraded(service: impl Into<String>, version: impl Into<String>, uptime: Duration) -> Self {
        Self::new("degraded", service, version, uptime)
    }

    /// Creates an unhealthy status.
    #[must_use]
    pub fn unhealthy(
        service: impl Into<String>,
        version: impl Into<String>,
        uptime: Duration,
    ) -> Self {
        Self::new("unhealthy", service, version, uptime)
    }

    /// Returns the status string.
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Returns the service name.
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Returns the service version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the uptime in seconds.
    #[must_use]
    pub fn uptime_seconds(&self) -> u64 {
        self.uptime_seconds
    }

    /// Returns whether the status is healthy.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.status == "healthy"
    }

    /// Returns whether the status is degraded.
    #[must_use]
    pub fn is_degraded(&self) -> bool {
        self.status == "degraded"
    }

    /// Returns whether the status is unhealthy.
    #[must_use]
    pub fn is_unhealthy(&self) -> bool {
        self.status == "unhealthy"
    }
}

/// Health check handler.
///
/// Provides liveness probe functionality for the server.
/// The health check is always "healthy" if the server is running.
///
/// # Example
///
/// ```rust
/// use archimedes_server::HealthCheck;
///
/// let health = HealthCheck::new("my-service", "1.0.0");
/// let status = health.status();
///
/// assert!(status.is_healthy());
/// assert_eq!(status.service(), "my-service");
/// ```
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Service name
    service: String,

    /// Service version
    version: String,

    /// Server start time
    start_time: Instant,
}

impl HealthCheck {
    /// Creates a new health check.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name to report in health status
    /// * `version` - Service version to report in health status
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::HealthCheck;
    ///
    /// let health = HealthCheck::new("api-service", "2.1.0");
    /// ```
    #[must_use]
    pub fn new(service: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            version: version.into(),
            start_time: Instant::now(),
        }
    }

    /// Returns the current health status.
    ///
    /// The server is considered healthy if it's running.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::HealthCheck;
    ///
    /// let health = HealthCheck::new("my-service", "1.0.0");
    /// let status = health.status();
    ///
    /// assert!(status.is_healthy());
    /// ```
    #[must_use]
    pub fn status(&self) -> HealthStatus {
        let uptime = self.start_time.elapsed();
        HealthStatus::healthy(&self.service, &self.version, uptime)
    }

    /// Returns the server uptime.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Returns the service name.
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Returns the service version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Readiness status response.
///
/// Returned by the `/ready` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessStatus {
    /// Whether the service is ready
    ready: bool,

    /// Individual check results
    checks: HashMap<String, bool>,
}

impl ReadinessStatus {
    /// Creates a new readiness status.
    #[must_use]
    pub fn new(ready: bool, checks: HashMap<String, bool>) -> Self {
        Self { ready, checks }
    }

    /// Returns whether the service is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Returns the individual check results.
    #[must_use]
    pub fn checks(&self) -> &HashMap<String, bool> {
        &self.checks
    }

    /// Returns the result of a specific check.
    #[must_use]
    pub fn check(&self, name: &str) -> Option<bool> {
        self.checks.get(name).copied()
    }
}

/// A readiness check function.
type ReadinessCheckFn = Arc<dyn Fn() -> bool + Send + Sync>;

/// Readiness check handler.
///
/// Provides readiness probe functionality with customizable checks.
/// The server is "ready" when all registered checks pass.
///
/// # Example
///
/// ```rust
/// use archimedes_server::ReadinessCheck;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use std::sync::Arc;
///
/// // Create with custom checks
/// let db_ready = Arc::new(AtomicBool::new(true));
/// let db_ready_clone = Arc::clone(&db_ready);
///
/// let readiness = ReadinessCheck::new()
///     .add_check("database", move || db_ready_clone.load(Ordering::SeqCst));
///
/// assert!(readiness.is_ready());
///
/// // Simulate database going down
/// db_ready.store(false, Ordering::SeqCst);
/// assert!(!readiness.is_ready());
/// ```
#[derive(Clone)]
pub struct ReadinessCheck {
    /// Registered checks
    checks: Vec<(String, ReadinessCheckFn)>,

    /// Manual ready override (for graceful shutdown)
    ready_override: Arc<AtomicBool>,
}

impl std::fmt::Debug for ReadinessCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadinessCheck")
            .field("checks", &self.checks.iter().map(|(n, _)| n).collect::<Vec<_>>())
            .field("ready_override", &self.ready_override)
            .finish()
    }
}

impl ReadinessCheck {
    /// Creates a new readiness check with no checks.
    ///
    /// By default, the service is ready when there are no checks.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ReadinessCheck;
    ///
    /// let readiness = ReadinessCheck::new();
    /// assert!(readiness.is_ready());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            ready_override: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Adds a check to the readiness handler.
    ///
    /// The check function should return `true` if the component is ready.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the check (for reporting)
    /// * `check` - Function that returns `true` if ready
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ReadinessCheck;
    ///
    /// let readiness = ReadinessCheck::new()
    ///     .add_check("config_loaded", || true)
    ///     .add_check("cache_warm", || true);
    ///
    /// assert!(readiness.is_ready());
    /// ```
    #[must_use]
    pub fn add_check<F>(mut self, name: impl Into<String>, check: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.checks.push((name.into(), Arc::new(check)));
        self
    }

    /// Returns whether the service is ready.
    ///
    /// The service is ready when:
    /// 1. The ready override is true (not shutting down)
    /// 2. All registered checks pass
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ReadinessCheck;
    ///
    /// let readiness = ReadinessCheck::new()
    ///     .add_check("check1", || true)
    ///     .add_check("check2", || false); // This fails
    ///
    /// assert!(!readiness.is_ready());
    /// ```
    #[must_use]
    pub fn is_ready(&self) -> bool {
        if !self.ready_override.load(Ordering::SeqCst) {
            return false;
        }

        self.checks.iter().all(|(_, check)| check())
    }

    /// Returns the full readiness status with individual check results.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ReadinessCheck;
    ///
    /// let readiness = ReadinessCheck::new()
    ///     .add_check("database", || true)
    ///     .add_check("cache", || false);
    ///
    /// let status = readiness.status();
    /// assert!(!status.is_ready());
    /// assert_eq!(status.check("database"), Some(true));
    /// assert_eq!(status.check("cache"), Some(false));
    /// ```
    #[must_use]
    pub fn status(&self) -> ReadinessStatus {
        let checks: HashMap<String, bool> = self
            .checks
            .iter()
            .map(|(name, check)| (name.clone(), check()))
            .collect();

        let ready = self.ready_override.load(Ordering::SeqCst)
            && checks.values().all(|&v| v);

        ReadinessStatus::new(ready, checks)
    }

    /// Sets the ready override.
    ///
    /// This can be used to mark the service as not ready during
    /// graceful shutdown, even if all checks pass.
    ///
    /// # Arguments
    ///
    /// * `ready` - Whether to allow readiness checks to pass
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::ReadinessCheck;
    ///
    /// let readiness = ReadinessCheck::new();
    /// assert!(readiness.is_ready());
    ///
    /// // Start graceful shutdown
    /// readiness.set_ready(false);
    /// assert!(!readiness.is_ready());
    /// ```
    pub fn set_ready(&self, ready: bool) {
        self.ready_override.store(ready, Ordering::SeqCst);
    }

    /// Returns the number of registered checks.
    #[must_use]
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
}

impl Default for ReadinessCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_healthy() {
        let status = HealthStatus::healthy("test", "1.0.0", Duration::from_secs(60));

        assert!(status.is_healthy());
        assert!(!status.is_degraded());
        assert!(!status.is_unhealthy());
        assert_eq!(status.service(), "test");
        assert_eq!(status.version(), "1.0.0");
        assert_eq!(status.uptime_seconds(), 60);
    }

    #[test]
    fn test_health_status_degraded() {
        let status = HealthStatus::degraded("test", "1.0.0", Duration::from_secs(30));

        assert!(!status.is_healthy());
        assert!(status.is_degraded());
        assert!(!status.is_unhealthy());
    }

    #[test]
    fn test_health_status_unhealthy() {
        let status = HealthStatus::unhealthy("test", "1.0.0", Duration::from_secs(10));

        assert!(!status.is_healthy());
        assert!(!status.is_degraded());
        assert!(status.is_unhealthy());
    }

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus::healthy("api", "2.0.0", Duration::from_secs(3600));
        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"service\":\"api\""));
        assert!(json.contains("\"version\":\"2.0.0\""));
        assert!(json.contains("\"uptime_seconds\":3600"));
    }

    #[test]
    fn test_health_check_new() {
        let health = HealthCheck::new("my-service", "1.0.0");

        assert_eq!(health.service(), "my-service");
        assert_eq!(health.version(), "1.0.0");
    }

    #[test]
    fn test_health_check_status() {
        let health = HealthCheck::new("my-service", "1.0.0");
        let status = health.status();

        assert!(status.is_healthy());
        assert_eq!(status.service(), "my-service");
        assert_eq!(status.version(), "1.0.0");
    }

    #[test]
    fn test_health_check_uptime() {
        let health = HealthCheck::new("test", "1.0.0");
        std::thread::sleep(Duration::from_millis(10));

        let uptime = health.uptime();
        assert!(uptime >= Duration::from_millis(10));
    }

    #[test]
    fn test_readiness_check_new() {
        let readiness = ReadinessCheck::new();
        assert!(readiness.is_ready());
        assert_eq!(readiness.check_count(), 0);
    }

    #[test]
    fn test_readiness_check_add_check() {
        let readiness = ReadinessCheck::new()
            .add_check("test", || true);

        assert!(readiness.is_ready());
        assert_eq!(readiness.check_count(), 1);
    }

    #[test]
    fn test_readiness_check_failing_check() {
        let readiness = ReadinessCheck::new()
            .add_check("passing", || true)
            .add_check("failing", || false);

        assert!(!readiness.is_ready());
    }

    #[test]
    fn test_readiness_check_all_passing() {
        let readiness = ReadinessCheck::new()
            .add_check("check1", || true)
            .add_check("check2", || true)
            .add_check("check3", || true);

        assert!(readiness.is_ready());
    }

    #[test]
    fn test_readiness_check_status() {
        let readiness = ReadinessCheck::new()
            .add_check("database", || true)
            .add_check("cache", || false);

        let status = readiness.status();

        assert!(!status.is_ready());
        assert_eq!(status.check("database"), Some(true));
        assert_eq!(status.check("cache"), Some(false));
        assert_eq!(status.check("nonexistent"), None);
    }

    #[test]
    fn test_readiness_check_set_ready() {
        let readiness = ReadinessCheck::new()
            .add_check("always_pass", || true);

        assert!(readiness.is_ready());

        // Simulate graceful shutdown
        readiness.set_ready(false);
        assert!(!readiness.is_ready());

        // Re-enable
        readiness.set_ready(true);
        assert!(readiness.is_ready());
    }

    #[test]
    fn test_readiness_check_dynamic() {
        let flag = Arc::new(AtomicBool::new(true));
        let flag_clone = Arc::clone(&flag);

        let readiness = ReadinessCheck::new()
            .add_check("dynamic", move || flag_clone.load(Ordering::SeqCst));

        assert!(readiness.is_ready());

        flag.store(false, Ordering::SeqCst);
        assert!(!readiness.is_ready());

        flag.store(true, Ordering::SeqCst);
        assert!(readiness.is_ready());
    }

    #[test]
    fn test_readiness_status_serialization() {
        let mut checks = HashMap::new();
        checks.insert("db".to_string(), true);
        checks.insert("cache".to_string(), false);

        let status = ReadinessStatus::new(false, checks);
        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("\"ready\":false"));
        assert!(json.contains("\"db\":true"));
        assert!(json.contains("\"cache\":false"));
    }

    #[test]
    fn test_readiness_check_default() {
        let readiness = ReadinessCheck::default();
        assert!(readiness.is_ready());
    }

    #[test]
    fn test_health_check_clone() {
        let health1 = HealthCheck::new("test", "1.0.0");
        let health2 = health1.clone();

        assert_eq!(health1.service(), health2.service());
        assert_eq!(health1.version(), health2.version());
    }

    #[test]
    fn test_readiness_check_clone() {
        let readiness1 = ReadinessCheck::new()
            .add_check("test", || true);

        let readiness2 = readiness1.clone();

        assert!(readiness1.is_ready());
        assert!(readiness2.is_ready());

        // Setting ready on one affects both (shared Arc)
        readiness1.set_ready(false);
        assert!(!readiness2.is_ready());
    }
}
