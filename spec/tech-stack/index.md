# Technology Stack — Main X Index family

Monorepo-wide technology-stack spec for the **Main X Index** family: a
federated identity index, one entity per top-level directory. This is
the umbrella view of *what we build with* — the languages, frameworks,
libraries, and the hard constraints that bind them. Per-crate dependency
detail still lives in each crate's `Cargo.toml`; per-entity behaviour is
governed by each entity's own `spec/index.md`.

See also:

- [../index.md](../index.md) — monorepo index + the entity table
- [../architecture/index.md](../architecture/index.md) — the family architecture spec
- [../postgresql/index.md](../postgresql/index.md) — PostgreSQL spec
- [../availability/index.md](../availability/index.md) — availability / scaling / health
- [../observability/index.md](../observability/index.md) — tracing / metrics / logs
- [../../agents/share/rust-axum-stack.md](../../agents/share/rust-axum-stack.md) — **canonical** Rust / axa dependency table + constraints
- [../../agents/share/axa.md](../../agents/share/axa.md) — axa service conventions

> **Single source of truth.** The dependency table and the hard
> constraints in
> [`agents/share/rust-axum-stack.md`](../../agents/share/rust-axum-stack.md)
> are canonical. This spec summarises them, adds the front-end half of
> the stack, and records honestly where the repo currently *deviates*
> from the target — framed as known drift, not as a second standard.

---

## 1. Backend stack

