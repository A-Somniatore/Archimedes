FROM rust:1.75-slim AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml .

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy real source and build
COPY src/ src/
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/example-rust-native .

# Expose port 8001 (native, no sidecar)
EXPOSE 8001

# Run
CMD ["./example-rust-native"]
