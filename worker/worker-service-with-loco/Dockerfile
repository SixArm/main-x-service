# Multi-stage Containerfile for Worker Service.
#
# Container runtime: Podman (Docker-compatible — `docker build` still
# works if a user has Docker installed instead).
# Base image:        debian:13-slim (Trixie, current Debian stable).
#
# IMPORTANT — build context must be the REPOSITORY ROOT, not this
# crate's own directory. This crate's Cargo.toml (like every crate in
# the family — there is no root Cargo.toml, so no shared vendor/lock
# step) depends on sibling crates by path: `integrity-mac`,
# `worker-matcher`, `authentication-verifier`, `entity-ref`. Build with:
#
#   podman build -f worker/worker-service-with-loco/Dockerfile \
#     -t worker-service .
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
COPY worker/worker-matcher-rust-crate worker/worker-matcher-rust-crate
COPY authentication/authentication-verifier-rust-crate authentication/authentication-verifier-rust-crate
COPY link/entity-ref-rust-crate link/entity-ref-rust-crate

# This crate itself: manifests first (better layer caching), then the
# nested `migration` sub-crate (loco's sea-orm-migration bridge —
# depended on as `migration = { path = "migration" }`), the sibling
# `migrations/` directory of raw SQL the migration crate embeds via
# `include_str!("../../migrations/…")`, `benches/` (Cargo.toml declares
# `[[bench]]` targets — cargo refuses to parse the manifest for ANY
# target, including `--bin`-only builds, if a declared bench file is
# missing), the IEC 62304 §5.3.3 SOUP register (`src/compliance/soup.rs`
# embeds `compliance/soup.tsv` via a relative `include_str!`), then
# sources.
COPY worker/worker-service-with-loco/Cargo.toml worker/worker-service-with-loco/Cargo.lock worker/worker-service-with-loco/
COPY worker/worker-service-with-loco/migration worker/worker-service-with-loco/migration
COPY worker/worker-service-with-loco/migrations worker/worker-service-with-loco/migrations
COPY worker/worker-service-with-loco/benches worker/worker-service-with-loco/benches
COPY worker/worker-service-with-loco/compliance worker/worker-service-with-loco/compliance
COPY worker/worker-service-with-loco/src worker/worker-service-with-loco/src

WORKDIR /app/worker/worker-service-with-loco

# Build the binary target declared by src/bin/main.rs.
RUN cargo build --release --bin worker-service

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
RUN useradd --create-home --uid 1000 --shell /bin/bash worker

WORKDIR /app

COPY --from=builder --chown=worker:worker \
    /app/worker/worker-service-with-loco/target/release/worker-service \
    /app/worker-service

# Migrations are NOT auto-applied on boot; keep the raw SQL in the
# image so an operator can shell in and run them against the
# configured `DATABASE_URL` without re-mounting.
COPY --chown=worker:worker worker/worker-service-with-loco/migrations ./migrations

# loco reads config/<environment>.yaml at runtime (not baked into the
# binary); LOCO_ENV below selects `production.yaml`.
COPY --chown=worker:worker worker/worker-service-with-loco/config ./config

RUN mkdir -p /app/data/search_index && chown -R worker:worker /app

USER worker

EXPOSE 8080

# NOTE: Podman's default OCI image format silently drops HEALTHCHECK
# ("HEALTHCHECK is not supported for OCI image format and will be
# ignored" — build with `--format docker` to bake it in). The
# examples/compose/ stacks set an equivalent compose-level
# `healthcheck:`, which works under either image format, so this
# instruction is kept for direct `docker build`/`docker run` users
# rather than relied on as the only health signal.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:8080/api/health || exit 1

# Runtime defaults; override at `podman run -e …`. LOCO_ENV selects
# config/production.yaml — a container with no override boots safely
# in production mode rather than defaulting to loco's own "development".
# NOTE: loco's own config template reads `PORT` (config/production.yaml
# `server.port: {{ get_env(name="PORT", default="8080") }}`) — the
# `SERVER_PORT` this crate's separate `src/config/mod.rs` documents is a
# different, unrelated config surface (used by non-loco code paths) and
# does NOT change what port the loco-served binary actually binds.
ENV LOCO_ENV=production \
    PORT=8080 \
    RUST_LOG=info \
    SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8080 \
    SEARCH_INDEX_PATH=/app/data/search_index

# loco's CLI entry point: `<bin> start` runs the server (see
# src/bin/main.rs — `loco_rs::cli::main`). A bare `<bin>` with no
# subcommand only prints the CLI's own --help and exits.
CMD ["/app/worker-service", "start"]
