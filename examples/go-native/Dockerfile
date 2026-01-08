# Build stage - Rust
FROM rust:1.77 as rust-builder

WORKDIR /build

# Copy Archimedes workspace
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the FFI library
RUN cargo build --release -p archimedes-ffi

# Build stage - Go
FROM golang:1.21 as go-builder

WORKDIR /build

# Copy FFI artifacts from rust stage
COPY --from=rust-builder /build/target/release/libarchimedes_ffi.so /usr/local/lib/
COPY --from=rust-builder /build/target/include/archimedes.h /usr/local/include/

# Copy Go source
COPY examples/go-native/go.mod examples/go-native/go.sum ./
RUN go mod download

COPY examples/go-native/ ./
COPY examples/contract.json ./contract.json

# Build with CGO
ENV CGO_ENABLED=1
ENV CGO_LDFLAGS="-L/usr/local/lib"
ENV CGO_CFLAGS="-I/usr/local/include"

RUN go build -o go-native-example .

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy FFI library
COPY --from=rust-builder /build/target/release/libarchimedes_ffi.so /usr/local/lib/
RUN ldconfig

# Copy application
COPY --from=go-builder /build/go-native-example ./
COPY --from=go-builder /build/contract.json ./

# Expose port
EXPOSE 8003

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8003/health || exit 1

# Run the application
CMD ["./go-native-example"]
