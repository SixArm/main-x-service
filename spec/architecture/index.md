# Architecture — Main X Index family

Monorepo-wide architecture spec for the **Main X Index** family of web
services: a federated identity index, one entity per top-level
directory. This is the umbrella view; per-entity behaviour is governed
by each entity's own `spec/index.md` (the single source of truth for
that entity).

See also:

- [../index.md](../index.md) — monorepo index + the entity table
- [../data-modeling.md](../data-modeling.md) — SQL-first data-modeling rules
- [../data.md](../data.md) — data conventions
- [../postgresql/index.md](../postgresql/index.md) — PostgreSQL spec
- [../../agents/share/architecture.md](../../agents/share/architecture.md) — the per-service architecture brief this expands on
- [../../agents/share/index.md](../../agents/share/index.md) — shared reference-doc index

> **Note on links.** Several per-topic references below point at
> `agents/share/*.md` rather than `spec/<topic>/index.md`. As of this
> writing the only monorepo-level topic spec dirs that exist are
> `spec/postgresql/` and this `spec/architecture/`; the canonical
> match / merge / search / dataflow / auditability / tech-stack
> references live under `agents/share/`. When a `spec/<topic>/index.md`
> is later promoted, repoint the link.

---

## 1. The family shape — one entity per top-level directory

The monorepo is organised by **domain entity**, not by layer. Each
entity owns a top-level directory containing the full vertical slice for
that entity: front-end, library crate, service crate, and umbrella docs.

Entities (10): `person`, `worker`, `place`, `thing`, `event`, `course`,
`organization`, `care-pathway`, `case`, `authentication`. Plus one
**consumer application** that is not itself an indexed entity:
`case-folder` (NHS paper case-note folder location tracking; it consumes
the person / place / worker services).

Each entity directory holds these parts:

| Part | Shape | Example |
| ---- | ----- | ------- |
| Front-end | SvelteKit 2 SPA (Svelte 5 runes, SVAR DataGrid, Lily) calling the sibling service's REST API | `person/person-front-end-with-svelte/` |
| Library crate | A **matcher** (dependency-light pairwise comparison lib) — except `authentication`, whose library is the **verifier** | `person/person-matcher-rust-crate/`, `authentication/authentication-verifier-rust-crate/` |
| Service crate | The HTTP API service over PostgreSQL | `person/person-service-rust-crate/` |
| Entity `spec/` | Entity-level umbrella spec (§1–§18 SDD shape; §13 live task queue) | `person/spec/index.md` |
| Entity `AGENTS/` | Entity-level agent guide index | `person/AGENTS/index.md` |

The full entity → (front-end, library, service, umbrella) mapping is the
table in [../index.md](../index.md). The matcher library is both usable
standalone **and** embedded in the sibling service's matching layer, so
the scoring algorithm has one canonical implementation per entity.

`authentication` is the odd one out by design: it is the central
single-sign-on provider (passwordless magic-link, RS256 JWT issuance,
JWKS for offline verification). Its library crate is the
`authentication-verifier` that the other services embed to verify
bearer tokens offline. See [../../authentication/spec/index.md](../../authentication/spec/index.md).

---

## 2. Two service architectures (and why both exist)

There are **two** service-crate architectures in the repo. This is a
deliberate, in-progress convergence — not an accident.

### 2a. loco.rs services (the reference style)

Entities: **authentication, organization, care-pathway, case.**

These are real [loco.rs](https://loco.rs/) 0.16 services (Axum 0.8
under the hood). Characteristics:

- An `App` type implements loco's `Hooks` (`boot`, `routes`,
  `after_routes`, `connect_workers`, `truncate`, `seed`). It carries no
  state; the framework supplies `AppContext` (DB handle, config) per
  request. See `organization/organization-service-rust-crate/src/app.rs`.
- Endpoints are **loco controllers** (`src/controllers/*.rs`) registered
  via `AppRoutes::with_default_routes().add_route(...)`. loco supplies
  default `/_health` and `/_ping`.
- Migrations are `sea-orm-migration` migrations under `migration/`.
- Persistence is **DTO-as-JSONB**: the matcher's domain type *is* the
  API DTO, stored verbatim in a `data` JSONB column alongside denormalised
  handles (`pid`, `name`, `active`, soft-delete). The service matches with
  the exact same type it stores, so there is no separate model or adapter
  to drift. (e.g. `organizations` table: `pid`, `name`, `data` JSONB,
  `active`.) See `organization/organization-service-rust-crate/AGENTS.md`.
- JWT verification is wired here: `src/auth.rs` (`AuthUser` /
  `MaybeAuthUser` extractors, `/whoami`) using the embedded
  `authentication-verifier`. Blanket `/api/*` enforcement is an
  `after_routes` middleware layer gated by an env flag (off by default).

### 2b. Older Axum "MPI-style" services

Entities: **person, worker, place, thing, event, course.**