The backend is **Rust 2024 edition** services built directly on Axum, using
[axa](https://github.com/publicprojects-io/axa) for shared plumbing, and
persisted in PostgreSQL. They are **backend-only**. The canonical dependency
table is in
[`rust-axum-stack.md`](../../agents/share/rust-axum-stack.md); the summary below
maps each concern to its technology.

Every service reaches Postgres through sqlx, with literal, compile-checked SQL, so
that a reviewer can read the exact statements. §5 describes the rules and how the
family moved off SeaORM, which no crate uses since 2026-11-08.

| Concern | Technology | Purpose |
|---|---|---|
| Language | Rust, 2024 edition; MSRV **1.97** | Systems performance + memory safety |
| Async runtime | Tokio | Async I/O and concurrency (NOT async_std) |
| Web framework | Axum 0.8 + axa | HTTP server, routing, config/hooks/CLI (backend-only) |
| Shared service plumbing | axa | Routes, middleware, errors, configuration, CLI, jobs (library, not framework) |
| HTTP layer | hyper, tower, tower-http | CORS, compression, tracing middleware |
| Database | PostgreSQL 18+ | Persistence (NOT SQLite) |
| Database access | sqlx 0.9, `query!` literals and `sqlx::migrate!` plain SQL | Direct, reviewable SQL; schema migrations |
| Search | Tantivy 0.22 | Embedded full-text search — live on all ten entity registries |
| API docs | Utoipa 5 + utoipa-swagger-ui 9 (or hand-written OpenAPI) | OpenAPI 3.0 spec + Swagger UI |
| Serialization | Serde + serde_json | JSON request/response |
| Logging | tracing + tracing-subscriber | Structured logs |
| Observability | OpenTelemetry 0.32 OTLP + tracing-opentelemetry 0.33; Prometheus 0.13 | Distributed traces, metrics, `/metrics.prom` |
| String matching | strsim, fuzzy-matcher | Jaro-Winkler, Levenshtein |
| Geo | geo, haversine | Coordinate distance (place / event matching) |
| gRPC | Tonic 0.12 + Prost 0.13 | High-throughput RPC stub |
| Timestamps | chrono 0.4 | Dates, times, durations |
| Error handling | thiserror 2 + anyhow 1 | Typed + contextual errors |
| Password hashing | argon2 0.5 | Magic-link / credential hashing (where used) |
| Authentication | rusty_paseto (PASETO v4.public) + authentication-verifier 0.9 | Server-side Postgres **cookie sessions** are the session; cross-service auth is short-lived **PASETO v4.public** tokens verified offline against the published Ed25519 key (`/.well-known/paseto-keys`). `jsonwebtoken`/RS256 + JWKS is **decommissioned**. See [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md). |
| Testing | assertables, tokio-test, mockall, tempfile (+ rstest, insta, serial_test in controller-style services) | Unit + integration + snapshot tests |
| Benchmarking | Criterion 0.5 | Statistical performance benchmarks |
| Memory allocator | MiMalloc 0.1 | Faster allocator on MUSL static builds (NOT jemalloc) |
| Identifiers | uuid 1 (v4 + serde) | Record + request IDs |
| Validation | validator 0.19/0.20 | Declarative field validation (422 on failure) |
| Numbers | bigdecimal 0.4 | Exact decimal fields |
| Env | dotenvy | Local config from `.env` |
| Streaming | Fluvio (target) / in-memory (current) | Durable event publishing |
| Container | Podman + Debian 13 slim | OCI packaging (NOT Docker) |

The matching libraries (`strsim`, `fuzzy-matcher`, `geo`/`haversine`)
back the algorithms documented in
[`../../agents/share/match.md`](../../agents/share/match.md); the
canonical per-entity algorithm lives in each sibling `*-matcher`
crate, embedded by the service and (for the newer controller-style services) reused
directly as the API DTO.

---

## 2. Hard constraints

These are non-negotiable target choices, lifted verbatim from
[`rust-axum-stack.md`](../../agents/share/rust-axum-stack.md) and
[`axa.md`](../../agents/share/axa.md). They exist to prevent
ecosystem fragmentation across the ten entity slices.

| Choose | Not | Rationale |
|---|---|---|
| Podman | Docker | OCI-compatible, rootless, daemonless |
| Tokio | async_std | One async runtime across the family |
| MiMalloc | jemalloc | Faster MUSL static builds |
| PostgreSQL | SQLite | One production database; sqlx's `postgres` feature only |
| chrono | jiff | One date/time crate at the database boundary: sqlx's `chrono` feature, which axa turns on |
| sqlx `chrono` | sqlx `time` alongside it | With both on, sqlx 0.9.0's macros warn on every timestamp column |
| Postgres-backed background jobs (the `axa_queue` table) | SQLite / external broker | No extra infra dependency for the job queue |

### 2.1 Known drift toward the target

The constraints describe the *destination*. The repo is mid-migration,
so several crates currently deviate. This is recorded honestly as drift
to be closed, not as an alternative standard:

| Crate | Deviation | Target |
|---|---|---|
| Older services (`person`, `worker`, `event`) | some wire and audit types are still the `time` crate, filled from `chrono` rows through crate-private twins in `src/db/` | one date/time crate end to end |
| Event streaming | default transport is still the in-memory publisher (`<ENTITY>_EVENT_TRANSPORT` defaults to `memory`); the durable Postgres outbox and the `FluvioSink` relay are implemented on all ten registries but opt-in, and only case's relay points at a live deployed broker today | flip the default, and point the remaining nine relays at a broker |

Closed since this table was last drift-checked: Tantivy full-text search (was ILIKE/`pg_trgm`) is now live on all ten entity registries — see [`search/index.md`](../search/index.md).

When a crate is brought to target, drop its row here and update its
`Cargo.toml` and the crate spec's §13 task queue in the same PR
(spec-first; see the SDD discipline).

---

## 3. Required crate-root attributes

Every Rust crate root (`lib.rs` / `main.rs`) carries the same quality
gate and the MUSL-gated allocator, placed immediately after the
top-level doc comment (per
[`rust-axum-stack.md`](../../agents/share/rust-axum-stack.md)):

```rust
// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use a faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

- `#![forbid(unsafe_code)]` — no `unsafe` anywhere in the crate.
- `#![deny(missing_docs)]` — every public item is documented.
- `#![warn(clippy::pedantic)]` — pedantic lints surface in CI.
- The allocator override is `#[cfg(target_env = "musl")]`-gated so it
  applies only to static release builds, not dev builds.

`mimalloc` is therefore a normal (non-dev) dependency of every service
even though the global allocator is only swapped in on MUSL.

---

## 4. Front-end stack

Each entity gets one sibling `*-front-end-with-svelte` SvelteKit SPA
that calls its service's REST API. Grounded in
`care-pathway/care-pathway-front-end-with-svelte/package.json`.

| Concern | Technology | Purpose |
|---|---|---|
| Framework | SvelteKit 2 (`@sveltejs/kit` ^2.55) | Routing, build, adapter-auto |
| UI library | Svelte 5 (^5.53) — **runes only** | `$state`/`$derived`/`$effect`/`$props`/`$bindable`; no `export let`, no `$:` |
| Language | TypeScript 5.8 **strict** | `noUncheckedIndexedAccess` on |
| Build / dev | Vite 6 + `@sveltejs/vite-plugin-svelte` 5 | Dev server (`:5173`), build |
| Data grid | SVAR Svelte DataGrid | Tabular operator views (richer front-ends only) |
| Design system | Lily Design System (Svelte Headless) | Headless primitives — **local `file:` deps** (not published) |
| Unit tests | vitest 3 + @testing-library/svelte + jsdom | Component / unit tests |
| E2E tests | Playwright 1.49 | Browser end-to-end tests |
| Formatting | prettier + prettier-plugin-svelte | Lint / format |
| Package manager | pnpm | Workspace installs |

Constraints and conventions:

- **SPA mode.** `+layout.ts` sets `ssr = false` and
  `prerender = false`; the service returns raw JSON (no envelope) and
  a lean `src/lib/api/client.ts` wraps fetch.
- **Drift accepted.** Per the 2026-06-02 decision there is **no shared
  `mxi-svelte-core` package**: API client, types, and form primitives
  are kept per-project; scaffold a new front-end by copy-adapting a
  sibling. Lily Design System packages are referenced as local
  `file:` dependencies rather than from a registry.
- **Dependency-light variants.** The leaner front-ends (organization,
  care-pathway, case, authentication) deliberately drop the data grid
  and design system in favour of plain inputs + an `app.css` utility
  layer.

---

## 5. Service shapes and the move to sqlx

Every service is plain Axum on axa. Two internal shapes coexist, and
[`architecture.md`](../../agents/share/architecture.md) describes them:
**person-style** (`src/api/rest/`, a rich model with per-field tables) and
**controller-style** (`src/controllers/`, the matcher type stored as JSONB).

### 5.1 Why sqlx

The goal is auditability through a more popular, more secure, more direct
stack. A reviewer should be able to read every statement a service runs. SeaORM
already runs on sqlx, so removing it removes a layer and adds no new database
driver.

### 5.2 Rules for sqlx code

- All SQL lives in one area per crate, so a reviewer has one place to read.
- Use `query!` and `query_as!` by default. Commit `.sqlx/`, build with
  `SQLX_OFFLINE=true`, and check for drift against a real database in CI.
  `scripts/ci-check.sh` exports `SQLX_OFFLINE=true` for every stage, and its
  `test-db` stage runs `cargo sqlx prepare --check` once the crate's own
  `db migrate` has built the schema.
- Dynamic SQL uses bound `QueryBuilder` calls only. `AssertSqlSafe`,
  `QueryBuilder`, and `raw_sql` are refused by the `clippy` stage in any file
  [`ci/sql-allowlist.txt`](../../ci/sql-allowlist.txt) does not name with a
  reason, so that list is every place SQL is assembled from text.
- Helpers that run several statements take `&mut PgConnection`, so the same code
  runs inside a transaction. Audit and outbox rows commit in the same
  transaction as the change they describe.
- Migrations are plain SQL run by `sqlx::migrate!`. There is no deployed data to
  preserve, so a crate may squash to one reviewed baseline, proven by a
  `pg_dump` schema diff.

### 5.3 Order

One crate at a time, each proven against a real Postgres before the next, with
the crates carrying the heaviest compliance load last. Audit digests, MACs, and
the outbox must behave identically, and their tests are not edited to pass.

### 5.4 Status

This list is the one place that says which crates are on sqlx; other documents
point here rather than repeat it. Since 2026-11-08 every service is on sqlx and
no crate depends on SeaORM.

| Crate | On | Since |
|---|---|---|
| place | sqlx | `119a0497`, 2026-11-07 |
| organization | sqlx | `7614a5a3`, 2026-11-07 |
| thing | sqlx | 2026-11-07 |
| event | sqlx | 2026-11-07 |
| course | sqlx | 2026-11-07 |
| link-graph | sqlx | 2026-11-07 |
| patient-flow | sqlx | 2026-11-08 |
| authentication | sqlx | 2026-11-08 |
| case | sqlx | 2026-11-08 |
| care-pathway | sqlx | 2026-11-08 |
| person | sqlx | 2026-11-08 |
| worker | sqlx | 2026-11-08 |

Each crate on sqlx proves its schema unchanged with a `pg_dump --schema-only`
diff against a database the old code migrated, which differs only in the
bookkeeping table (`_sqlx_migrations` for `seaql_migrations`), so a database
an old build migrated must be recreated. In the two pilots each crate lost
about 34 normal dependencies (place 419 to 386, organization 382 to 348), and
line counts stayed about even, because `query_as!` spells out the column lists
that SeaORM's derives hid.

Two things every crate on sqlx carries: timestamps map through sqlx's
`chrono` feature, never `time` (with both on, sqlx 0.9.0's macros warn on
every timestamp column), and `[profile.release.build-override] strip = false`,
because macOS cannot load a stripped `sqlx-macros`.

---

## 6. Versioning, edition, and MSRV

| Aspect | Value |
|---|---|
| Rust edition (target) | 2024 |
| MSRV | **Rust 1.97** — current stable minus two, declared per crate and enforced in CI ([`../rust-msrv-n-minus-2/index.md`](../rust-msrv-n-minus-2/index.md)) |
| axa | 0.10, from crates.io (test helpers from `axa-test` 0.1) |
| Axum | 0.8 |
| sqlx | 0.9 |
| PostgreSQL | 18+ |
| Svelte / SvelteKit | 5 / 2 |
| Service crate versions | per-crate (`person-service` 0.5.0, `organization-service` 0.1.0, …) |
| License | dual / multi (MIT OR Apache-2.0, sometimes plus BSD-3-Clause / GPL) |

Crate versions are independent: each service, matcher, library, and
front-end carries its own SemVer in its manifest.

**MSRV is no longer a README claim.** It is the current stable minus
two — **1.97** today, derived from stable 1.99.1 — declared as
`rust-version` in all 46 non-`fuzz` manifests, sourced from
[`ci/msrv.txt`](../../ci/msrv.txt), and checked by
`scripts/ci-check.sh msrv`, which compiles every crate against that
toolchain. The policy, including why N-2 and how to bump it, is
[`../rust-msrv-n-minus-2/index.md`](../rust-msrv-n-minus-2/index.md). Note that the
binding constraint is the dependency graph, not our code: `axa` and `sqlx`
already require 1.94.

Every crate is on edition 2024; the `migration/` sub-crates that were still on
2021 went with SeaORM.

---

## 7. Where the pieces are documented

| Topic | Canonical doc |
|---|---|
| Full dependency table + constraints | [`../../agents/share/rust-axum-stack.md`](../../agents/share/rust-axum-stack.md) |
| axa conventions, background jobs | [`../../agents/share/axa.md`](../../agents/share/axa.md) |
| Architecture (module layout, data flow) | [`../architecture/index.md`](../architecture/index.md) |
| PostgreSQL + extensions | [`../postgresql/index.md`](../postgresql/index.md) |
| Availability, scaling, health checks | [`../availability/index.md`](../availability/index.md) |
| Observability (tracing / metrics / logs) | [`../observability/index.md`](../observability/index.md) |
