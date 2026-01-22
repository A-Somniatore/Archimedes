# archimedes-ffi

[![crates.io](https://img.shields.io/crates/v/archimedes-ffi.svg)](https://crates.io/crates/archimedes-ffi)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

C ABI layer for the Archimedes HTTP framework. This crate enables cross-language bindings for Python, Go, TypeScript, and C++ through a stable C interface.

## Overview

`archimedes-ffi` provides a stable C ABI that other languages can call through FFI (Foreign Function Interface). This is the foundation for:

- **archimedes-py** - Python bindings via PyO3
- **archimedes-node** - Node.js/TypeScript bindings via napi-rs  
- **archimedes-go** - Go bindings via cgo
- **libarchimedes** - C++ bindings via C headers

## C API

The C API is designed to be simple and memory-safe:

```c
#include <archimedes.h>

// Create application
ArchimedesApp* app = archimedes_app_new("contract.json");

// Register handler
archimedes_app_operation(app, "listUsers", my_handler, NULL);

// Run server
archimedes_app_run(app, 8080);

// Cleanup
archimedes_app_free(app);
```

### Request Handling

```c
ArchimedesResponse* my_handler(
    const ArchimedesRequest* request,
    void* user_data
) {
    // Get request data
    const char* method = archimedes_request_method(request);
    const char* path = archimedes_request_path(request);
    const char* body = archimedes_request_body(request);

    // Get headers
    const char* auth = archimedes_request_header(request, "Authorization");

    // Create response
    return archimedes_response_json(200, "{\"status\": \"ok\"}");
}
```

### Memory Management

All Archimedes objects must be explicitly freed:

```c
// Strings returned by archimedes_* functions must be freed
char* body = archimedes_request_body(request);
// ... use body ...
archimedes_string_free(body);

// Responses are freed by the runtime after sending
// Requests are managed by the runtime
```

## Building

### From Rust

```bash
cargo build --release -p archimedes-ffi

# Output: target/release/libarchimedes.so (Linux)
#         target/release/libarchimedes.dylib (macOS)
#         target/release/archimedes.dll (Windows)
```

### Header Generation

Headers are generated via `cbindgen`:

```bash
cbindgen --config cbindgen.toml --output include/archimedes.h
```

## Type Mapping

| Rust Type | C Type | Description |
|-----------|--------|-------------|
| `String` | `char*` | Must be freed with `archimedes_string_free` |
| `&str` | `const char*` | Borrowed, do not free |
| `Vec<u8>` | `uint8_t*` + `size_t` | Buffer with length |
| `Option<T>` | `T*` (nullable) | NULL if None |
| `Result<T, E>` | `T*` + error out-param | Returns NULL on error |

## Thread Safety

The FFI layer is thread-safe:

- `ArchimedesApp` can be shared across threads
- Handlers can be called concurrently
- Request/Response objects are thread-local

## Error Handling

Errors are returned via out-parameters:

```c
ArchimedesError* error = NULL;
ArchimedesApp* app = archimedes_app_new_with_error("contract.json", &error);

if (error != NULL) {
    fprintf(stderr, "Error: %s\n", archimedes_error_message(error));
    archimedes_error_free(error);
    return 1;
}
```

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x86_64 | ✅ Supported |
| Linux | aarch64 | ✅ Supported |
| macOS | x86_64 | ✅ Supported |
| macOS | aarch64 (M1/M2) | ✅ Supported |
| Windows | x86_64 | ✅ Supported |

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Part of the Themis Platform

This crate is part of the [Archimedes](https://github.com/themis-platform/archimedes) server framework.
