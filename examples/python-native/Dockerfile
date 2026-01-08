# Build stage: compile the native extension
FROM python:3.11-slim as builder

# Install Rust toolchain
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install maturin
RUN pip install maturin

# Copy the archimedes-py crate
WORKDIR /build
COPY crates/archimedes-py /build/archimedes-py
COPY crates/archimedes-ffi /build/archimedes-ffi
COPY crates/archimedes-core /build/archimedes-core
COPY Cargo.toml Cargo.lock /build/

# Build the wheel
WORKDIR /build/archimedes-py
RUN maturin build --release --strip

# Runtime stage
FROM python:3.11-slim

WORKDIR /app

# Copy the wheel and install it
COPY --from=builder /build/archimedes-py/target/wheels/*.whl /tmp/
RUN pip install /tmp/*.whl && rm /tmp/*.whl

# Copy application files
COPY examples/python-native/main.py /app/
COPY examples/python-native/archimedes.yaml /app/
COPY examples/contract.json /app/../contract.json

# Expose the service port
EXPOSE 8002

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8002/health || exit 1

# Run the application
CMD ["python", "main.py"]
