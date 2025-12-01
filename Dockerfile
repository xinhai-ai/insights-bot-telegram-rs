# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations
COPY locales ./locales

# Build the actual binary
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.20

# Install runtime dependencies
RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/insights-bot-telegram-rs /app/insights-bot

# Copy locale files
COPY locales /app/locales

# Copy migrations (for reference, actual migration handled by app)
COPY migrations /app/migrations

# Create required directories
RUN mkdir -p /app/data /app/logs

# Set environment defaults
ENV LOG_LEVEL=info
ENV LOCALES_DIR=/app/locales
ENV SQLITE_PATH=/app/data/insights.db

# Expose health check port
EXPOSE 3000

# Run the binary
CMD ["/app/insights-bot"]
