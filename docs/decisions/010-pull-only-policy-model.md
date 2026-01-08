# ADR-010: Pull-Only Policy and Contract Loading Model

**Date**: 2026-01-09
**Status**: Accepted
**Decision Makers**: Architecture Team
**Technical Story**: Resolve spec gap for control plane endpoint (§8.3)

## Context and Problem Statement

The Archimedes specification (§8.3) originally required a **private control-plane endpoint** for Eunomia to push policy bundles:

> "Archimedes exposes a private control-plane endpoint. Eunomia calls this endpoint to push updates. Security: mTLS, SPIFFE allowlist."

This creates complexity:

1. Archimedes must expose an inbound endpoint
2. Must implement mTLS certificate validation
3. Must maintain a SPIFFE allowlist
4. Must handle atomic updates with rollback
5. Creates bidirectional dependency between Archimedes and Eunomia

## Decision Drivers

- **Simplicity** – Minimize deployment complexity
- **Security** – Reduce attack surface
- **Kubernetes-native** – Work with standard K8s patterns
- **Decoupling** – Avoid tight integration between components
- **Time-to-market** – Ship V1.0 faster

## Considered Options

### Option 1: Push Endpoint (Original Spec)

Implement the control-plane endpoint as specified.

**Pros:**

- Matches original spec
- Enables real-time policy updates
- Eunomia has direct control

**Cons:**

- Security complexity (mTLS, SPIFFE allowlist)
- Additional attack surface
- Tight coupling with Eunomia
- More code to maintain

### Option 2: Pull-Only with File Watching

Archimedes loads policies from filesystem and watches for changes.

**Pros:**

- Simple deployment model
- No inbound endpoints needed
- Works with K8s ConfigMaps/Secrets
- Loose coupling (any tool can update files)
- Standard operational patterns

**Cons:**

- Slight delay in policy propagation
- Requires shared storage or volume mounts

### Option 3: Hybrid (Pull + Optional Push)

Support both models with push as an optional feature.

**Pros:**

- Maximum flexibility
- Migration path to push

**Cons:**

- Code complexity
- Two code paths to maintain

## Decision Outcome

**Chosen Option: Pull-Only with File Watching (Option 2)**

For V1.0, Archimedes will use a pull-only model:

1. **Contract Loading**: File-based via `ArtifactLoader`
2. **Policy Loading**: File-based via `BundleLoader`
3. **Hot Reload**: File system watching (notify crate)
4. **Deployment**: K8s ConfigMap/Secret mounting

## Implementation

### File Watching

```rust
use notify::{Watcher, RecursiveMode};

impl PolicyWatcher {
    pub fn watch(&self, path: &Path) -> Result<(), WatchError> {
        let mut watcher = notify::recommended_watcher(|res| {
            match res {
                Ok(event) => {
                    // Reload policy bundle
                    self.reload_policy();
                }
                Err(e) => tracing::error!("Watch error: {:?}", e),
            }
        })?;

        watcher.watch(path, RecursiveMode::NonRecursive)?;
        Ok(())
    }
}
```

### Kubernetes Deployment

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: archimedes-policy
data:
  policy.tar.gz: |
    # Base64-encoded policy bundle
---
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
        - name: archimedes
          volumeMounts:
            - name: policy
              mountPath: /etc/archimedes/policies
      volumes:
        - name: policy
          configMap:
            name: archimedes-policy
```

### Update Flow

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────────┐
│   Eunomia    │     │   K8s API /     │     │   Archimedes     │
│   Compiler   │────►│   ConfigMap     │────►│   File Watcher   │
└──────────────┘     └─────────────────┘     └──────────────────┘
                            │
                            │ Update ConfigMap
                            ▼
                     ┌─────────────────┐
                     │   kubelet       │
                     │   sync to pod   │
                     └─────────────────┘
```

## Consequences

### Positive

- **Simpler security model** – No inbound endpoint, no SPIFFE allowlist
- **Standard K8s patterns** – ConfigMap updates are well-understood
- **Loose coupling** – Archimedes doesn't depend on Eunomia directly
- **Easier testing** – Just update a file to test policy changes
- **CI/CD friendly** – GitOps workflows work naturally

### Negative

- **Propagation delay** – ConfigMap sync can take 1-2 minutes
- **No push notification** – Archimedes polls/watches, doesn't receive push
- **Spec deviation** – Doesn't match §8.3 exactly

### Mitigations

- **Propagation delay**: Use `subPath` mounts for faster sync, or direct Secret mounts
- **Push notification**: Can add optional webhook endpoint in V1.1 if needed
- **Spec deviation**: Update spec.md to document the decision

## Future Considerations

For V1.1, we may add an optional push endpoint if:

- Real-time policy updates are required (<1s propagation)
- Eunomia needs acknowledgment of successful policy load
- Audit requirements mandate push-based updates

The pull-only model does NOT preclude adding push later.

## Related Decisions

- **ADR-006**: gRPC deferred to post-MVP (push endpoint would use gRPC)
- **ADR-009**: Sidecar also uses pull-only model

## Spec Update Required

Update `spec.md` §8.3 to reflect this decision:

```markdown
### 8.3 Policy Loading (Updated)

**V1.0 Model**: Pull-only with file watching

- Policies loaded from filesystem at startup
- File watcher detects changes and reloads
- Works with K8s ConfigMap/Secret mounting
- See ADR-010 for rationale

**V1.1+ (Optional)**: Push endpoint

- Private gRPC endpoint for Eunomia
- Protected via mTLS + SPIFFE allowlist
- Atomic updates with rollback support
```
