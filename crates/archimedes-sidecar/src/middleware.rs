//! Middleware integration for the sidecar.
//!
//! This module provides the middleware components that process requests
//! before they are forwarded to the upstream service.

use std::sync::Arc;

use bytes::Bytes;
use http::{Method, StatusCode};
use serde_json::Value;
use tracing::{debug, warn};

use crate::config::{SidecarConfig, ValidationMode};
use crate::error::{SidecarError, SidecarResult};
use crate::headers::PropagatedHeaders;
use crate::proxy::ProxyRequest;

#[cfg(feature = "sentinel")]
use archimedes_sentinel::{ArtifactLoader, Sentinel, SentinelConfig};

#[cfg(feature = "authz")]
use archimedes_authz::{EvaluatorConfig, PolicyEvaluator};

#[cfg(feature = "authz")]
use themis_platform_types::PolicyInput;

/// Middleware pipeline for the sidecar.
///
/// This processes requests through validation and authorization
/// before forwarding to the upstream service.
#[derive(Clone)]
pub struct MiddlewarePipeline {
    /// Configuration.
    config: Arc<SidecarConfig>,
    /// Contract validator (optional).
    #[cfg(feature = "sentinel")]
    sentinel: Option<Arc<Sentinel>>,
    /// Policy evaluator (optional).
    #[cfg(feature = "authz")]
    evaluator: Option<Arc<parking_lot::RwLock<PolicyEvaluator>>>,
}

impl MiddlewarePipeline {
    /// Create a new middleware pipeline.
    #[allow(unused_variables)]
    pub async fn new(config: Arc<SidecarConfig>) -> SidecarResult<Self> {
        #[cfg(feature = "sentinel")]
        let sentinel = if let Some(ref path) = config.contract.path {
            debug!("Loading contract from {:?}", path);
            let artifact = ArtifactLoader::from_file(path)
                .await
                .map_err(|e| SidecarError::config(format!("failed to load contract: {e}")))?;
            Some(Arc::new(Sentinel::new(artifact, SentinelConfig::default())))
        } else {
            None
        };

        #[cfg(feature = "authz")]
        let evaluator = if let Some(ref path) = config.policy.bundle_path {
            debug!("Loading policy bundle from {:?}", path);
            let mut evaluator = PolicyEvaluator::new(EvaluatorConfig::default())
                .map_err(|e| SidecarError::config(format!("failed to create evaluator: {e}")))?;
            evaluator
                .load_bundle_from_file(path)
                .await
                .map_err(|e| SidecarError::config(format!("failed to load policy: {e}")))?;
            Some(Arc::new(parking_lot::RwLock::new(evaluator)))
        } else {
            None
        };

        Ok(Self {
            config,
            #[cfg(feature = "sentinel")]
            sentinel,
            #[cfg(feature = "authz")]
            evaluator,
        })
    }

    /// Process a request through the middleware pipeline.
    ///
    /// Returns the processed request with any modifications, or an error
    /// if validation or authorization failed.
    #[allow(clippy::unused_async)]
    pub async fn process(&self, request: &ProxyRequest, body: &Bytes) -> SidecarResult<MiddlewareResult> {
        let mut result = MiddlewareResult::default();

        // Try to match operation from contract
        #[cfg(feature = "sentinel")]
        if let Some(ref sentinel) = self.sentinel {
            result.operation_id = self.match_operation(sentinel, &request.method, &request.path);

            if let Some(ref operation_id) = result.operation_id {
                // Validate request against contract
                if let Err(e) = self.validate_request(sentinel, operation_id, body) {
                    match self.config.contract.mode {
                        ValidationMode::Enforce => return Err(e),
                        ValidationMode::Monitor => {
                            warn!(
                                operation_id = %operation_id,
                                error = %e,
                                "Request validation failed (monitor mode)"
                            );
                        }
                    }
                }
            }
        }

        // Evaluate authorization policy
        #[cfg(feature = "authz")]
        if let Some(ref evaluator) = self.evaluator {
            if self.config.policy.enabled {
                let input = self.build_policy_input(request, result.operation_id.as_deref());
                self.evaluate_policy(evaluator, &input)?;
            }
        }

        Ok(result)
    }

    /// Match the request to a contract operation.
    #[cfg(feature = "sentinel")]
    fn match_operation(&self, sentinel: &Sentinel, method: &Method, path: &str) -> Option<String> {
        sentinel
            .resolve(method.as_str(), path)
            .ok()
            .map(|r| r.operation_id)
    }

