# Subprojects — the event trio

One entity, three subprojects, strict one-way dependencies:

```
event-front-end-with-svelte  →(HTTP /api)→  event-service-with-loco  →(Cargo dep)→  event-matcher-rust-crate
```

| Subproject | Kind | Responsibility |
|---|---|---|
| [event-service-with-loco](../event-service-with-loco/) | Rust service (loco.rs/Axum, PostgreSQL + SeaORM, Tantivy) | System of record: CRUD, validation, search, matching, dedup, merge, review queue, privacy, audit, event streaming, REST `/api` + OpenAPI; FHIR + gRPC stubs |
| [event-matcher-rust-crate](../event-matcher-rust-crate/) | Rust library (pure, dependency-light) | Canonical pairwise Event comparison: deterministic rule + probabilistic weighted score with per-field breakdown; no IO, no unsafe, deterministic |
| [event-front-end-with-svelte](../event-front-end-with-svelte/) | SvelteKit 2 SPA (Svelte 5 runes, SVAR DataGrid, Lily Headless) | Operator UI: list/search, create with 409 surfacing, detail/edit/delete, audit, match check, merge |

Rules of the road:

- The matcher never depends on the service or front-end; it must
  stay runtime-free (no tokio) and IO-free.
- The service is the only writer of PostgreSQL and the only consumer
  of the matcher (via `src/matching/adapter.rs`).
- The front-end binds only to the REST API; it keeps its own copies
  of API types / client / form primitives (drift between front-ends
  is accepted by repo decision 2026-06-02 — no shared package).

## How to run each

**Service** (Rust 1.93+, PostgreSQL 18+):

```bash
cd event-service-with-loco
cp .env.example .env            # set DATABASE_URL
sea-orm-cli migrate up
cargo run --release             # http://localhost:8080/api, /swagger-ui
cargo test --lib                # unit tests
cargo test --test duplicate_detection   # service↔matcher bridge tests
```

Or containerised: `podman compose up -d`.

**Matcher** (pure library — no setup):

```bash
cd event-matcher-rust-crate
cargo test                      # unit + integration + property + doctests
cargo clippy --all-targets -- -D warnings
cargo run --example basic_usage
```

**Front-end** (Node 20+, pnpm; needs a running service):

```bash
cd event-front-end-with-svelte
cp .env.example .env            # PUBLIC_API_BASE_URL, default http://localhost:8080
pnpm install
pnpm dev                        # http://localhost:5173
pnpm test && pnpm test:e2e && pnpm check
```

## Where each subproject's docs live

| Doc | Service | Matcher | Front-end |
|---|---|---|---|
| Living spec | [spec/](../event-service-with-loco/spec/index.md) (§1–§18) | [spec/](../event-matcher-rust-crate/spec/index.md) (§1–§13; partially superseded — see entity ET-1) | [spec/](../event-front-end-with-svelte/spec/index.md) (§1–§18) |
| Agent guide | [AGENTS.md](../event-service-with-loco/AGENTS.md) + [agents/](../event-service-with-loco/agents/index.md) | [AGENTS.md](../event-matcher-rust-crate/AGENTS.md) + [agents/](../event-matcher-rust-crate/agents/architecture.md) | [AGENTS.md](../event-front-end-with-svelte/AGENTS.md) + [agents/](../event-front-end-with-svelte/agents/index.md) |
| User intro | [README](../event-service-with-loco/README.md) | [README](../event-matcher-rust-crate/README.md) | [README](../event-front-end-with-svelte/README.md) |
| Changelog | [CHANGELOG](../event-service-with-loco/CHANGELOG.md) | [CHANGELOG](../event-matcher-rust-crate/CHANGELOG.md) | [CHANGELOG](../event-front-end-with-svelte/CHANGELOG.md) |

Entity-level docs: [`../spec/`](../spec/index.md) (cross-subproject
contract) and this [`agents/`](index.md) directory. A PostgreSQL
schema snapshot sits at
[`../event-service-schema.sql`](../event-service-schema.sql)
(migrations in the service crate are authoritative).
