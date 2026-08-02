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
- [../../agents/share/rust-loco-stack.md](../../agents/share/rust-loco-stack.md) — **canonical** Rust / Loco dependency table + constraints
- [../../agents/share/loco.md](../../agents/share/loco.md) — Loco backend conventions

> **Single source of truth.** The dependency table and the hard
> constraints in
> [`agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md)
> are canonical. This spec summarises them, adds the front-end half of
> the stack, and records honestly where the repo currently *deviates*
> from the target — framed as known drift, not as a second standard.

---

## 1. Backend stack

The backend is **Rust 2024 edition** services built on the Loco.rs
conventions over Axum, persisted in PostgreSQL via SeaORM. They are
**backend-only** — Loco's view/template tier (Tera, HTMX, Alpine, Lily)
is deliberately *not* used; see
[`loco.md`](../../agents/share/loco.md). The canonical dependency table
is in
[`rust-loco-stack.md`](../../agents/share/rust-loco-stack.md); the
summary below maps each concern to its technology and grounds it in the
representative service manifests
(`person/person-service-with-loco/Cargo.toml`,
`organization/organization-service-with-loco/Cargo.toml`).

| Concern | Technology | Purpose |
|---|---|---|
| Language | Rust 1.93+/1.95+, 2024 edition | Systems performance + memory safety |
| Async runtime | Tokio | Async I/O and concurrency (NOT async_std) |
| Web framework | Axum 0.8 + Loco.rs 1.0 | HTTP server, routing, config/hooks/CLI (backend-only) |
| HTTP layer | hyper, tower, tower-http | CORS, compression, tracing middleware |
| Database | PostgreSQL 18+ | Persistence (NOT SQLite) |
| ORM / migrations | SeaORM 2.0 + sea-orm-migration | Async ORM; schema migrations |
| Search | Tantivy 0.22 | Embedded full-text search (target; several services still ILIKE) |
| API docs | Utoipa 5 + utoipa-swagger-ui 9 (or hand-written OpenAPI) | OpenAPI 3.0 spec + Swagger UI |
| Serialization | Serde + serde_json | JSON request/response |
| Logging | tracing + tracing-subscriber | Structured logs |
| Observability | OpenTelemetry (0.27) OTLP + tracing-opentelemetry; Prometheus 0.13 | Distributed traces, metrics, `/metrics.prom` |
| String matching | strsim, fuzzy-matcher | Jaro-Winkler, Levenshtein |
| Geo | geo, haversine | Coordinate distance (place / event matching) |
| gRPC | Tonic 0.12 + Prost 0.13 | High-throughput RPC stub |
| Timestamps | chrono 0.4 | Dates, times, durations |
| Error handling | thiserror 2 + anyhow 1 | Typed + contextual errors |
| Password hashing | argon2 0.5 | Magic-link / credential hashing (where used) |
| Authentication | rusty_paseto (PASETO v4.public) + authentication-verifier 0.1 | Server-side Postgres **cookie sessions** are the session; cross-service auth is short-lived **PASETO v4.public** tokens verified offline against the published Ed25519 key (`/.well-known/paseto-keys`). `jsonwebtoken`/RS256 + JWKS is **decommissioned**. See [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md). |
| Testing | assertables, tokio-test, mockall, tempfile (+ rstest, insta, serial_test in loco services) | Unit + integration + snapshot tests |
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
crate, embedded by the service and (for the newer loco services) reused
directly as the API DTO.

---

## 2. Hard constraints

These are non-negotiable target choices, lifted verbatim from
[`rust-loco-stack.md`](../../agents/share/rust-loco-stack.md) and
[`loco.md`](../../agents/share/loco.md). They exist to prevent
ecosystem fragmentation across the ten entity slices.

| Choose | Not | Rationale |
|---|---|---|
| Podman | Docker | OCI-compatible, rootless, daemonless |
| Tokio | async_std | One async runtime across the family |
| MiMalloc | jemalloc | Faster MUSL static builds |
| PostgreSQL | SQLite | One production database; SeaORM Postgres feature only |
| chrono | jiff | One date/time crate (sea-orm has no `with-jiff`) |
| sea-orm `with-chrono` / `with-time` | sea-orm `with-jiff` | `with-jiff` does not exist in sea-orm 2.0 |
| Postgres-backed background jobs (loco `worker` feature, `queue.kind: Postgres`) | SQLite `queue.kind` / external broker | No extra infra dependency for the job queue |

Loco background-job config (target):

```yaml
queue:
  kind: Postgres
  uri: "TODO"
  dangerously_flush: false
  num_workers: 2
```

### 2.1 Known drift toward the target

The constraints describe the *destination*. The repo is mid-migration,
so several crates currently deviate. This is recorded honestly as drift
to be closed, not as an alternative standard:

| Crate | Deviation | Target |
|---|---|---|
| Older services (`person` and peers) | SeaORM columns use `with-time` (the `time` crate) at the persistence boundary; the domain layer is `chrono` (bridged in `db/convert.rs`) | unify on SeaORM `with-chrono` |
| Several services | ILIKE / Postgres `pg_trgm` name search rather than Tantivy full-text | Tantivy 0.22 |
| Event streaming | in-memory publisher | Fluvio durable stream |

When a crate is brought to target, drop its row here and update its
`Cargo.toml` and the crate spec's §13 task queue in the same PR
(spec-first; see the SDD discipline).

---

## 3. Required crate-root attributes

Every Rust crate root (`lib.rs` / `main.rs`) carries the same quality
gate and the MUSL-gated allocator, placed immediately after the
top-level doc comment (per
[`rust-loco-stack.md`](../../agents/share/rust-loco-stack.md)):

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

## 5. Two service generations

The service crates were not all built at once, so two generations
coexist. Read a crate's `Cargo.toml` to know which one you are in.

### 5.1 Loco generation (current target)

Examples: `organization-service`, `care-pathway-service`,
`case-service`, `authentication-service`. Thin manifests that lean on
`loco-rs` 1.0 to pull the web/runtime/CLI surface.

Pulls in: `loco-rs` (Hooks / AppContext / CLI / loco config),
`axum` 0.8, `sea-orm` 2.0 + `migration` (sea-orm-migration), `serde`,
`tokio` (minimal `rt-multi-thread`), `validator`, `uuid`, `mimalloc`,
the sibling `*-matcher` crate (reused as the API DTO),
`authentication-verifier`. Dev-deps add `rstest`, `insta`,
`serial_test`, and in-process short-lived-token minting for auth tests
(target: PASETO v4.public via `rusty_paseto`; the prior RS256 path —
`jsonwebtoken`, `rsa`, `sha2`, `base64` — is being decommissioned).
Search is Postgres `ILIKE`; events are in-memory; OpenAPI is
hand-written or Utoipa.

### 5.2 Older Axum generation

Example: `person-service` (and its early peers). A fat, explicit
manifest that wires the full stack by hand and predates the loco
conversion.

Pulls in (beyond the loco set): `hyper` + `tower` + `tower-http`
explicitly, `sea-orm-migration` directly, `tantivy` 0.22, `tonic` +
`prost` + `tonic-build` (gRPC, with a `build.rs`), `utoipa` +
`utoipa-swagger-ui` + `openapiv3`, `fluvio`, the full OpenTelemetry
0.27 stack + `tracing-opentelemetry` + `prometheus`, `chrono` **and**
`time`, `strsim` + `fuzzy-matcher`, `argon2`, `bigdecimal`, `dotenvy`,
plus Criterion benches (`matching`, `search`, `validation`, `bridge`)
and a tuned `[profile.release]` (`lto = true`, `codegen-units = 1`,
`strip = true`).

The conversion direction is older Axum → Loco; the authentication
service is the reference conversion. Both generations target the same
external contract (REST + OpenAPI + the matcher DTO).

---

## 6. Versioning, edition, and MSRV

| Aspect | Value |
|---|---|
| Rust edition (target) | 2024 |
| MSRV | Rust 1.93+ (READMEs); 1.95+ stated in `rust-loco-stack.md` |
| Loco.rs | 1.0 |
| Axum | 0.8 |
| SeaORM | 2.0 |
| PostgreSQL | 18+ |
| Svelte / SvelteKit | 5 / 2 |
| Service crate versions | per-crate (`person-service` 0.5.0, `organization-service` 0.1.0, …) |
| License | dual / multi (MIT OR Apache-2.0, sometimes plus BSD-3-Clause / GPL) |

Crate versions are independent: each service, matcher, library, and
front-end carries its own SemVer in its manifest. The 2024 edition +
MSRV 1.93/1.95 is the target; the `organization-service` `edition =
"2021"` row in §2.1 is the one current edition deviation.

---

## 7. Where the pieces are documented

| Topic | Canonical doc |
|---|---|
| Full dependency table + constraints | [`../../agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md) |
| Loco conventions, background jobs | [`../../agents/share/loco.md`](../../agents/share/loco.md) |
| Architecture (module layout, data flow) | [`../architecture/index.md`](../architecture/index.md) |
| PostgreSQL + extensions | [`../postgresql/index.md`](../postgresql/index.md) |
| Availability, scaling, health checks | [`../availability/index.md`](../availability/index.md) |
| Observability (tracing / metrics / logs) | [`../observability/index.md`](../observability/index.md) |
