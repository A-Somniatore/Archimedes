# archimedes-router

High-performance radix tree router for the Archimedes HTTP framework.

[![Crates.io](https://img.shields.io/crates/v/archimedes-router)](https://crates.somniatore.com/crates/archimedes-router)
[![License](https://img.shields.io/crates/l/archimedes-router)](LICENSE)

## Features

- **Radix Tree Matching**: O(k) path lookup where k is path length, vs O(n) linear scan
- **Path Parameters**: Extract named parameters from paths (`/users/{id}`)
- **Wildcards**: Catch-all routes (`/files/*path`)
- **Method-Based Routing**: Different handlers per HTTP method
- **Zero Allocations**: Path matching with minimal heap allocations

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
archimedes-router = { version = "0.1", registry = "somniatore" }
```

## Usage

```rust
use archimedes_router::{Router, MethodRouter};
use http::Method;

let mut router = Router::new();

// Add routes
router.insert("/users", MethodRouter::new().get("listUsers").post("createUser"));
router.insert("/users/{id}", MethodRouter::new().get("getUser").delete("deleteUser"));
router.insert("/files/*path", MethodRouter::new().get("serveFile"));

// Match routes
let result = router.match_route(&Method::GET, "/users/123");
assert!(result.is_some());

let route_match = result.unwrap();
assert_eq!(route_match.operation_id, "getUser");
assert_eq!(route_match.params.get("id"), Some("123"));
```

## Architecture

The router uses a radix tree where each node represents a path segment:

```
                   (root)
                     │
             ┌───────┴───────┐
             │               │
           "users"        "files"
             │               │
       ┌─────┴─────┐        "*path"
       │           │
      (leaf)    "{id}"
  [GET,POST]      │
                (leaf)
             [GET,DELETE]
```

## Path Parameter Syntax

| Pattern | Description | Example Match |
|---------|-------------|---------------|
| `{name}` | Named parameter | `/users/{id}` matches `/users/123` |
| `*name` | Wildcard (catch-all) | `/files/*path` matches `/files/a/b/c` |

## Performance

Benchmarks show the radix tree significantly outperforms linear matching for routing tables with more than a few routes.

## License

Licensed under Apache-2.0. See [LICENSE](LICENSE) for details.

## Part of Themis Platform

This crate is part of the [Archimedes](https://github.com/ThemisPlatform/archimedes) framework in the Themis Platform ecosystem.
