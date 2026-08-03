# Multi-stage Containerfile for Project Portfolio Management Service.
#
# Container runtime: Podman (Docker-compatible — `docker build` still
# works if a user has Docker installed instead).
# Base image:        debian:13-slim (Trixie, current Debian stable).
#
# IMPORTANT — build context must be the REPOSITORY ROOT, not this
# crate's own directory. This crate's Cargo.toml (like every crate in
# the family — there is no root Cargo.toml, so no shared vendor/lock
# step) depends on sibling crates by path: `integrity-mac`,
# `authentication-verifier`, `project-portfolio-management-matcher`.
# Build with:
#
#   podman build \
#     -f project-portfolio-management/project-portfolio-management-service-with-loco/Dockerfile \
#     -t project-portfolio-management-service .
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
COPY project-portfolio-management/project-portfolio-management-matcher-rust-crate project-portfolio-management/project-portfolio-management-matcher-rust-crate

# This crate itself: manifests first (better layer caching), then the
# nested `migration` sub-crate (loco's sea-orm-migration bridge — a
# same-directory path dependency, so it travels with the COPY below
# regardless), then sources.
COPY project-portfolio-management/project-portfolio-management-service-with-loco/Cargo.toml project-portfolio-management/project-portfolio-management-service-with-loco/Cargo.lock project-portfolio-management/project-portfolio-management-service-with-loco/
COPY project-portfolio-management/project-portfolio-management-service-with-loco/migration project-portfolio-management/project-portfolio-management-service-with-loco/migration
COPY project-portfolio-management/project-portfolio-management-service-with-loco/src project-portfolio-management/project-portfolio-management-service-with-loco/src

WORKDIR /app/project-portfolio-management/project-portfolio-management-service-with-loco

# Build the binary target declared by src/bin/main.rs. Migration SQL
# is embedded into the binary at compile time via `include_str!`
# (agents/share/architecture.md), so the runtime stage below needs no
# loose migration files. Default features only — this crate's optional
# `fluvio` (BUS-3 relay sink) feature stays off in the reference
# image; enable at build time if a deployment needs a real broker.
RUN cargo build --release --bin project-portfolio-management-service

# Stage 2: runtime — minimal Debian 13 slim base.
FROM debian:13-slim

# Runtime libraries. libpq5 for the SeaORM PostgreSQL connector,
# libssl3 for openssl, ca-certificates for HTTPS chains (PASETO
# key-set fetch), curl for the HEALTHCHECK below.
RUN apt-get update && apt-get install -y --no-install-recommends \
        libpq5 \
        libssl3 \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user. Podman maps host UID 1000 to the container UID 1000
# by default when running rootless, so bind-mounted volumes stay
# accessible without `:Z` relabel headaches.
RUN useradd --create-home --uid 1000 --shell /bin/bash portfolio

WORKDIR /app

COPY --from=builder --chown=portfolio:portfolio \
    /app/project-portfolio-management/project-portfolio-management-service-with-loco/target/release/project-portfolio-management-service \
    /app/project-portfolio-management-service

# loco reads config/<environment>.yaml at runtime (not baked into the
# binary); LOCO_ENV below selects `production.yaml`.
COPY --chown=portfolio:portfolio project-portfolio-management/project-portfolio-management-service-with-loco/config ./config

# Tantivy full-text index (PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_INDEX_PATH,
# default `data/search-index` relative to CWD — see src/search/mod.rs).
RUN mkdir -p /app/data/search-index && chown -R portfolio:portfolio /app

USER portfolio

EXPOSE 5150

# Loco's default `/_health` (this crate mounts no bespoke health route
# — see spec/index.md §9). NOTE: Podman's default OCI image format
# silently drops HEALTHCHECK ("HEALTHCHECK is not supported for OCI
# image format and will be ignored" — build with `--format docker` to
# bake it in). The examples/compose/ stacks set an equivalent
# compose-level `healthcheck:`, which works under either image format,
# so this instruction is kept for direct `docker build`/`docker run`
# users rather than relied on as the only health signal.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:5150/_health || exit 1

# Runtime defaults; override at `podman run -e …`. LOCO_ENV selects
# config/production.yaml — a container with no override boots safely
# in production mode rather than defaulting to loco's own "development".
ENV LOCO_ENV=production \
    PORT=5150

# loco's CLI entry point: `<bin> start` runs the server (see
# src/bin/main.rs — `loco_rs::cli::main`).
CMD ["/app/project-portfolio-management-service", "start"]
