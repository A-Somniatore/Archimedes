FROM ubuntu:22.04 AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source
COPY CMakeLists.txt .
COPY src/ src/

# Build
RUN mkdir build && cd build && cmake .. && make -j$(nproc)

# Runtime stage
FROM ubuntu:22.04

WORKDIR /app

# Copy binary
COPY --from=builder /app/build/example-cpp .

# Expose port 3000 (internal port, sidecar proxies to this)
EXPOSE 3000

# Run
CMD ["./example-cpp"]
