# ADR-009: Archimedes Sidecar Pattern for Multi-Language Support

**Date**: 2026-01-09
**Status**: Accepted
**Decision Makers**: Architecture Team
**Technical Story**: Enable non-Rust services to use Themis Platform middleware

## Context and Problem Statement

Archimedes is the enforcement framework for the Themis Platform, providing contract validation, authorization (OPA), and observability middleware. Currently, Archimedes is a Rust library, which means:

1. **Only Rust services** can use Archimedes directly
2. **Other languages** (Python, Go, TypeScript, C++) cannot participate in the Themis ecosystem
3. **Team adoption** is limited to teams with Rust expertise

We need to enable services written in **any language** to benefit from:
- Contract validation (request/response schema enforcement)
- Authorization (OPA policy evaluation)
- Observability (distributed tracing, metrics)
- Identity propagation (mTLS, JWT, API keys)

## Decision Drivers

- **Multi-language support** is critical for platform adoption
- **Consistent enforcement** across all services regardless of language
- **Minimal application changes** for existing services
- **Production-grade reliability** with health checks, graceful shutdown
- **Container-native deployment** in Kubernetes environments
- **Low latency overhead** for request proxying

## Considered Options

### Option 1: Language-Specific SDKs

Generate native SDKs for each language (Python, Go, TypeScript, C++).

**Pros:**
- Native performance
- Idiomatic API for each language
- No network hop

**Cons:**
- Massive engineering effort (4x the work)
- Version synchronization nightmare
- Each SDK needs full OPA/regorus port
- Different behavior in edge cases

### Option 2: Sidecar Proxy Pattern

Deploy Archimedes as a sidecar container alongside the application.

**Pros:**
- Single implementation (Rust)
- Consistent behavior across all languages
- Application changes minimal (just change upstream URL)
- Native Kubernetes deployment model
- Hot-reload contracts and policies

**Cons:**
- Network hop latency (~1-2ms)
- Additional container resource usage
- Deployment complexity

### Option 3: Service Mesh Integration

Integrate Archimedes middleware into an existing service mesh (Istio, Linkerd).

**Pros:**
- Leverages existing infrastructure
- Zero application changes
- Scales with service mesh

**Cons:**
- Service mesh is a heavy dependency
- Contract validation doesn't fit mesh model
- Limited customization
- OPA integration already exists in mesh (but not Themis contracts)

## Decision Outcome

**Chosen Option: Sidecar Proxy Pattern (Option 2)**

The sidecar pattern provides the best balance of:
- **Single codebase** to maintain
- **Consistent behavior** across all languages
- **Minimal application changes** (just proxy configuration)
- **Kubernetes-native** deployment model

The ~1-2ms latency overhead is acceptable for most services and is offset by the benefits of consistent enforcement.

## Architecture

### Request Flow

```
                    ┌─────────────────────────────────────────────┐
                    │              Kubernetes Pod                  │
                    │                                              │
  External  ────────┤►  ┌─────────────────┐    ┌────────────────┐ │
  Request           │   │ Archimedes      │    │ Application    │ │
                    │   │ Sidecar         │────│ (any language) │ │
                    │   │ (:8080)         │    │ (:8081)        │ │
                    │   └─────────────────┘    └────────────────┘ │
  External  ◄───────┤           │                      │          │
  Response          │           ▼                      │          │
                    │   ┌─────────────────┐            │          │
                    │   │ Middleware      │            │          │
                    │   │ Pipeline        │            │          │
                    │   │ • RequestId     │            │          │
                    │   │ • Identity      │            │          │
                    │   │ • Contract      │            │          │
                    │   │ • Authorization │            │          │
                    │   │ • Telemetry     │            │          │
                    │   └─────────────────┘            │          │
                    └─────────────────────────────────────────────┘
```

### Middleware Pipeline

1. **RequestId Middleware**: Generate/extract W3C Trace Context headers
2. **Identity Extraction**: Extract caller identity from mTLS, JWT, or API key
3. **Contract Validation**: Validate request against Themis contract schema
4. **Authorization**: Evaluate OPA policy for operation permission
5. **Response Validation**: Validate response against contract schema
6. **Telemetry**: Emit metrics, traces, and logs

### Configuration

The sidecar is configured via TOML file or environment variables:

```toml
[service]
name = "my-service"
upstream = "http://localhost:8081"
listen_addr = "0.0.0.0:8080"

[contract]
path = "/etc/archimedes/contract.json"
validation_mode = "enforce"  # or "monitor"

[policy]
bundle_path = "/etc/archimedes/policy.tar.gz"
default_decision = "deny"

[identity]
sources = ["mtls", "jwt", "apikey"]
jwt_issuer = "https://auth.example.com"

[telemetry]
otlp_endpoint = "http://otel-collector:4317"
service_name = "my-service"
```

### Health Endpoints

- `/_archimedes/health` - Liveness probe (sidecar running)
- `/_archimedes/ready` - Readiness probe (contracts loaded, upstream healthy)
- `/_archimedes/metrics` - Prometheus metrics endpoint

## Deployment Patterns

### Kubernetes Sidecar Injection

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-python-service
spec:
  template:
    spec:
      containers:
        - name: app
          image: my-python-service:latest
          ports:
            - containerPort: 8081
        - name: archimedes
          image: ghcr.io/themis-platform/archimedes-sidecar:latest
          ports:
            - containerPort: 8080
          volumeMounts:
            - name: contracts
              mountPath: /etc/archimedes
          env:
            - name: ARCHIMEDES_UPSTREAM
              value: "http://localhost:8081"
      volumes:
        - name: contracts
          configMap:
            name: my-service-contract
```

### Docker Compose (Development)

```yaml
version: "3.8"
services:
  app:
    build: .
    ports:
      - "8081:8081"
  
  archimedes:
    image: ghcr.io/themis-platform/archimedes-sidecar:latest
    ports:
      - "8080:8080"
    environment:
      ARCHIMEDES_UPSTREAM: http://app:8081
      ARCHIMEDES_CONTRACT_PATH: /etc/archimedes/contract.json
    volumes:
      - ./contract.json:/etc/archimedes/contract.json
```

## Consequences

### Positive

- **Unified enforcement** across all languages
- **Single codebase** to maintain and test
- **Consistent behavior** guaranteed
- **Hot-reload** contracts and policies without restart
- **Observability** automatically injected
- **Kubernetes-native** deployment model

### Negative

- **Latency overhead** (~1-2ms per request)
- **Resource overhead** (additional container per pod)
- **Debugging complexity** (two containers to inspect)
- **Network dependency** between sidecar and app

### Mitigations

- **Latency**: Optimize proxy path, use Unix sockets if needed
- **Resources**: Minimal memory footprint (~20MB), efficient Rust binary
- **Debugging**: Rich structured logging, tracing correlation
- **Network**: Health checks ensure connectivity

## Implementation

The sidecar is implemented in the `archimedes-sidecar` crate with:

- **ProxyClient**: HTTP client for forwarding to upstream
- **SidecarServer**: HTTP server handling incoming requests
- **MiddlewarePipeline**: Integration with Sentinel and Authz
- **HealthChecker**: Liveness and readiness probes
- **SidecarConfig**: TOML/JSON configuration with env overrides

## Related Decisions

- **ADR-001**: Middleware pipeline architecture
- **ADR-005**: OPA integration via regorus
- **ADR-007**: Contract validation via Sentinel

## References

- [Kubernetes Sidecar Pattern](https://kubernetes.io/docs/concepts/workloads/pods/sidecar-containers/)
- [Service Mesh Data Plane](https://www.servicemesh.io/concepts/data-plane/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
