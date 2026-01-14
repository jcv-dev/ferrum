# Build stage
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy source to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash ferrum

# Copy binary from builder
COPY --from=builder /app/target/release/ferrum /app/ferrum

# Create directories for data and music
RUN mkdir -p /app/data /music && \
    chown -R ferrum:ferrum /app /music

USER ferrum

# Default environment variables
ENV HOST=0.0.0.0 \
    PORT=8080 \
    MUSIC_FOLDER=/music \
    USERS_FILE=/app/data/users.json \
    LOG_LEVEL=info \
    LOG_FORMAT=json \
    RUST_BACKTRACE=1

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["./ferrum"]
