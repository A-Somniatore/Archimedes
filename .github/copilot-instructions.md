# Copilot Instructions for Archimedes

## Project Overview

**Archimedes** is the async HTTP/gRPC/GraphQL server framework for the Themis Platform. It provides contract-first enforcement, mandatory middleware, built-in authorization via OPA, and first-class observability with OpenTelemetry.

## Technology Stack

| Area          | Technology                       |
| ------------- | -------------------------------- |
| Language      | Rust (latest stable)             |
| Async Runtime | Tokio                            |
| HTTP Server   | Hyper                            |
| gRPC          | Tonic                            |
| Serialization | Serde                            |
| Observability | OpenTelemetry                    |
| Authorization | OPA (embedded)                   |
| Testing       | Built-in + proptest + tokio-test |

## Development Guidelines

### Code Formatting

- Use `rustfmt` with default settings
- Run `cargo fmt` before every commit
- Use `cargo clippy` and fix all warnings (treat warnings as errors in CI)

### Linting Rules

```toml
# Cargo.toml or .cargo/config.toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

### Naming Conventions

- Types: `PascalCase`
- Functions/methods: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`
- Crates: `kebab-case` (e.g., `archimedes-core`)

## Testing Requirements

### CRITICAL: Test-Driven Development

**Every change MUST include tests.** This is non-negotiable.

1. **Before writing code** → Write a failing test first (TDD)
2. **New features** → Add unit tests + integration tests
3. **Bug fixes** → Add regression test that fails before fix
4. **Refactors** → Ensure existing tests still pass
5. **New files** → Every new `.rs` file needs corresponding tests

### Test Structure

```rust
// Unit tests in same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}

// Async tests
#[tokio::test]
async fn test_async_handler() {
    // Arrange
    let ctx = RequestContext::mock();

    // Act
    let result = handler.handle(&ctx, request).await;

    // Assert
    assert!(result.is_ok());
}

// Integration tests in tests/ directory
// tests/integration_test.rs
```

### Middleware Testing

For middleware tests:

```rust
#[tokio::test]
async fn test_middleware_passes_valid_request() {
    let middleware = ValidationMiddleware::new(mock_contract());
    let request = create_valid_request();

    let result = middleware.process(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_rejects_invalid_request() {
    let middleware = ValidationMiddleware::new(mock_contract());
    let request = create_invalid_request();

    let result = middleware.process(request).await;
    assert!(matches!(result, Err(ThemisError::ValidationError(_))));
}
```

### Run Tests Before Every Push

```bash
# Run ALL of these before pushing
cargo test                        # Unit + integration tests
cargo clippy -- -D warnings       # Linting (fail on warnings)
cargo fmt --check                 # Formatting check
cargo doc --no-deps               # Ensure docs build
```

## Documentation Requirements

### CRITICAL: Document After Every Change

**After writing tests and implementing a change, you MUST add documentation.** This is mandatory, not optional.

#### Development Workflow: Test → Implement → Document

1. Write failing test
2. Implement the change
3. Verify tests pass
4. **Add in-code documentation (rustdoc)**
5. **Update `docs/` for significant changes**

### In-Code Documentation (Always Required)

1. **New function** → Add rustdoc with examples
2. **New module** → Add module-level documentation
3. **New crate** → Update crate-level docs and README
4. **API changes** → Update all affected rustdoc immediately
5. **New types** → Document all public structs, enums, traits

### Documentation in `docs/` (Required for Significant Changes)

Update the `docs/` folder when changes affect:

1. **Contracts & Constraints**

   - New middleware constraints → Update `docs/spec.md`
   - Changed request/response handling → Update `docs/design.md`
   - New validation rules → Update both

2. **Breaking Changes**

   - Any breaking change → Document in `docs/` with migration guide
   - Changed middleware order → Update `docs/design.md`

3. **New Features**

   - New middleware → Update `docs/design.md` and `docs/spec.md`
   - New transport support → Update `docs/roadmap.md` and README
   - New extension points → Document in `docs/design.md`

4. **Architecture Changes**
   - New crates → Update `docs/design.md`
   - Changed request pipeline → Update architecture diagrams

### What Counts as "Significant"?

- Changes to middleware pipeline
- Changes to public API surface
- New constraints or validation rules
- Behavior changes that affect users
- New capabilities or features
- Deprecations or removals

### Rustdoc Standards

````rust
/// Brief one-line description.
///
/// Longer description if needed, explaining the purpose
/// and any important details.
///
/// # Arguments
///
/// * `ctx` - The request context containing identity and trace info
/// * `request` - The typed request object
///
/// # Returns
///
/// The handler response or a Themis error
///
/// # Errors
///
/// Returns `ThemisError` if:
/// - Request validation fails
/// - Authorization is denied
/// - Handler returns an error
///
/// # Examples
///
/// ```
/// use archimedes::prelude::*;
///
/// async fn get_user(ctx: &RequestContext, req: GetUserRequest) -> Result<User, ThemisError> {
///     // implementation
/// }
/// ```
pub async fn handle<Req, Res>(
    ctx: &RequestContext,
    request: Req,
) -> Result<Res, ThemisError> {
    // implementation
}
````

