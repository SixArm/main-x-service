# Multi-stage Containerfile for Person Service.
#
# Container runtime: Podman (Docker-compatible — `docker build` still
# works if a user has Docker installed instead).
# Base image:        debian:13-slim (Trixie, current Debian stable).
#
# Stage 1: build with the full Rust toolchain.
FROM rust:1.93-slim AS builder

# Build dependencies — `libssl-dev` for openssl-sys, `libpq-dev` for
# the PostgreSQL C client headers SeaORM links against via sqlx.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        libpq-dev \
        ca-certificates \
        perl \
        make \
        gcc \
        curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency manifests + sources. `benches/` is included because
# Cargo.toml's `[[bench]]` declarations require the files to exist
# even when we only build the binary target.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
COPY migrations ./migrations

# Build the binary target declared by src/main.rs.
RUN cargo build --release --bin person-service

# Stage 2: runtime — minimal Debian 13 slim base.
FROM debian:13-slim

# Runtime libraries. libpq5 for the SeaORM PostgreSQL connector,
# libssl3 for openssl, ca-certificates for HTTPS chains, curl for the
# HEALTHCHECK below.
RUN apt-get update && apt-get install -y --no-install-recommends \
        libpq5 \
        libssl3 \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user. Podman maps host UID 1000 to the container UID 1000
# by default when running rootless, so the bind-mounted volumes stay
# accessible without `:Z` relabel headaches.
RUN useradd --create-home --uid 1000 --shell /bin/bash person

RUN mkdir -p /app/data/search_index && chown -R person:person /app

USER person
WORKDIR /app

# Copy the binary that `cargo build --bin person-service` produces.
# The output name comes from the Cargo package name (`person-service`).
COPY --from=builder --chown=person:person /app/target/release/person-service /app/person-service

# Migrations are NOT auto-applied on boot; document the manual step in
# README. Keep them in the image so an operator can shell in and run
# them against the configured `DATABASE_URL` without re-mounting.
COPY --chown=person:person migrations ./migrations

EXPOSE 8080

# REST routes mount at /api per spec.md §9 (Event service uses /api/v1).
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:8080/api/health || exit 1

# Runtime defaults; override at `podman run -e …`.
ENV RUST_LOG=info \
    SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8080 \
    SEARCH_INDEX_PATH=/app/data/search_index

CMD ["/app/person-service"]
