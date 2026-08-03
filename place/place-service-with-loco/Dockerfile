# Multi-stage Containerfile for Place Service.
#
# Container runtime: Podman (Docker-compatible — `docker build` still
# works if a user has Docker installed instead).
# Base image:        debian:13-slim (Trixie, current Debian stable).
#
# IMPORTANT — build context must be the REPOSITORY ROOT, not this
# crate's own directory. This crate's Cargo.toml (like every crate in
# the family — there is no root Cargo.toml, so no shared vendor/lock
# step) depends on sibling crates by path: `integrity-mac`,
# `authentication-verifier` (its matcher, `place-matcher`, is pulled
# from crates.io, not a path dependency, so it needs no COPY here).
# Build with:
#
#   podman build -f place/place-service-with-loco/Dockerfile \
#     -t place-service .
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
# nested `migration` sub-crate (loco's sea-orm-migration bridge — a
# same-directory path dependency, so it travels with the COPY below
# regardless), then sources.
COPY place/place-service-with-loco/Cargo.toml place/place-service-with-loco/Cargo.lock place/place-service-with-loco/
COPY place/place-service-with-loco/migration place/place-service-with-loco/migration
# The raw numbered SQL `migration/src/*.rs` embeds via `include_str!`
# lives at the CRATE ROOT here (a sibling of `migration/`, not nested
# inside it — this varies by crate; some nest `migrations/` inside
# `migration/` instead, e.g. link-graph-service).
COPY place/place-service-with-loco/migrations place/place-service-with-loco/migrations
COPY place/place-service-with-loco/src place/place-service-with-loco/src
# `benches/` is included because Cargo.toml's `[[bench]]` declarations
# require the files to exist even when we only build the `--bin` target.
COPY place/place-service-with-loco/benches place/place-service-with-loco/benches

WORKDIR /app/place/place-service-with-loco

# Build the binary target declared by src/bin/main.rs. Migration SQL
# is embedded into the binary at compile time via `include_str!`
# (agents/share/architecture.md), so the runtime stage below needs no
# loose migration files.
RUN cargo build --release --bin place-service

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
RUN useradd --create-home --uid 1000 --shell /bin/bash place

WORKDIR /app

COPY --from=builder --chown=place:place \
    /app/place/place-service-with-loco/target/release/place-service \
    /app/place-service

# loco reads config/<environment>.yaml at runtime (not baked into the
# binary); LOCO_ENV below selects `production.yaml`.
COPY --chown=place:place place/place-service-with-loco/config ./config

# Tantivy full-text index (SEARCH_INDEX_PATH — this crate's own
# src/config/ module, not the loco config's search key; default
# `./data/search_index` relative to CWD).
RUN mkdir -p /app/data/search_index && chown -R place:place /app

USER place

EXPOSE 8080

# This crate mounts an explicit /api/health route (unlike the
# loco-native crates, which rely on /_health) — see
# src/api/rest/handlers.rs. NOTE: Podman's default OCI image format
# silently drops HEALTHCHECK ("HEALTHCHECK is not supported for OCI
# image format and will be ignored" — build with `--format docker` to
# bake it in). The examples/compose/ stacks set an equivalent
# compose-level `healthcheck:`, which works under either image format,
# so this instruction is kept for direct `docker build`/`docker run`
# users rather than relied on as the only health signal.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl --silent --fail http://localhost:8080/api/health || exit 1

# Runtime defaults; override at `podman run -e …`. LOCO_ENV selects
# config/production.yaml — a container with no override boots safely
# in production mode rather than defaulting to loco's own "development".
# SEARCH_INDEX_PATH matches the volume created above.
ENV LOCO_ENV=production \
    PORT=8080 \
    SEARCH_INDEX_PATH=/app/data/search_index

# loco's CLI entry point: `<bin> start` runs the server (see
# src/bin/main.rs — `loco_rs::cli::main`).
CMD ["/app/place-service", "start"]
