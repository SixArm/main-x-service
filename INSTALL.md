# Install

How to build and run the Main X Index subprojects from source.

## Prerequisites

- **Rust** — the toolchain is pinned by
  [rust-toolchain.toml](rust-toolchain.toml) (rustup installs it
  automatically on first `cargo` invocation). Declared MSRV: see
  [ci/msrv.txt](ci/msrv.txt).
- **Podman** (not Docker) — for the per-service test databases and
  compose stacks.
- **PostgreSQL 18** — provided by the per-crate `compose.test.yaml`
  containers; no host install needed for development.
- **Node.js + pnpm** — for the SvelteKit front-ends.

## Build and run a service

Every service crate uses the same entry points:

```sh
cd person/person-service-with-loco
cargo run --release        # REST API server
cargo test                 # unit + non-DB tests
```

## Run the DB-gated test suite for a crate

```sh
scripts/test-db.sh up   person/person-service-with-loco
scripts/ci-check.sh test-db person/person-service-with-loco
scripts/test-db.sh down person/person-service-with-loco
```

Two databases at once collide on port 5432 — move one with
`TEST_DB_PORT=5433 scripts/test-db.sh up <crate>`.

## Run a front-end

```sh
cd person/person-front-end-with-svelte
pnpm install
pnpm dev
```

## Run everything

See [examples/compose/](examples/compose/) for single-service,
full-family, and ABAC-enforced Podman Compose stacks, and
[tutorials/01-getting-started.md](tutorials/01-getting-started.md) for
a verified end-to-end walkthrough.

## CI parity

Run any CI stage locally exactly as CI does:

```sh
scripts/ci-check.sh fmt|docs|clippy|test|test-db|deny|evidence|fuzz|msrv|bench [crate]
```