## Git Practices

### Commit Often, Push Frequently

- Make small, focused commits
- Each commit should be a logical unit of work
- Push at least at end of each work session
- Never leave work only on local machine

### Commit Message Format

```
type(scope): short description

- Detail 1
- Detail 2

Refs: #issue-number (if applicable)
```

**Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `refactor`: Code change that neither fixes bug nor adds feature
- `test`: Adding or updating tests
- `chore`: Build, CI, tooling changes

**Examples:**

```
feat(middleware): add request validation middleware

- Validate request body against contract schema
- Return 400 with structured error on validation failure
- Add validation_errors_total metric

test(middleware): add validation middleware tests

- Test valid request passthrough
- Test invalid request rejection
- Test schema edge cases
```

### Branch Strategy

- `main` – Always deployable, protected
- `feat/description` – Feature branches
- `fix/description` – Bug fix branches
- `docs/description` – Documentation branches

## Dependency Management

### Use Latest Stable Versions

- Always use the latest stable version of dependencies
- Run `cargo update` regularly
- Check for security advisories with `cargo audit`

### Minimize Dependencies

- Prefer well-maintained, audited crates
- Evaluate necessity before adding new dependencies
- Pin versions in `Cargo.lock`

## Error Handling

- Use `Result<T, E>` for fallible operations
- Use `thiserror` for defining error types
- **No `.unwrap()` in library code** (only tests/examples)
- Provide context with error chaining

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThemisError {
    #[error("Request validation failed: {reason}")]
    ValidationError { reason: String },

    #[error("Authorization denied for operation {operation_id}")]
    AuthorizationDenied { operation_id: String },

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}
```

## Security Practices

- Never commit secrets or credentials
- Use environment variables for configuration
- All external input is untrusted (validate everything)
- Follow principle of least privilege
- Log security-relevant events
- mTLS for all internal communication

## Performance Considerations

- Profile before optimizing
- Document performance-critical paths
- Add benchmarks for hot paths (`cargo bench`)
- Consider memory allocation patterns
- Minimize allocations in the request path
- Use `Arc` for shared state, avoid `Mutex` in hot paths

## Parallel Development Strategy

Archimedes can be developed **in parallel** with Themis and Eunomia:

### Phase 1: Core Framework (No Dependencies)

- Server infrastructure (hyper, tokio)
- Middleware pipeline architecture
- Request context and routing
- Error handling framework
- **Use mock contracts for testing**

### Phase 2: Integration (After Themis/Eunomia)

- Themis contract validation integration
- Eunomia OPA policy integration
- Full end-to-end testing

### Mock Contracts for Development

```rust
// Use mock contracts during parallel development
fn mock_contract() -> Contract {
    Contract {
        operations: vec![
            Operation {
                operation_id: "getUser".to_string(),
                method: Method::GET,
                path: "/users/{userId}".to_string(),
                // ... mock schema
            }
        ],
        // ...
    }
}
```

## Project Structure

```
archimedes/
├── .github/
│   └── copilot-instructions.md   # This file
├── docs/
│   ├── design.md                 # Implementation design
│   ├── spec.md                   # Specification
│   └── roadmap.md                # Development roadmap
├── README.md
└── CONTRIBUTING.md
```

## Terminal Command Preferences

When running terminal commands:

- **Never use `2>&1`** – Avoid stderr redirection in commands
- **Never use `-p` package filters** – Run `cargo test` for the whole workspace, not `cargo test -p <crate>`
- **Avoid subprocesses that don't terminate** – Some filtered commands hang indefinitely
- **Use `cargo test --workspace`** – For running all tests across the workspace
- **Single-line commit messages** – Keep commit messages concise

## Key Reminders

1. **Test everything** – If it's not tested, it's broken
2. **Document immediately** – Don't defer documentation
3. **Format and lint** – Run `cargo fmt` and `cargo clippy` always
4. **Small commits** – Atomic, focused changes
5. **Latest versions** – Keep dependencies up to date
6. **No unsafe code** – Unless absolutely necessary and documented
7. **Error handling** – No `.unwrap()` in production code
8. **Middleware is sacred** – Never skip or reorder core middleware

## CI Checklist

Before creating a PR, ensure:

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code is formatted (`cargo fmt --check`)
- [ ] Docs build (`cargo doc --no-deps`)
- [ ] No security vulnerabilities (`cargo audit`)
- [ ] Documentation updated for changes
- [ ] Commit messages follow format
