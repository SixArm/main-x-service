# Multi-stage Containerfile for Event Service.
#
# Container runtime: Podman (Docker-compatible).
# Base image:        debian:13-slim (Trixie).
#
# Stage 1: build with the full Rust toolchain.
FROM rust:1.93-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        libpq-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build the application in release mode
RUN cargo build --release

# Stage 2: Runtime stage with minimal dependencies
FROM debian:13-slim

# Runtime libraries: libpq5 (PostgreSQL client), libssl3 (openssl),
# ca-certificates (HTTPS chains), curl (HEALTHCHECK below).
RUN apt-get update && apt-get install -y --no-install-recommends \
        libpq5 \
        libssl3 \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security (don't run as root)
RUN useradd -m -u 1000 -s /bin/bash mpi

# Create necessary directories
RUN mkdir -p /app/data/search_index && \
    chown -R mpi:mpi /app

# Switch to app user
USER mpi
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/event_service /app/event_service-server

# Copy migrations for runtime schema management
COPY --chown=mpi:mpi migrations ./migrations

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

# Set environment defaults (can be overridden)
ENV RUST_LOG=info
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV SEARCH_INDEX_PATH=/app/data/search_index

# Run the application
CMD ["/app/event_service-server"]
