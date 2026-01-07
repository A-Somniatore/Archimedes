//! Health check functionality for the sidecar.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::config::SidecarConfig;

/// Health status of the sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Sidecar is healthy.
    Healthy,
    /// Sidecar is degraded but functional.
    Degraded,
    /// Sidecar is unhealthy.
    Unhealthy,
}

impl HealthStatus {
    /// Check if the status indicates the service is operational.
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }
}

/// Readiness status of the sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadinessStatus {
    /// Sidecar is ready to handle traffic.
    Ready,
    /// Sidecar is not ready.
    NotReady,
}

impl ReadinessStatus {
    /// Check if the sidecar is ready.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status.
    pub status: HealthStatus,
    /// Individual check results.
    pub checks: Vec<CheckResult>,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
    /// Version information.
    pub version: String,
}

/// Readiness check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessResponse {
    /// Overall readiness status.
    pub status: ReadinessStatus,
    /// Individual check results.
    pub checks: Vec<CheckResult>,
}

/// Result of a single health/readiness check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Name of the check.
    pub name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Time taken for the check in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl CheckResult {
    /// Create a passing check result.
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            message: None,
            duration_ms: None,
        }
    }

    /// Create a failing check result.
    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            message: Some(message.into()),
            duration_ms: None,
        }
    }

    /// Set the duration.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self
    }

    /// Set the message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Health checker for the sidecar.
#[derive(Debug)]
pub struct HealthChecker {
    /// Start time for uptime calculation.
    start_time: Instant,
    /// Whether the sidecar is ready.
    ready: AtomicBool,
    /// Last upstream check time.
    last_upstream_check: RwLock<Option<Instant>>,
    /// Last upstream check result.
    upstream_healthy: AtomicBool,
    /// Configuration.
    config: Arc<SidecarConfig>,
    /// HTTP client for upstream checks.
    client: reqwest::Client,
}

impl HealthChecker {
    /// Create a new health checker.
    pub fn new(config: Arc<SidecarConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("failed to create HTTP client");

        Self {
            start_time: Instant::now(),
            ready: AtomicBool::new(false),
            last_upstream_check: RwLock::new(None),
            upstream_healthy: AtomicBool::new(false),
            config,
            client,
        }
    }

    /// Mark the sidecar as ready.
    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::SeqCst);
    }

    /// Check if the sidecar is ready.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    /// Get the uptime.
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Perform a liveness check.
    pub fn liveness(&self) -> HealthResponse {
        let checks = vec![
            CheckResult::pass("process").with_message("sidecar is running"),
        ];

        let all_passed = checks.iter().all(|c| c.passed);
        let status = if all_passed {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };

        HealthResponse {
            status,
            checks,
            uptime_seconds: self.uptime().as_secs(),
            version: crate::VERSION.to_string(),
        }
    }

    /// Perform a readiness check.
    pub async fn readiness(&self) -> ReadinessResponse {
        let mut checks = Vec::new();

        // Check configuration loaded
        checks.push(CheckResult::pass("config").with_message("configuration loaded"));

        // Check upstream connectivity
        let upstream_check = self.check_upstream().await;
        checks.push(upstream_check);

        // Check contract loaded (if configured)
        if self.config.contract.path.is_some() {
            checks.push(CheckResult::pass("contract").with_message("contract loaded"));
        }

        // Check policy loaded (if configured)
        if self.config.policy.bundle_path.is_some() {
            checks.push(CheckResult::pass("policy").with_message("policy loaded"));
        }

        let all_passed = checks.iter().all(|c| c.passed);
        let status = if all_passed && self.is_ready() {
            ReadinessStatus::Ready
        } else {
            ReadinessStatus::NotReady
        };

        ReadinessResponse { status, checks }
    }

    /// Check upstream service health.
    pub async fn check_upstream(&self) -> CheckResult {
        let start = Instant::now();
        let health_url = format!(
            "{}{}",
            self.config.sidecar.upstream_url, self.config.sidecar.upstream_health_path
        );

        match self.client.get(&health_url).send().await {
            Ok(resp) => {
                let duration = start.elapsed();
                *self.last_upstream_check.write() = Some(Instant::now());

                if resp.status().is_success() {
                    self.upstream_healthy.store(true, Ordering::SeqCst);
                    CheckResult::pass("upstream")
                        .with_message(format!("status {}", resp.status()))
                        .with_duration(duration)
                } else {
                    self.upstream_healthy.store(false, Ordering::SeqCst);
                    CheckResult::fail(
                        "upstream",
                        format!("unhealthy status: {}", resp.status()),
                    )
                    .with_duration(duration)
                }
            }
            Err(e) => {
                self.upstream_healthy.store(false, Ordering::SeqCst);
                CheckResult::fail("upstream", format!("connection failed: {e}"))
            }
        }
    }

    /// Check if upstream was recently healthy.
    pub fn is_upstream_healthy(&self) -> bool {
        self.upstream_healthy.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded.is_operational());
        assert!(!HealthStatus::Unhealthy.is_operational());
    }

    #[test]
    fn test_readiness_status() {
        assert!(ReadinessStatus::Ready.is_ready());
        assert!(!ReadinessStatus::NotReady.is_ready());
    }

    #[test]
    fn test_check_result() {
        let pass = CheckResult::pass("test");
        assert!(pass.passed);
        assert_eq!(pass.name, "test");

        let fail = CheckResult::fail("test", "error message");
        assert!(!fail.passed);
        assert_eq!(fail.message, Some("error message".to_string()));

        let with_duration = CheckResult::pass("test").with_duration(Duration::from_millis(100));
        assert_eq!(with_duration.duration_ms, Some(100));
    }

    #[test]
    fn test_health_checker_liveness() {
        let config = Arc::new(SidecarConfig::default());
        let checker = HealthChecker::new(config);

        let response = checker.liveness();
        assert_eq!(response.status, HealthStatus::Healthy);
        assert!(!response.checks.is_empty());
    }

    #[test]
    fn test_health_checker_ready_state() {
        let config = Arc::new(SidecarConfig::default());
        let checker = HealthChecker::new(config);

        assert!(!checker.is_ready());
        checker.set_ready(true);
        assert!(checker.is_ready());
        checker.set_ready(false);
        assert!(!checker.is_ready());
    }

    #[test]
    fn test_uptime() {
        let config = Arc::new(SidecarConfig::default());
        let checker = HealthChecker::new(config);

        std::thread::sleep(Duration::from_millis(10));
        assert!(checker.uptime() >= Duration::from_millis(10));
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            checks: vec![CheckResult::pass("test")],
            uptime_seconds: 100,
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("test"));
    }
}
