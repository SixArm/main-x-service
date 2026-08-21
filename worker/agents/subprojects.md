# Subprojects — the Worker trio

One entity, three subprojects, one-way dependencies:
**front-end → service → matcher**.

| Subproject | Role | Stack | Spec shape |
|---|---|---|---|
| [worker-service-with-loco](../worker-service-with-loco/) | System of record: CRUD, search, dedup, merge, audit, privacy, REST/FHIR/gRPC | Rust, loco.rs 0.16 (Axum 0.8), SeaORM + PostgreSQL 18, Tantivy, utoipa | [spec §1–§18](../worker-service-with-loco/spec/index.md) |
| [worker-matcher-rust-crate](../worker-matcher-rust-crate/) | Canonical pairwise matching library (published to crates.io as `worker-matcher`) | Rust, dependency-light pure library — no IO, no `unsafe`, no async | [spec §1–§25](../worker-matcher-rust-crate/spec/index.md) |
| [worker-front-end-with-svelte](../worker-front-end-with-svelte/) | Operator UI (SPA) over the service REST API | SvelteKit 2, Svelte 5 runes, SVAR DataGrid, Lily Headless, TypeScript strict | [spec §1–§18](../worker-front-end-with-svelte/spec/index.md) |

## Responsibilities and seams

- The **service** owns all persistence and every network surface. It
  embeds the matcher (`worker-matcher = "0.6.1"` in `Cargo.toml`) and
  bridges to it via
  [`src/matching/adapter.rs`](../worker-service-with-loco/src/matching/adapter.rs)
  (`to_matcher_worker()`); the contract is specified in
  [entity spec §5.3](../spec/05-domain-model.md) and pinned by
  [`tests/duplicate_detection.rs`](../worker-service-with-loco/tests/duplicate_detection.rs).
- The **matcher** knows nothing about the service, HTTP, or the
  database. Treat its public API as a SemVer contract.
- The **front-end** talks only to the service's REST API (base URL
  `PUBLIC_API_BASE_URL`, default `http://localhost:8080`) and keeps
  its own copy of wire types (`src/lib/api/types.ts`). Drift between
  sibling front-ends is accepted; drift against the service is a bug.

## How to run each

```bash
# Service (needs PostgreSQL; see its README / DEPLOY.md)
cd worker-service-with-loco
podman compose up -d            # or: cargo run --release
cargo test --lib                # 99 unit tests
cargo test --test duplicate_detection   # 13 bridge tests

# Matcher (pure library — nothing to deploy)
cd worker-matcher-rust-crate
cargo test
cargo run --example basic_usage

# Front-end (needs a running service for live data)
cd worker-front-end-with-svelte
cp .env.example .env
pnpm install && pnpm dev        # http://localhost:5173
pnpm test && pnpm test:e2e
```

## Where each subproject's docs live

| Doc | Service | Matcher | Front-end |
|---|---|---|---|
| Living spec | [spec/](../worker-service-with-loco/spec/index.md) | [spec/](../worker-matcher-rust-crate/spec/index.md) | [spec/](../worker-front-end-with-svelte/spec/index.md) |
| Agent guide | [AGENTS.md](../worker-service-with-loco/AGENTS.md) + [agents/](../worker-service-with-loco/agents/index.md) | [AGENTS.md](../worker-matcher-rust-crate/AGENTS.md) + [agents/](../worker-matcher-rust-crate/agents/) | [AGENTS.md](../worker-front-end-with-svelte/AGENTS.md) |
| User intro | [README](../worker-service-with-loco/README.md) | [README](../worker-matcher-rust-crate/README.md) | [README](../worker-front-end-with-svelte/README.md) |
| Changelog | [CHANGELOG](../worker-service-with-loco/CHANGELOG.md) | [CHANGELOG](../worker-matcher-rust-crate/CHANGELOG.md) | [CHANGELOG](../worker-front-end-with-svelte/CHANGELOG.md) |

Also at the entity root:
[`worker-service-schema.sql`](../worker-service-schema.sql) — a
point-in-time schema snapshot (the service's migrations are
authoritative; see entity spec §10.1 and §16 OQ-3).
