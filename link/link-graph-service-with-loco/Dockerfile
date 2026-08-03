# Multi-stage Containerfile for Link Graph Service.
#
# Container runtime: Podman (Docker-compatible — `docker build` still
# works if a user has Docker installed instead).
# Base image:        debian:13-slim (Trixie, current Debian stable).
#
# IMPORTANT — build context must be the REPOSITORY ROOT, not this
# crate's own directory. This crate's Cargo.toml (like every crate in
# the family — there is no root Cargo.toml, so no shared vendor/lock
# step) depends on sibling crates by path: `integrity-mac`,
# `authentication-verifier`, `entity-ref`. Those live outside
# `link/link-graph-service-with-loco/`, so a build context scoped to
# this directory alone cannot resolve them. Build with:
#
#   podman build -f link/link-graph-service-with-loco/Dockerfile \
#     -t link-graph-service .
#
# run from the repository root (see examples/compose/ for the wired
# multi-service stacks that build every image this way).

FROM rust:1.96-slim AS builder

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

# Sibling path dependencies, copied in before this crate's own sources
# so they layer-cache independently of day-to-day edits here.
COPY integrity/integrity-mac-rust-crate integrity/integrity-mac-rust-crate
COPY authentication/authentication-verifier-rust-crate authentication/authentication-verifier-rust-crate
COPY link/entity-ref-rust-crate link/entity-ref-rust-crate

# This crate itself: manifests first (better layer caching), then the
# nested `migration` sub-crate (loco's sea-orm-migration bridge — a
# same-directory path dependency, so it travels with the COPY below
# regardless), then sources.
COPY link/link-graph-service-with-loco/Cargo.toml link/link-graph-service-with-loco/Cargo.lock link/link-graph-service-with-loco/
COPY link/link-graph-service-with-loco/migration link/link-graph-service-with-loco/migration
COPY link/link-graph-service-with-loco/src link/link-graph-service-with-loco/src

WORKDIR /app/link/link-graph-service-with-loco

# Build the binary target declared by src/bin/main.rs. Migration SQL
# (migration/migrations/*.sql) is embedded into the binary at compile
# time via `include_str!` (agents/share/architecture.md), so the
# runtime stage below needs no loose migration files.
RUN cargo build --release --bin link-graph-service

# Stage 2: runtime — minimal Debian 13 slim base.
FROM debian:13-slim

# Runtime libraries. libpq5 for the SeaORM PostgreSQL connector,
# libssl3 for openssl, ca-certificates for HTTPS chains (PASETO
# key-set fetch, presence probes), curl for the HEALTHCHECK below.
RUN apt-get update && apt-get install -y --no-install-recommends \
        libpq5 \
        libssl3 \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user. Podman maps host UID 1000 to the container UID 1000
# by default when running rootless, so bind-mounted volumes stay
# accessible without `:Z` relabel headaches.
RUN useradd --create-home --uid 1000 --shell /bin/bash linkgraph

WORKDIR /app

COPY --from=builder --chown=linkgraph:linkgraph \
    /app/link/link-graph-service-with-loco/target/release/link-graph-service \
    /app/link-graph-service

# loco reads config/<environment>.yaml at runtime (not baked into the
# binary); LOCO_ENV below selects `production.yaml`.
COPY --chown=linkgraph:linkgraph link/link-graph-service-with-loco/config ./config

USER linkgraph

EXPOSE 5160

# Loco's default `/_health` (this crate mounts no bespoke health route
# — see spec/09-api-surface.md). NOTE: Podman's default OCI image
# format silently drops HEALTHCHECK ("HEALTHCHECK is not supported for
# OCI image format and will be ignored" — build with `--format docker`
# to bake it in). The examples/compose/ stacks set an equivalent
# compose-level `healthcheck:`, which works under either image format,
# so this instruction is kept for direct `docker build`/`docker run`
# users rather than relied on as the only health signal.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:5160/_health || exit 1

# Runtime defaults; override at `podman run -e …`. LOCO_ENV selects
# config/production.yaml — a container with no override boots safely
# in production mode rather than defaulting to loco's own "development".
ENV LOCO_ENV=production \
    PORT=5160

# loco's CLI entry point: `<bin> start` runs the server (see
# src/bin/main.rs — `loco_rs::cli::main`).
CMD ["/app/link-graph-service", "start"]
