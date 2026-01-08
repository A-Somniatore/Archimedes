FROM golang:1.21-alpine AS builder

WORKDIR /app

# Copy go mod files
COPY go.mod go.sum* ./
RUN go mod download

# Copy source
COPY main.go .

# Build
RUN CGO_ENABLED=0 GOOS=linux go build -o example-go .

# Runtime stage
FROM alpine:3.19

WORKDIR /app

# Copy binary
COPY --from=builder /app/example-go .

# Expose port 3000 (internal port, sidecar proxies to this)
EXPOSE 3000

# Run
CMD ["./example-go"]