These predate the loco conversion and follow a hand-rolled,
master-patient-index-style layout with explicit layered modules. From
`person/person-service-rust-crate/src/lib.rs`:

```
src/
├── api/        REST (Axum) + FHIR R5 + gRPC (stub) + ApiResponse/ApiError envelopes
├── db/         SeaORM entities, repositories, audit log
├── matching/   probabilistic + deterministic matchers, algorithms, scoring, phonetic
├── search/     Tantivy index + query builder
├── streaming/  event producer/consumer (in-memory)
├── validation/ data-quality validation, normalization, standardization
├── privacy/    masking, GDPR export, consent checking
├── models/     domain models (leaves)
├── config/  observability/  error.rs  lib.rs
```

Differences from the loco style:

- A hand-built Axum router + `AppState` (`src/api/rest/state.rs`) holding
  `Arc`-wrapped shared services, rather than loco `Hooks`/`AppContext`.
- A **normalized schema**: every repeating collection is its own child
  table with a FK and ordering column (names, identifiers, addresses,
  contacts, links, …) — not DTO-as-JSONB. JSONB is reserved for opaque
  snapshots only (audit old/new values, merge `transferred_data`,
  review-queue `score_breakdown`). See [../data-modeling.md](../data-modeling.md).
- A broader surface: FHIR R5 endpoints, Tantivy search, privacy/GDPR
  endpoints, batch deduplication — features the loco services have
  deferred in favour of a thinner first cut.

### Why both — and the convergence plan

The older services were built first as full-featured MPI services. The
loco services were built later as the **reference target**: loco.rs is
the chosen framework going forward (`authentication-service` was the
first real loco crate; `organization`, `care-pathway`, `case`
followed). The intent is for the older six to converge onto the loco
shape over time. Until then, both styles coexist; treat the loco
services as the canonical pattern when scaffolding or refactoring. (A
prior bulk loco-conversion mounted the older services under loco but
left their idiomatic-controller rewrite deferred.)

| Aspect | loco style | MPI style |
| ------ | ---------- | --------- |
| Entities | authentication, organization, care-pathway, case | person, worker, place, thing, event, course |
| Framework seam | loco `Hooks` + `AppContext` | hand-built Axum `Router` + `AppState` |
| Endpoints | loco controllers | `api/rest` handlers (+ `api/fhir`, `api/grpc`) |
| Migrations | `sea-orm-migration` | SeaORM migrations |
| Persistence | matcher DTO as JSONB (+ denormalised handles) | normalized child tables |
| Matching | embedded matcher crate, same type stored & matched | embedded matcher + service-side adapter |
| Search | Postgres `ILIKE` (Tantivy deferred) | Tantivy full-text |
| Role | reference target | converging toward loco |

---

## 3. Layering rules

Both styles obey the same dependency direction. The API layer is the
entry point and may depend on everything below it; the engines below
must not depend back up.

```
api  ──►  db · matching · search · streaming · validation · privacy
                │            │
                └──► models (leaves) ◄── (everything)
```

Rules:

- **`api` is the only inbound layer.** Handlers/controllers orchestrate
  the engines; nothing depends on `api`.
