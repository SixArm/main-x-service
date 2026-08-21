# Subprojects — Course Entity

The course entity is a trio in one directory. Dependency direction is
strictly one-way: **front-end → service → matcher**.

| Subproject | Role | Stack | Docs |
|---|---|---|---|
| [course-service-with-loco](../course-service-with-loco/) | Registry service: CRUD (course + instance sub-resource), search, matching, merge, dedup, audit, privacy, REST + OpenAPI. Boots through loco.rs; **family reference** for idiomatic-loco controllers. | Rust, loco.rs 0.16 (Axum 0.8), SeaORM + PostgreSQL, Tantivy, utoipa | [spec](../course-service-with-loco/spec/index.md) · [AGENTS](../course-service-with-loco/agents/index.md) · [README](../course-service-with-loco/README.md) |
| [course-matcher-rust-crate](../course-matcher-rust-crate/) | Canonical pairwise matching library — pure, dependency-light, no IO / async / unsafe. Embedded by the service via `src/matching/adapter.rs`. | Rust library (§1–§25 spec shape) | [spec](../course-matcher-rust-crate/spec/index.md) · [AGENTS](../course-matcher-rust-crate/agents/index.md) · [README](../course-matcher-rust-crate/README.md) |
| [course-front-end-with-svelte](../course-front-end-with-svelte/) | Operator SPA over the service's REST API: list/search, create-with-409, detail, edit, audit, match, merge. | SvelteKit 2, Svelte 5 runes, SVAR DataGrid, Lily Headless, TypeScript strict | [spec](../course-front-end-with-svelte/spec/index.md) · [AGENTS](../course-front-end-with-svelte/agents/index.md) · [README](../course-front-end-with-svelte/README.md) |

## Responsibilities at a glance

- **Service** owns all durable state (PostgreSQL, Tantivy index,
  event bus), validation, privacy, audit, and the public REST surface
  under `/api` on port **8084**.
- **Matcher** owns the matching algorithm: probabilistic weights and
  deterministic short-circuits. It has no network surface and no
  state — the service is its only in-entity consumer.
- **Front-end** owns the operator experience and its own copies of
  API types / client / form primitives (drift policy 2026-06-02 — no
  shared package).

## How to run each

```bash
# Service (requires PostgreSQL; see its README for podman compose)
cd course-service-with-loco
cargo loco start            # or: cargo run -- start
cargo loco db migrate       # run migrations explicitly
cargo test --lib            # unit tests

# Matcher (pure library)
cd course-matcher-rust-crate
cargo test
cargo run                   # demo binary, illustrative only

# Front-end (requires the service at http://localhost:8084)
cd course-front-end-with-svelte
cp .env.example .env
pnpm install
pnpm dev                    # http://localhost:5173
pnpm test && pnpm test:e2e  # vitest + playwright (no live service needed)
```

## Cross-subproject pinch points

| Contract | Where it lives | What pins it |
|---|---|---|
| Service ↔ matcher DTO | [`course-service/src/matching/adapter.rs`](../course-service-with-loco/src/matching/adapter.rs) | 14 bridge tests in [`tests/duplicate_detection.rs`](../course-service-with-loco/tests/duplicate_detection.rs) |
| Service ↔ front-end wire format | [`course-front-end/src/lib/api/types.ts`](../course-front-end-with-svelte/src/lib/api/types.ts) + `client.ts` + `courses.ts` | 9 Vitest unit tests |
| Both contracts, normatively | [`../spec/05-domain-model.md`](../spec/05-domain-model.md) | Entity spec §5 |

## Entity-level docs

- [`../spec/`](../spec/index.md) — entity living spec (§1–§18);
  source of truth for the cross-subproject contract.
- [`./`](index.md) — this AGENTS set.
- `../course-service-schema.sql` — schema snapshot of unclear
  ownership; see entity spec [§16 OQ-4](../spec/16-open-questions.md).
