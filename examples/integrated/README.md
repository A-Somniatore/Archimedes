# Integrated Example: Archimedes + Themis + Eunomia

This example demonstrates the full integration of all three core components of the Themis Platform:

- **Archimedes** - HTTP server framework with handler registry and routing
- **Themis** - Contract validation via Sentinel (loads artifacts)
- **Eunomia** - Authorization via OPA/Rego policies

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        HTTP Request                              │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Archimedes Server                          │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Handler Registry                          │ │
│  │  • listUsers → list_users()                                 │ │
│  │  • createUser → create_user()                               │ │
│  │  • getUser → get_user()                                     │ │
│  │  • updateUser → update_user()                               │ │
│  │  • deleteUser → delete_user()                               │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                    ┌───────────┴───────────┐
                    ▼                       ▼
        ┌─────────────────────┐   ┌─────────────────────┐
        │   Themis Sentinel   │   │  Eunomia Policy     │
        │   (Validation)      │   │  Evaluator (AuthZ)  │
        ├─────────────────────┤   ├─────────────────────┤
        │ • Load artifact     │   │ • Load Rego policy  │
        │ • Validate requests │   │ • Evaluate access   │
        │ • Validate responses│   │ • Role-based rules  │
        └─────────────────────┘   └─────────────────────┘
```

## Files

- `src/main.rs` - Main server code with handlers
- `contracts/users-api.openapi.yaml` - OpenAPI spec
- `contracts/users-api.artifact.json` - Compiled Themis artifact
- `policies/authz.rego` - Eunomia authorization policy

## Running the Example

```bash
cd archimedes/examples/integrated
cargo run
```

## Testing

### Health Check

```bash
curl http://localhost:8080/health
```

### Create User (as Admin - succeeds)

```bash
curl -X POST -H 'Content-Type: application/json' \
     -d '{"name":"Test User","email":"test@example.com","_user_id":"admin-1","_roles":["admin"]}' \
     http://localhost:8080/users
```

### Create User (as Regular User - denied)

```bash
curl -X POST -H 'Content-Type: application/json' \
     -d '{"name":"Test User","email":"test@example.com","_user_id":"user-1","_roles":["user"]}' \
     http://localhost:8080/users
```

## Identity Handling

**Note:** In this example, identity is passed in the request body (fields prefixed with `_`) to simulate what a decoded JWT token would provide. This is because `archimedes-server` doesn't yet have middleware support for header-based authentication.

In production, you would use:
1. Authentication middleware to extract identity from JWT/headers
2. The identity would be set on `RequestContext` before reaching handlers
3. Handlers would use `ctx.identity()` directly

The special body fields used in this example:
- `_user_id` - User identifier
- `_email` - User email (optional)
- `_roles` - Array of roles (e.g., `["admin"]`, `["user"]`)

## Authorization Policy

The Rego policy in `policies/authz.rego` implements:

- **Admins** can perform all operations
- **Users** can only read their own resources
- **Services** can read (for service-to-service calls)
- **Anonymous** is denied by default

## Regenerating the Artifact

If you modify `users-api.openapi.yaml`, regenerate the artifact:

```bash
cd contracts
themis pack users-api.openapi.yaml --output users-api.artifact.json
```

## Integration Points

### Themis (Sentinel)

```rust
// Load artifact
let artifact = ArtifactLoader::from_file(&artifact_path).await?;
let sentinel = Sentinel::new(artifact, SentinelConfig::default());

// Validate request
sentinel.validate_request("createUser", &body_json)?;
```

### Eunomia (PolicyEvaluator)

```rust
// Create evaluator
let evaluator = PolicyEvaluator::new(EvaluatorConfig {
    allow_query: "data.authz.users.allow".to_string(),
    ..Default::default()
})?;

// Load policy
evaluator.add_policy("authz.rego", &policy_content)?;

// Evaluate
let decision = evaluator.evaluate(&PolicyInput { ... })?;
```
