FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./
RUN npm ci

# Copy source and build
COPY tsconfig.json .
COPY src/ src/
RUN npm run build

# Runtime stage
FROM node:20-alpine

WORKDIR /app

# Copy package files and install production dependencies
COPY package*.json ./
RUN npm ci --omit=dev

# Copy built files
COPY --from=builder /app/dist ./dist

# Expose port 3000 (internal port, sidecar proxies to this)
EXPOSE 3000

# Run
CMD ["node", "dist/index.js"]
