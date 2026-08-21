# Subprojects — the Person trio

One entity, three subprojects, one capability: a population-scale
person-identity registry with explainable matching and an operator UI.

## The trio

| Subproject | Kind | Responsibility | Spec shape |
|---|---|---|---|
| [person-service-with-loco](../person-service-with-loco/) | Rust service (loco.rs / Axum, SeaORM + PostgreSQL, Tantivy) | System of record: CRUD, search, matching, dedup, merge, audit, privacy, REST + FHIR | [spec/](../person-service-with-loco/spec/index.md) §1–§18 |
| [person-matcher-rust-crate](../person-matcher-rust-crate/) | Rust library (dependency-light, pure, no IO) | Canonical pairwise matching: deterministic + probabilistic, 42 national-identifier schemes, normalisation | [spec/](../person-matcher-rust-crate/spec/index.md) §1–§25 |
| [person-front-end-with-svelte](../person-front-end-with-svelte/) | SvelteKit 2 SPA (Svelte 5 runes, SVAR DataGrid, Lily Headless) | Operator UI over the service's REST API | [spec/](../person-front-end-with-svelte/spec/index.md) §1–§18 |

## Dependency direction (one-way, always)

```
front-end  ──HTTP /api/*──▶  service  ──Cargo path dep──▶  matcher
```

- Front-end consumes only the REST API — never the DB, index, or
  matcher.
- Service embeds the matcher and bridges with
  `src/matching/adapter.rs` (`to_matcher_person`) — see entity
  [spec §5.3](../spec/05-domain-model.md).
- Matcher knows nothing upstream: no HTTP, no DB, no service types.

## How to run each

```bash
# Service (Postgres + API at :8080; Swagger at /swagger-ui)
cd person-service-with-loco
podman compose up -d            # or: cargo run --release (needs DATABASE_URL + migrations)
cargo test --lib                # unit tests
cargo test --test duplicate_detection   # adapter↔matcher bridge tests

# Matcher (pure library — nothing to deploy)
cd person-matcher-rust-crate
cargo test
cargo run --example basic_usage

# Front-end (dev server at :5173; expects service at :8080)
cd person-front-end-with-svelte
cp .env.example .env            # PUBLIC_API_BASE_URL
pnpm install && pnpm dev
pnpm test                       # vitest (no live service needed)
bin/e2e                         # integration golden paths (needs live service)
```

## Where each subproject's docs live

| Doc | Service | Matcher | Front-end |
|---|---|---|---|
| Living spec | [spec/](../person-service-with-loco/spec/index.md) | [spec/](../person-matcher-rust-crate/spec/index.md) | [spec/](../person-front-end-with-svelte/spec/index.md) |
| Agent guide | [AGENTS.md](../person-service-with-loco/AGENTS.md) + [agents/](../person-service-with-loco/agents/index.md) | [AGENTS.md](../person-matcher-rust-crate/AGENTS.md) + [agents/](../person-matcher-rust-crate/agents/) | [AGENTS.md](../person-front-end-with-svelte/AGENTS.md) |
| User intro | [README.md](../person-service-with-loco/README.md) | [README.md](../person-matcher-rust-crate/README.md) | [README.md](../person-front-end-with-svelte/README.md) |
| Navigation / examples | [index.md](../person-service-with-loco/index.md) | [index.md](../person-matcher-rust-crate/index.md) | [index.md](../person-front-end-with-svelte/index.md) |

## Rules of engagement

- Single-subproject change → work entirely inside that subproject,
  following its own AGENTS guide.
- Seam change (adapter, wire types, shared invariants) → also edit the
  entity spec; see
  [spec-driven-development.md](spec-driven-development.md).
- The drift policy applies: the front-end keeps its own copies of API
  types / client / form primitives; do not factor out a shared
  package.
