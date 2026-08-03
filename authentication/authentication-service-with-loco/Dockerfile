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
#   podman build -f authentication/authentication-service-with-loco/Dockerfile \
#     -t authentication-service .
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

# This crate itself: manifests first (better layer caching), then the
# in-crate `migration` module (unlike the entity services, this
# crate's migrator lives at src/migration/ per its own AGENTS.md, not
# a separate sibling `migration` package — it travels with `src` below
# regardless), then sources.
COPY authentication/authentication-service-with-loco/Cargo.toml authentication/authentication-service-with-loco/Cargo.lock authentication/authentication-service-with-loco/
COPY authentication/authentication-service-with-loco/src authentication/authentication-service-with-loco/src

WORKDIR /app/authentication/authentication-service-with-loco

# Build the binary target declared by src/bin/main.rs (Cargo package
# name authentication-service-cli — this crate names its binary
# differently from its package/directory name). Migration SQL is
# embedded into the binary at compile time via `include_str!`
# (agents/share/architecture.md), so the runtime stage below needs no
# loose migration files.
RUN cargo build --release --bin authentication-service-cli

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
# by default when running rootless, so bind-mounted volumes stay
# accessible without `:Z` relabel headaches.
RUN useradd --create-home --uid 1000 --shell /bin/bash authsvc

WORKDIR /app

COPY --from=builder --chown=authsvc:authsvc \
    /app/authentication/authentication-service-with-loco/target/release/authentication-service-cli \
    /app/authentication-service-cli

# loco reads config/<environment>.yaml at runtime (not baked into the
# binary); LOCO_ENV below selects `production.yaml`. `config/keys/`
# holds only a README in the repo — no signing key is ever baked into
# the image; TOKEN_PRIVATE_KEY_SEED / TOKEN_PRIVATE_KEY_FILE are
# supplied at `podman run` time (agents/share/security.md SEC-A1: the
# service fails closed in production without one — see ENV comment
# below).
COPY --chown=authsvc:authsvc authentication/authentication-service-with-loco/config ./config

RUN chown -R authsvc:authsvc /app

USER authsvc

EXPOSE 5150

# Loco's default `/_health` (this crate mounts no bespoke health route
# — see AGENTS.md's API surface table). NOTE: Podman's default OCI
# image format silently drops HEALTHCHECK ("HEALTHCHECK is not
# supported for OCI image format and will be ignored" — build with
# `--format docker` to bake it in). The examples/compose/ stacks set
# an equivalent compose-level `healthcheck:`, which works under either
# image format, so this instruction is kept for direct `docker
# build`/`docker run` users rather than relied on as the only health
# signal.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:5150/_health || exit 1

# Runtime defaults; override at `podman run -e …`. LOCO_ENV selects
# config/production.yaml — a container with no override boots safely
# in production mode rather than defaulting to loco's own "development".
# TOKEN_PRIVATE_KEY_SEED (or TOKEN_PRIVATE_KEY_FILE) is REQUIRED in
# production — the dev-only fallback seed is refused outside an
# explicitly non-production environment (SEC-A1, fail-closed). Set it
# via `podman run -e TOKEN_PRIVATE_KEY_SEED=…` or a secret-mounted file;
# never bake a real seed into an image layer.
ENV LOCO_ENV=production \
    PORT=5150

# loco's CLI entry point: `<bin> start` runs the server (see
# src/bin/main.rs — `loco_rs::cli::main`).
CMD ["/app/authentication-service-cli", "start"]
