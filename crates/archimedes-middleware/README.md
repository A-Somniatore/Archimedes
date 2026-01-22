# archimedes-middleware

[![crates.io](https://img.shields.io/crates/v/archimedes-middleware.svg)](https://crates.io/crates/archimedes-middleware)
[![docs.rs](https://docs.rs/archimedes-middleware/badge.svg)](https://docs.rs/archimedes-middleware)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Middleware pipeline for the Archimedes HTTP framework. Provides the fixed-order middleware execution that ensures consistent security, validation, and observability.

## Middleware Pipeline

Archimedes enforces a fixed middleware order that cannot be changed:

```
┌────────────────────────────────────────────────────────────┐
│                     Incoming Request                        │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  1. Request ID         Generate/propagate X-Request-Id     │
├────────────────────────────────────────────────────────────┤
│  2. Tracing            OpenTelemetry span creation         │
├────────────────────────────────────────────────────────────┤
│  3. Identity           Extract caller identity             │
├────────────────────────────────────────────────────────────┤
│  4. Authorization      OPA policy evaluation               │
├────────────────────────────────────────────────────────────┤
│  5. Request Validation Contract schema validation          │
├────────────────────────────────────────────────────────────┤
│  6. HANDLER            Your application code               │
├────────────────────────────────────────────────────────────┤
│  7. Response Validation Validate response (optional)       │
├────────────────────────────────────────────────────────────┤
│  8. Telemetry          Emit metrics and logs               │
├────────────────────────────────────────────────────────────┤
│  9. Error Normalization Standard error format              │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│                     Outgoing Response                       │
└────────────────────────────────────────────────────────────┘
```

## Optional Middleware

In addition to the fixed pipeline, you can add optional middleware:

### CORS

```rust
use archimedes_middleware::CorsMiddleware;

let cors = CorsMiddleware::new()
    .allow_origins(["https://example.com"])
    .allow_methods(["GET", "POST", "PUT", "DELETE"])
    .allow_headers(["Content-Type", "Authorization"])
    .allow_credentials(true)
    .max_age(Duration::from_secs(3600));

app.middleware(cors);
```

### Rate Limiting

```rust
use archimedes_middleware::RateLimitMiddleware;

let rate_limit = RateLimitMiddleware::new()
    .requests_per_second(100)
    .burst_size(200)
    .key_extractor(|req| req.client_ip().to_string());

app.middleware(rate_limit);
```

### Compression

```rust
use archimedes_middleware::CompressionMiddleware;

let compression = CompressionMiddleware::new()
    .gzip(true)
    .brotli(true)
    .min_size(1024); // Only compress responses > 1KB

app.middleware(compression);
```

### Static Files

```rust
use archimedes_middleware::StaticFilesMiddleware;

let static_files = StaticFilesMiddleware::new("./static")
    .prefix("/assets")
    .cache_control("max-age=86400");

app.middleware(static_files);
```

## Feature Flags

- `sentinel` - Enable Themis contract validation
- `opa` - Enable OPA/Eunomia authorization
- `compression` - Enable response compression
- `static-files` - Enable static file serving

## Creating Custom Middleware

```rust
use archimedes_middleware::{Middleware, Next};
use archimedes_core::{Request, Response};
use async_trait::async_trait;

struct TimingMiddleware;

#[async_trait]
impl Middleware for TimingMiddleware {
    async fn call(&self, request: Request, next: Next<'_>) -> Response {
        let start = std::time::Instant::now();

        // Call the next middleware/handler
        let response = next.run(request).await;

        // Log timing
        let duration = start.elapsed();
        tracing::info!(duration_ms = duration.as_millis(), "Request processed");

        response
    }
}
```

## Middleware Order (Custom)

Custom middleware is added around the fixed pipeline:

```rust
app.middleware(CustomBefore); // Runs before fixed pipeline
app.route("/api", handler);   // Fixed pipeline executes here
app.middleware(CustomAfter);  // Runs after fixed pipeline
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Part of the Themis Platform

This crate is part of the [Archimedes](https://github.com/themis-platform/archimedes) server framework.
