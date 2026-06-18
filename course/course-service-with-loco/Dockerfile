# Multi-stage Containerfile for Course Service.
#
# Container runtime: Podman (Docker-compatible).
# Base image:        debian:13-slim (Trixie).

FROM rust:1.95-slim AS builder

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

# Both crates participate in the build because course-service depends
# on the sibling course-matcher via a path dependency.
COPY course-service-with-loco/Cargo.toml ./course-service/Cargo.toml
COPY course-service-with-loco/Cargo.lock ./course-service/Cargo.lock
COPY course-service-with-loco/src        ./course-service/src
COPY course-service-with-loco/migrations ./course-service/migrations
COPY course-matcher-rust-crate/Cargo.toml ./course-matcher/Cargo.toml
COPY course-matcher-rust-crate/Cargo.lock ./course-matcher/Cargo.lock 2>/dev/null || true
COPY course-matcher-rust-crate/src        ./course-matcher/src

WORKDIR /app/course-service
RUN cargo build --release --bin course-service

# Stage 2: runtime — minimal debian:13-slim base.
FROM debian:13-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        libpq5 \
        libssl3 \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 1000 --shell /bin/bash course
RUN mkdir -p /app/data/search_index && chown -R course:course /app

USER course
WORKDIR /app

COPY --from=builder --chown=course:course /app/course-service/target/release/course-service /app/course-service
COPY --chown=course:course course-service-with-loco/migrations ./migrations

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:8080/api/health || exit 1

ENV RUST_LOG=info \
    SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8080 \
    SEARCH_INDEX_PATH=/app/data/search_index

CMD ["/app/course-service"]
