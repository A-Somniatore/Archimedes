# =============================================================================
# Archimedes Sidecar Dockerfile
# =============================================================================
# Multi-stage build for minimal production image
#
# Build: docker build -t archimedes-sidecar -f crates/archimedes-sidecar/Dockerfile .
# Run:   docker run -p 8080:8080 -e ARCHIMEDES_UPSTREAM=http://host.docker.internal:8081 archimedes-sidecar
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build
# -----------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

# Install required build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project for caching dependencies
WORKDIR /build

# Copy workspace Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy all crate manifests (but not source code yet)
COPY crates/archimedes/Cargo.toml crates/archimedes/
COPY crates/archimedes-authz/Cargo.toml crates/archimedes-authz/
COPY crates/archimedes-config/Cargo.toml crates/archimedes-config/
COPY crates/archimedes-core/Cargo.toml crates/archimedes-core/
COPY crates/archimedes-docs/Cargo.toml crates/archimedes-docs/
COPY crates/archimedes-extract/Cargo.toml crates/archimedes-extract/
COPY crates/archimedes-macros/Cargo.toml crates/archimedes-macros/
COPY crates/archimedes-middleware/Cargo.toml crates/archimedes-middleware/
COPY crates/archimedes-router/Cargo.toml crates/archimedes-router/
COPY crates/archimedes-sentinel/Cargo.toml crates/archimedes-sentinel/
COPY crates/archimedes-server/Cargo.toml crates/archimedes-server/
COPY crates/archimedes-sidecar/Cargo.toml crates/archimedes-sidecar/
COPY crates/archimedes-sse/Cargo.toml crates/archimedes-sse/
COPY crates/archimedes-tasks/Cargo.toml crates/archimedes-tasks/
COPY crates/archimedes-telemetry/Cargo.toml crates/archimedes-telemetry/
COPY crates/archimedes-ws/Cargo.toml crates/archimedes-ws/

# Create dummy source files to enable dependency caching
RUN mkdir -p crates/archimedes/src && echo "pub fn dummy() {}" > crates/archimedes/src/lib.rs
RUN mkdir -p crates/archimedes-authz/src && echo "pub fn dummy() {}" > crates/archimedes-authz/src/lib.rs
RUN mkdir -p crates/archimedes-config/src && echo "pub fn dummy() {}" > crates/archimedes-config/src/lib.rs
RUN mkdir -p crates/archimedes-core/src && echo "pub fn dummy() {}" > crates/archimedes-core/src/lib.rs
RUN mkdir -p crates/archimedes-docs/src && echo "pub fn dummy() {}" > crates/archimedes-docs/src/lib.rs
RUN mkdir -p crates/archimedes-extract/src && echo "pub fn dummy() {}" > crates/archimedes-extract/src/lib.rs
RUN mkdir -p crates/archimedes-macros/src && echo "pub fn dummy() {}" > crates/archimedes-macros/src/lib.rs
RUN mkdir -p crates/archimedes-middleware/src && echo "pub fn dummy() {}" > crates/archimedes-middleware/src/lib.rs
RUN mkdir -p crates/archimedes-router/src && echo "pub fn dummy() {}" > crates/archimedes-router/src/lib.rs
RUN mkdir -p crates/archimedes-sentinel/src && echo "pub fn dummy() {}" > crates/archimedes-sentinel/src/lib.rs
RUN mkdir -p crates/archimedes-server/src && echo "pub fn dummy() {}" > crates/archimedes-server/src/lib.rs
RUN mkdir -p crates/archimedes-sidecar/src && echo "fn main() {}" > crates/archimedes-sidecar/src/main.rs
RUN mkdir -p crates/archimedes-sse/src && echo "pub fn dummy() {}" > crates/archimedes-sse/src/lib.rs
RUN mkdir -p crates/archimedes-tasks/src && echo "pub fn dummy() {}" > crates/archimedes-tasks/src/lib.rs
RUN mkdir -p crates/archimedes-telemetry/src && echo "pub fn dummy() {}" > crates/archimedes-telemetry/src/lib.rs
RUN mkdir -p crates/archimedes-ws/src && echo "pub fn dummy() {}" > crates/archimedes-ws/src/lib.rs

# Build dependencies only (will be cached)
RUN cargo build --release --package archimedes-sidecar || true

# Now copy the actual source code
COPY crates/ crates/

# Touch source files to invalidate cache for actual build
RUN touch crates/archimedes-sidecar/src/main.rs

# Build the actual binary
RUN cargo build --release --package archimedes-sidecar

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd --gid 1000 archimedes \
    && useradd --uid 1000 --gid archimedes --shell /bin/bash --create-home archimedes

# Create directories for configuration
RUN mkdir -p /etc/archimedes /var/run/archimedes \
    && chown -R archimedes:archimedes /etc/archimedes /var/run/archimedes

# Copy the binary from builder
COPY --from=builder /build/target/release/archimedes-sidecar /usr/local/bin/

# Set ownership
RUN chown archimedes:archimedes /usr/local/bin/archimedes-sidecar

# Switch to non-root user
USER archimedes

# Set working directory
WORKDIR /var/run/archimedes

# Expose the sidecar port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/archimedes-sidecar", "--health-check"] || exit 1

# Default environment variables
ENV ARCHIMEDES_LISTEN_ADDR=0.0.0.0:8080 \
    ARCHIMEDES_UPSTREAM=http://localhost:8081 \
    ARCHIMEDES_SERVICE_NAME=archimedes-sidecar \
    RUST_LOG=info

# Run the sidecar
ENTRYPOINT ["/usr/local/bin/archimedes-sidecar"]
CMD ["--config", "/etc/archimedes/config.toml"]
