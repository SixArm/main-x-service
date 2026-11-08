# Multi-stage Containerfile for Care Pathway Service.
#
# Container runtime: Podman (Docker-compatible — `docker build` still
# works if a user has Docker installed instead).
# Base image:        debian:13-slim (Trixie, current Debian stable).
#
# IMPORTANT — build context must be the REPOSITORY ROOT, not this
# crate's own directory. This crate's Cargo.toml (like every crate in
# the family — there is no root Cargo.toml, so no shared vendor/lock
# step) depends on sibling crates by path: `integrity-mac`,
# `authentication-verifier`, `care-pathway-matcher`. Build with:
#
#   podman build -f care-pathway/care-pathway-service-with-axum/Dockerfile \
#     -t care-pathway-service .
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
COPY care-pathway/care-pathway-matcher-rust-crate care-pathway/care-pathway-matcher-rust-crate
# `entity-ref` (the continues_as journey-link write side, landed
# 2026-08-24) is a sibling path dependency this Dockerfile never
# picked up — this crate's container was last verified 2026-08-03,
# before that dependency existed. Found building a fresh image for
# the T-28o runbook verification: "failed to read
# /app/link/entity-ref-rust-crate/Cargo.toml".
COPY link/entity-ref-rust-crate link/entity-ref-rust-crate

# This crate itself: manifests first (better layer caching), then the
# plain-SQL migrations `sqlx::migrate!` embeds, then the `.sqlx/` query
# data the `query!` macros check against (no database at build time),
# then sources.
COPY care-pathway/care-pathway-service-with-axum/Cargo.toml care-pathway/care-pathway-service-with-axum/Cargo.lock care-pathway/care-pathway-service-with-axum/
COPY care-pathway/care-pathway-service-with-axum/migrations care-pathway/care-pathway-service-with-axum/migrations
COPY care-pathway/care-pathway-service-with-axum/.sqlx care-pathway/care-pathway-service-with-axum/.sqlx
# The IEC 62304 §5.3.3 SOUP register (src/compliance/soup.rs embeds it
# via a relative include_str! from src/compliance/../../compliance/).
COPY care-pathway/care-pathway-service-with-axum/compliance care-pathway/care-pathway-service-with-axum/compliance
COPY care-pathway/care-pathway-service-with-axum/src care-pathway/care-pathway-service-with-axum/src
# Cargo parses the whole manifest even for a --bin-only build, and
# this crate's Cargo.toml declares `[[bench]] name = "service_bench"`
# — the same trap place-service's own container fix documented for
# its `bridge_bench`. Without this, cargo refuses to parse the
# manifest at all ("can't find `service_bench` bench at
# `benches/service_bench.rs`").
COPY care-pathway/care-pathway-service-with-axum/benches care-pathway/care-pathway-service-with-axum/benches

WORKDIR /app/care-pathway/care-pathway-service-with-axum

# Build the binary target declared by src/bin/main.rs. Migration SQL
# is embedded into the binary at compile time by `sqlx::migrate!`, so
# the runtime stage below needs no loose migration files.
# `SQLX_OFFLINE=true` makes the `query!` macros read `.sqlx/` instead of
# connecting to a database. Default features only — this crate's optional
# `s3` (bulk artifacts) and `fluvio` (BUS-3 relay sink) features stay
# off in the reference image; enable at build time if a deployment
# needs them.
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin care-pathway-service

# Stage 2: runtime — minimal Debian 13 slim base.
FROM debian:13-slim

# Runtime libraries. libpq5 for the PostgreSQL connector,
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
RUN useradd --create-home --uid 1000 --shell /bin/bash carepathway

WORKDIR /app

COPY --from=builder --chown=carepathway:carepathway \
    /app/care-pathway/care-pathway-service-with-axum/target/release/care-pathway-service \
    /app/care-pathway-service


# Tantivy full-text index (CARE_PATHWAY_SEARCH_INDEX_PATH, default
# `data/search-index` relative to CWD — see src/search/mod.rs).
RUN mkdir -p /app/data/search-index && chown -R carepathway:carepathway /app

USER carepathway

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

# Runtime defaults; override at `podman run -e …`. APP_ENV selects
# config/production.yaml — a container with no override boots safely
# in production mode rather than defaulting to loco's own "development".
ENV APP_ENV=production \
    PORT=5150

# loco's CLI entry point: `<bin> start` runs the server (see
# src/bin/main.rs — `axa::cli::main`).
CMD ["/app/care-pathway-service", "start", "--all"]
