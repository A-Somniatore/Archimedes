FROM rust:1.77 AS rust-builder

WORKDIR /build

# Copy Rust source
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build libarchimedes
RUN cargo build --release -p archimedes-ffi

# =============================================================================
# C++ Build Stage
# =============================================================================
FROM debian:bookworm-slim AS cpp-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy libarchimedes from Rust build
COPY --from=rust-builder /build/target/release/libarchimedes_ffi.so /usr/local/lib/
COPY --from=rust-builder /build/target/include/archimedes.h /usr/local/include/

# Copy C++ headers
COPY include/ /usr/local/include/

# Copy example source
COPY examples/cpp-native/ ./
COPY examples/contract.json ./

# Build the application
RUN mkdir build && cd build && \
    cmake -DCMAKE_BUILD_TYPE=Release \
    -DARCHIMEDES_LIBRARY=/usr/local/lib/libarchimedes_ffi.so \
    -DARCHIMEDES_INCLUDE_DIR=/usr/local/include \
    -DARCHIMEDES_C_HEADER_DIR=/usr/local/include \
    .. && \
    make

# =============================================================================
# Runtime Stage
# =============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy built binary
COPY --from=cpp-builder /app/build/cpp-native-example ./

# Copy libarchimedes
COPY --from=rust-builder /build/target/release/libarchimedes_ffi.so /usr/local/lib/
RUN ldconfig

# Copy contract
COPY examples/contract.json ./

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose port
EXPOSE 8080
EXPOSE 9090

# Environment
ENV RUST_LOG=info

# Run the application
CMD ["./cpp-native-example", "--contract", "contract.json", "--port", "8080"]