- **`matching` and `search` must not depend on `api` or `db`.** They
  operate on domain types (or the matcher's own types), so the matcher
  crate stays usable standalone and the algorithm has one home.
- **`models` are leaves.** Domain models depend on nothing in the crate;
  every layer may depend on them.
- **Engines are trait-abstracted** so implementations swap: `*Repository`
  (SeaORM), `*Matcher` (probabilistic/deterministic), `EventProducer`
  (in-memory now, Kafka/NATS/Fluvio later), `EventConsumer` (stub).

Shared-services wiring differs by style but plays the same role:

- **MPI style:** `AppState` (`src/api/rest/state.rs`) holds
  `db`, `*_repository`, `event_publisher`, `audit_log`, `search_engine`,
  `matcher`, `config` — all `Arc`-shared, injected into handlers.
- **loco style:** loco's `AppContext` carries the DB + config; the
  matcher is invoked directly inside controllers; auth verifier and event
  publisher are process-wide `OnceLock`s rather than state.

---

## 4. Cross-service integration

Services are **independent deployables**. There is no shared database and
no shared in-process state across entities.

- **One database per service.** Each service owns its schema and
  migrations; services never read each other's tables.
- **Integration is over HTTP.** A service that needs another entity
  calls its REST API (e.g. the `case-folder` consumer app calls the
  person / place / worker services).
- **Trust is via RS256 JWT.** `authentication-service` is the single
  issuer; every other service verifies bearer tokens **offline** using
  the embedded `authentication-verifier` (fetches/holds the issuer's
  JWKS; checks `kid` / `iss` / `aud` / `exp`). No shared secret, no
  introspection round-trip. See
  [../../authentication/spec/index.md](../../authentication/spec/index.md)
  and [../../agents/share/jwt-enforcement.md](../../agents/share/jwt-enforcement.md).
- **Stateless services** scale horizontally; state lives in PostgreSQL.
  See [../../agents/share/availability.md](../../agents/share/availability.md).

This keeps each entity independently deployable, releasable, and
scalable, at the cost of cross-service joins (done over the API, not in
SQL).

---

## 5. Request lifecycle / data flows

The three core write/read flows are identical in intent across both
styles (the loco services implement thinner first cuts — e.g. `ILIKE`
candidate selection instead of Tantivy, no privacy step yet). Canonical
flow reference: [../../agents/share/dataflow.md](../../agents/share/dataflow.md).

**Create** (`POST /api/<plural>`):

```
HTTP POST → validate → duplicate-detect (search + match) →
  if duplicates: 409 Conflict with candidates
  else: persist (INSERT) → index (Tantivy / denormalised) →
        publish *Created event → audit-log → 201 response
```

Validation failures return `422`. See
[../../agents/share/dataflow.md](../../agents/share/dataflow.md).

**Match** (`POST /api/<plural>/match`, `/check-duplicates`):

```
HTTP POST → candidate selection (search / ILIKE) → fetch candidates →
  matcher.find_matches → score + classify (certain/probable/possible) → response
```

`match` is stateless ranking of an explicit `{query, candidates}` set;
`check-duplicates` ranks the query against stored records. See
[../../agents/share/match-search-merge.md](../../agents/share/match-search-merge.md).

**Merge** (`POST /api/<plural>/merge`):

```
HTTP POST → fetch main + duplicate → transfer data to main →
  update main → soft-delete duplicate → update index →
  publish *Merged event → return merge record (transferred-data snapshot)
```

Equal pids → `422`; unknown pid → `404`. See
[../../agents/share/merge.md](../../agents/share/merge.md).

---

## 6. Shared building blocks every service provides

Every service offers the same conceptual surface (implemented fully in
the MPI services; partially, with documented deferrals, in the loco
services).

| Block | Summary | Reference |
| ----- | ------- | --------- |
| CRUD + soft-delete | Create/read/update/delete with `active` flag, never hard-deleted | [../../agents/share/overview.md](../../agents/share/overview.md) |
| Identifiers | Multiple per record (type + system + value) | [../../agents/share/overview.md](../../agents/share/overview.md) |
| Matching | Probabilistic (weighted fuzzy) + deterministic (short-circuit) | [../../agents/share/match.md](../../agents/share/match.md) |
| Search | Full-text / fuzzy / phonetic (Tantivy in MPI; `ILIKE` in loco) | [../../agents/share/search.md](../../agents/share/search.md) |
| Merge | Confirmed-duplicate merge with link tracking + snapshot | [../../agents/share/merge.md](../../agents/share/merge.md) |
| Audit logging | HIPAA-style who/what/when, old/new values as JSON | [../../agents/share/auditability.md](../../agents/share/auditability.md) |
| Event streaming | `*Created/*Updated/*Deleted/*Merged` on every change | [../../agents/share/event-bus.md](../../agents/share/event-bus.md) |
| Validation | Required fields, format/range checks, normalization → `422` | [../../agents/share/overview.md](../../agents/share/overview.md) |
| Privacy | Masking, GDPR export, consent (MPI services; deferred in loco) | [../../agents/share/privacy.md](../../agents/share/privacy.md) |
| REST + OpenAPI | JSON REST API with Utoipa/Swagger docs | [../../agents/share/restful.md](../../agents/share/restful.md) |
| Observability | tracing + OpenTelemetry; Prometheus text exposition | [../../agents/share/observability.md](../../agents/share/observability.md) |

**Implemented vs planned (loco services):** CRUD, matching, name search
(`ILIKE`), merge, audit, in-memory event streaming, OpenAPI/Swagger, and
offline JWT verification are wired. **Deferred:** Tantivy full-text,
per-field privacy / GDPR export, durable event bus, blanket `/api/*` JWT
enforcement. The MPI services additionally ship Tantivy search, FHIR R5,
privacy/GDPR endpoints, and batch deduplication today; the gRPC API is a
stub in both styles.

---

## 7. Technology stack

The full dependency stack (Rust 2024, Tokio, Axum/loco, SeaORM,
PostgreSQL 18, Tantivy, Utoipa, OpenTelemetry, strsim, geo/haversine,
Podman, …) and its hard constraints (Podman not Docker, PostgreSQL not
SQLite, jiff not chrono, MiMalloc not jemalloc) are specified in:

- [../../agents/share/rust-loco-stack.md](../../agents/share/rust-loco-stack.md) — the canonical stack table + constraints
- [../../agents/share/loco.md](../../agents/share/loco.md) — loco backend conventions (no view tier; Postgres-backed background jobs)
- [../postgresql/index.md](../postgresql/index.md) — PostgreSQL spec
- [../../agents/share/postgresql.md](../../agents/share/postgresql.md) — PostgreSQL extensions
