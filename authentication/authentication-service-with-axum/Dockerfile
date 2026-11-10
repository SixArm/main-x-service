# Multi-stage Containerfile for Authentication Service — the family's
# central SSO provider.
#
# Container runtime: Podman (Docker-compatible — `docker build` still
# works if a user has Docker installed instead).
# Base image:        debian:13-slim (Trixie, current Debian stable).
#
# IMPORTANT — build context must be the REPOSITORY ROOT, not this
# crate's own directory. This crate's Cargo.toml (like every crate in
# the family — there is no root Cargo.toml, so no shared vendor/lock
# step) depends on sibling crates by path: `integrity-mac`,
# `authentication-verifier`. Build with:
#
#   podman build -f authentication/authentication-service-with-axum/Dockerfile \
#     -t authentication-service .
#
# run from the repository root (see examples/compose/ for the wired
# multi-service stacks that build every image this way).

FROM rust:1.97-slim AS builder

# Build dependencies — `libssl-dev` for openssl-sys, `libpq-dev` for
# the PostgreSQL C client headers (sqlx itself is pure Rust).
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

# This crate itself: manifests first (better layer caching), then the
# plain-SQL migrations `sqlx::migrate!` embeds, then the `.sqlx/` query
# data the `query!` macros check against (no database at build time),
# then sources.
COPY authentication/authentication-service-with-axum/Cargo.toml authentication/authentication-service-with-axum/Cargo.lock authentication/authentication-service-with-axum/
COPY authentication/authentication-service-with-axum/migrations authentication/authentication-service-with-axum/migrations
COPY authentication/authentication-service-with-axum/.sqlx authentication/authentication-service-with-axum/.sqlx
COPY authentication/authentication-service-with-axum/src authentication/authentication-service-with-axum/src

WORKDIR /app/authentication/authentication-service-with-axum

# The `query!` macros read `.sqlx/` instead of a live database.
ENV SQLX_OFFLINE=true

# Build the binary target declared by src/bin/main.rs (Cargo package
# name authentication-service-cli — this crate names its binary
# differently from its package/directory name). The migration SQL is
# embedded into the binary at compile time by `sqlx::migrate!`, so the
# runtime stage below needs no loose migration files.
RUN cargo build --release --bin authentication-service-cli

# Stage 2: runtime — minimal Debian 13 slim base.
FROM debian:13-slim

# Runtime libraries. libpq5 for the PostgreSQL client,
# libssl3 for openssl, ca-certificates for HTTPS chains, curl for the
# HEALTHCHECK below.
RUN apt-get update && apt-get install -y --no-install-recommends \
        libpq5 \
        libssl3 \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user. Podman maps host UID 1000 to the container UID 1000
# by default when running rootless, so bind-mounted volumes stay
# accessible without `:Z` relabel headaches.
RUN useradd --create-home --uid 1000 --shell /bin/bash authsvc

WORKDIR /app

COPY --from=builder --chown=authsvc:authsvc \
    /app/authentication/authentication-service-with-axum/target/release/authentication-service-cli \
    /app/authentication-service-cli


RUN chown -R authsvc:authsvc /app

USER authsvc

EXPOSE 5150

# axa's liveness route `/_health/live` (this crate mounts no bespoke health route
# — see AGENTS.md's API surface table). NOTE: Podman's default OCI
# image format silently drops HEALTHCHECK ("HEALTHCHECK is not
# supported for OCI image format and will be ignored" — build with
# `--format docker` to bake it in). The examples/compose/ stacks set
# an equivalent compose-level `healthcheck:`, which works under either
# image format, so this instruction is kept for direct `docker
# build`/`docker run` users rather than relied on as the only health
# signal.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:5150/_health/live || exit 1

# Runtime defaults; override at `podman run -e …`. APP_ENV selects
# config/production.yaml — a container with no override boots safely
# in production mode rather than defaulting to loco's own "development".
# TOKEN_PRIVATE_KEY_SEED (or TOKEN_PRIVATE_KEY_FILE) is REQUIRED in
# production — the dev-only fallback seed is refused outside an
# explicitly non-production environment (SEC-A1, fail-closed). Set it
# via `podman run -e TOKEN_PRIVATE_KEY_SEED=…` or a secret-mounted file;
# never bake a real seed into an image layer.
ENV APP_ENV=production \
    PORT=5150

# loco's CLI entry point: `<bin> start` runs the server (see
# src/bin/main.rs — `axa::cli::main`).
CMD ["/app/authentication-service-cli", "start"]
