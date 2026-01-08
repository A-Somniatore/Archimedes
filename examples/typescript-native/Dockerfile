# Build stage
FROM rust:1.77 as rust-builder

WORKDIR /build

# Copy Archimedes workspace
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the Node addon
RUN apt-get update && apt-get install -y nodejs npm
RUN cd crates/archimedes-node && npm install && npm run build

# Node stage
FROM node:20-slim

WORKDIR /app

# Copy the built addon from rust stage
COPY --from=rust-builder /build/crates/archimedes-node/*.node ./node_modules/@archimedes/node/

# Copy application files
COPY examples/typescript-native/package*.json ./
COPY examples/typescript-native/tsconfig.json ./
COPY examples/typescript-native/src ./src
COPY examples/contract.json ./contract.json

# Install dependencies
RUN npm install

# Build TypeScript
RUN npm run build

# Expose port
EXPOSE 8004

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8004/health || exit 1

# Run the application
CMD ["npm", "start"]
