//! OpenTelemetry distributed tracing for Archimedes.
//!
//! This module provides distributed tracing with OpenTelemetry, supporting
//! OTLP export and W3C trace context propagation.
//!
//! # Features
//!
//! - Automatic span creation for requests
//! - W3C Trace Context propagation
//! - OTLP export (gRPC or HTTP)
//! - Baggage propagation
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_telemetry::tracing::{TracingConfig, init_tracing};
//!
//! let config = TracingConfig::default();
//! let provider = init_tracing(&config)?;
//! ```

use crate::error::TelemetryError;
use crate::TelemetryResult;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, TracerProvider};
use opentelemetry_sdk::Resource;

/// Tracing configuration.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Whether tracing is enabled.
    pub enabled: bool,

    /// OTLP endpoint (e.g., `http://localhost:4317`).
    pub otlp_endpoint: String,

    /// Service name for spans.
    pub service_name: String,

    /// Service version.
    pub service_version: String,

    /// Deployment environment.
    pub environment: String,

    /// Sampling ratio (0.0 to 1.0).
    pub sample_ratio: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            otlp_endpoint: "http://localhost:4317".to_string(),
            service_name: "archimedes".to_string(),
            service_version: "0.1.0".to_string(),
            environment: "development".to_string(),
            sample_ratio: 1.0, // Sample all traces by default in dev
        }
    }
}

impl TracingConfig {
    /// Creates a production configuration with lower sampling.
    #[must_use]
    pub fn production(service_name: &str, version: &str) -> Self {
        Self {
            enabled: true,
            otlp_endpoint: "http://localhost:4317".to_string(),
            service_name: service_name.to_string(),
            service_version: version.to_string(),
            environment: "production".to_string(),
            sample_ratio: 0.1, // Sample 10% in production
        }
    }
}

/// Initializes the tracing subsystem.
///
/// # Arguments
///
/// * `config` - Tracing configuration
///
/// # Returns
///
/// Returns the `TracerProvider` for later shutdown.
///
/// # Errors
///
/// Returns `TelemetryError::TracingInit` if initialization fails.
pub fn init_tracing(config: &TracingConfig) -> TelemetryResult<Option<TracerProvider>> {
    if !config.enabled {
        return Ok(None);
    }

    // Build resource with service info
    let resource = Resource::new([
        KeyValue::new(
            opentelemetry_semantic_conventions::attribute::SERVICE_NAME,
            config.service_name.clone(),
        ),
        KeyValue::new(
            opentelemetry_semantic_conventions::attribute::SERVICE_VERSION,
            config.service_version.clone(),
        ),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);

    // Build the OTLP exporter
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.otlp_endpoint)
        .build()
        .map_err(|e| TelemetryError::TracingInit(e.to_string()))?;

    // Build sampler based on ratio
    let sampler = if config.sample_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sample_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sample_ratio)
    };

    // Build tracer provider
    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_sampler(sampler)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource)
        .build();

    // Set global provider
    global::set_tracer_provider(provider.clone());

    Ok(Some(provider))
}

/// Shuts down the tracing subsystem gracefully.
pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

/// Returns a tracer for creating spans.
///
/// # Arguments
///
/// * `name` - Tracer name (typically crate/module name)
#[must_use]
pub fn tracer(name: &'static str) -> opentelemetry::global::BoxedTracer {
    global::tracer(name)
}

/// Extracts trace context from HTTP headers.
///
/// Use this to propagate trace context from incoming requests.
///
/// # Arguments
///
/// * `headers` - HTTP headers containing trace context
pub fn extract_context<T: opentelemetry::propagation::Extractor>(
    headers: &T,
) -> opentelemetry::Context {
    global::get_text_map_propagator(|propagator| propagator.extract(headers))
}

/// Injects trace context into HTTP headers.
///
/// Use this to propagate trace context to outgoing requests.
///
/// # Arguments
///
/// * `context` - Current trace context
/// * `headers` - HTTP headers to inject into
pub fn inject_context<T: opentelemetry::propagation::Injector>(
    context: &opentelemetry::Context,
    headers: &mut T,
) {
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(context, headers);
    });
}

/// HTTP header extractor for `http::HeaderMap`.
pub struct HeaderExtractor<'a>(pub &'a http::HeaderMap);

impl opentelemetry::propagation::Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(http::HeaderName::as_str).collect()
    }
}

/// HTTP header injector for `http::HeaderMap`.
pub struct HeaderInjector<'a>(pub &'a mut http::HeaderMap);

impl opentelemetry::propagation::Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(name), Ok(val)) = (
            http::header::HeaderName::try_from(key),
            http::header::HeaderValue::try_from(&value),
        ) {
            self.0.insert(name, val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::propagation::{Extractor, Injector};

    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sample_ratio, 1.0);
        assert_eq!(config.environment, "development");
    }

    #[test]
    fn test_production_config() {
        let config = TracingConfig::production("my-service", "1.0.0");
        assert_eq!(config.sample_ratio, 0.1);
        assert_eq!(config.environment, "production");
        assert_eq!(config.service_name, "my-service");
    }

    #[test]
    fn test_header_extractor() {
        let mut headers = http::HeaderMap::new();
        headers.insert("traceparent", "test-value".parse().unwrap());

        let extractor = HeaderExtractor(&headers);
        assert_eq!(extractor.get("traceparent"), Some("test-value"));
        assert!(extractor.get("nonexistent").is_none());
    }

    #[test]
    fn test_header_injector() {
        let mut headers = http::HeaderMap::new();

        {
            let mut injector = HeaderInjector(&mut headers);
            injector.set("traceparent", "injected-value".to_string());
        }

        assert_eq!(
            headers.get("traceparent").unwrap().to_str().unwrap(),
            "injected-value"
        );
    }

    #[test]
    fn test_disabled_tracing() {
        let config = TracingConfig {
            enabled: false,
            ..Default::default()
        };

        let result = init_tracing(&config);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