    /// Validate request body against the contract schema.
    #[cfg(feature = "sentinel")]
    fn validate_request(
        &self,
        sentinel: &Sentinel,
        operation_id: &str,
        body: &Bytes,
    ) -> SidecarResult<()> {
        if body.is_empty() {
            return Ok(());
        }

        let body_json: Value = serde_json::from_slice(body)
            .map_err(|e| SidecarError::validation(format!("invalid JSON body: {e}")))?;

        let result = sentinel
            .validate_request(operation_id, &body_json)
            .map_err(|e| SidecarError::validation(e.to_string()))?;

        if !result.valid {
            let errors: Vec<String> = result
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.path, e.message))
                .collect();
            return Err(SidecarError::validation(errors.join("; ")));
        }

        Ok(())
    }

    /// Build policy input for authorization evaluation.
    #[cfg(feature = "authz")]
    fn build_policy_input(&self, request: &ProxyRequest, operation_id: Option<&str>) -> PolicyInput {
        use themis_platform_types::{CallerIdentity, RequestId};
        
        let request_id = RequestId::new();
        
        PolicyInput::builder()
            .caller(CallerIdentity::Anonymous)
            .service("sidecar")
            .operation_id(operation_id.map(String::from).unwrap_or_else(|| "unknown".to_string()))
            .method(request.method.as_str())
            .path(&request.path)
            .request_id(request_id)
            .build()
    }

    /// Evaluate authorization policy.
    #[cfg(feature = "authz")]
    #[allow(clippy::significant_drop_tightening)]
    fn evaluate_policy(
        &self,
        evaluator: &Arc<parking_lot::RwLock<PolicyEvaluator>>,
        input: &PolicyInput,
    ) -> SidecarResult<()> {
        let decision = evaluator.read()
            .evaluate(input)
            .map_err(|e| SidecarError::internal(format!("policy evaluation error: {e}")))?;

        if !decision.allowed {
            let reason = decision.reason.unwrap_or_else(|| "access denied".to_string());
            return Err(SidecarError::authorization_denied(reason));
        }

        Ok(())
    }

    /// Validate response against contract schema (optional).
    #[cfg(feature = "sentinel")]
    pub fn validate_response(
        &self,
        operation_id: &str,
        status: StatusCode,
        body: &Bytes,
    ) -> SidecarResult<()> {
        let Some(ref sentinel) = self.sentinel else {
            return Ok(());
        };

        if body.is_empty() {
            return Ok(());
        }

        let body_json: Value = serde_json::from_slice(body)
            .map_err(|e| SidecarError::validation(format!("invalid response JSON: {e}")))?;

        let result = sentinel
            .validate_response(operation_id, status.as_u16(), &body_json)
            .map_err(|e| {
                warn!(
                    operation_id = %operation_id,
                    status = %status,
                    error = %e,
                    "Response validation failed"
                );
                SidecarError::validation(e.to_string())
            })?;

        if !result.valid {
            warn!(
                operation_id = %operation_id,
                status = %status,
                errors = ?result.errors,
                "Response validation failed"
            );
            // Response validation failures are logged but not enforced
            // to avoid breaking clients
        }

        Ok(())
    }
}

/// Result of middleware processing.
#[derive(Debug, Default)]
pub struct MiddlewareResult {
    /// Matched operation ID from contract.
    pub operation_id: Option<String>,
    /// Additional headers to propagate.
    pub headers: PropagatedHeaders,
}

impl MiddlewareResult {
    /// Get the operation ID if matched.
    pub fn operation_id(&self) -> Option<&str> {
        self.operation_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_middleware_pipeline_creation() {
        let config = Arc::new(SidecarConfig::default());
        let pipeline = MiddlewarePipeline::new(config).await;
        assert!(pipeline.is_ok());
    }

    #[tokio::test]
    async fn test_middleware_result_default() {
        let result = MiddlewareResult::default();
        assert!(result.operation_id.is_none());
    }

    #[tokio::test]
    async fn test_process_request_no_validators() {
        let config = Arc::new(SidecarConfig::default());
        let pipeline = MiddlewarePipeline::new(config).await.unwrap();
        
        let request = ProxyRequest::new(Method::GET, "/test");
        let body = Bytes::new();
        
        let result = pipeline.process(&request, &body).await;
        assert!(result.is_ok());
    }
}
