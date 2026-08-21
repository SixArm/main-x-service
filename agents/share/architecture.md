# Architecture

The Main X Index is a **family of Rust services** sharing one architecture,
one matching approach, and one set of operational conventions. Every service
boots on **loco.rs** (Axum + SeaORM + PostgreSQL); the differences are which
*internal shape* a crate uses and which capabilities it carries (see the
capability matrix in [overview.md](overview.md)).

## The family at a glance

- **Ten entity-registry services** — person, worker, place, thing, event,
  course, organization, care-pathway, case, portfolio — each a CRUD +
  matching registry for one domain entity, embedding the sibling
  `*-matcher` crate.
- **authentication-service** — the central SSO provider (not a registry):
  passwordless magic-link login, Postgres cookie sessions, and PASETO
  v4.public token **issuance** with a published key set.
- **link-graph-service** — the read-model **aggregator** for cross-service
  links (read-only to the world; writes are event-driven).
- **Library crates** — the `*-matcher` crates (pairwise comparison),
  `authentication-verifier` (offline PASETO + the shared ABAC engine), and
  `entity-ref` (the cross-service `EntityRef` URN + edge-kind registry).
- **SvelteKit front-ends** — one operator SPA per entity (out of scope here).

## Layered request flow

```
+------------------------------------------------------------------+
|  Clients (operator SPAs, peer services, EHR/analytics)           |
+---------------------------------+--------------------------------+
                                  |
+---------------------------------v--------------------------------+
|  API layer (Axum, mounted by loco)                               |
|   REST + OpenAPI/Swagger  ·  FHIR R5 (8 crates)  ·  gRPC (3)      |
|   blanket ABAC guard (<ENTITY>_REQUIRE_AUTH, default-off)         |
+---------------------------------+--------------------------------+
                                  |
+---------------------------------v--------------------------------+
|  Domain logic                                                    |
|   matching (embeds *-matcher)  ·  validation → 422               |
|   duplicate detection + record merge  ·  privacy masking         |
|   event emit (CRUD + linked/unlinked)  ·  audit                  |
+---------------------------------+--------------------------------+
                                  |
        +-------------------------+-------------------------+
        |                         |                         |
+-------v--------+   +------------v-----------+  +----------v---------+
| PostgreSQL     |   | Tantivy full-text      |  | Event transport    |
| (SeaORM +      |   | index (6 crates; the   |  | in-memory (default)|
|  migrations)   |   | rest use ILIKE)        |  | or Postgres outbox |
| entity rows,   |   |                        |  | → relay → Fluvio   |
| audit, outbox, |   +------------------------+  +----------+---------+
| entity_links   |                                          |
+----------------+                          (link/created/… events)
                                                            |
                                          +-----------------v---------+
                                          | link-graph aggregator     |
                                          | (edges read-model,        |
                                          |  neighbors/single-view,   |
                                          |  reconcile)               |
                                          +---------------------------+
```

## Two internal shapes

Both boot identically through a loco `Hooks` impl in `src/app.rs`; they
differ in how routes and persistence are organised.

### person-style (`src/api/rest/`)

The older hand-rolled Axum layout, now mounted under loco. Used by
**person, worker, course** (and **place / thing / event**, which are
mid-conversion and also carry a `src/controllers/` surface). A rich
domain model with per-field tables.

```
src/
├── app.rs                 loco Hooks (boot, routes, workers)
├── api/rest/              mod · handlers · routes · state (AppState) · auth · links
├── api/fhir/  api/grpc/   FHIR R5 resource + gRPC stub
├── models/                domain model (HumanName, Identifier, Address, …)
├── db/                    mod · models (SeaORM) · repositories · audit · outbox · convert
├── matching/              adapter to the *-matcher crate + scoring
├── search/                Tantivy index + query
├── streaming/             mod · envelope (durable Envelope) · producer · consumer
├── validation/  privacy/  boundary validation + masking
└── bulk/                  (person only) import/export codecs + jobs
```

### loco-style (`src/controllers/`)

The newer loco-idiomatic layout. Used by **organization, care-pathway,
case, portfolio** (and **link-graph**, **authentication**). The API DTO
**is** the matcher type, stored verbatim as JSONB — no separate model to
drift.

```
src/
├── app.rs                 loco Hooks (routes, workers, boot init)
├── controllers/           one module per resource (+ docs, metrics; fhir where present)
├── models/                CRUD helpers over the JSONB payload + _entities/ (SeaORM)
├── auth.rs                offline PASETO verify + ABAC (authorize_record)
├── merge.rs               pure record-merge logic
├── streaming.rs           durable Envelope + EventPublisher seam (single file)
├── validation.rs          payload validation → 422
└── openapi.rs             hand-written OpenAPI 3 doc
```

Migrations live in a crate-root `migration/` directory (a `sea-orm-migration`
migrator) in both shapes — `authentication-service` is the exception, with
`src/migration/`.

## Cross-cutting subsystems

- **Authentication & authorization.** The auth-service issues short-lived
  **PASETO v4.public** tokens from cookie sessions and publishes its
  Ed25519 keys at `/.well-known/paseto-keys`. Every other service verifies
  **offline** via the embedded `authentication-verifier` (no shared secret,
  no introspection hop) and authorizes with the crate's shared **ABAC**
  engine — a blanket `/api/*` guard (`<ENTITY>_REQUIRE_AUTH`, default-off)
  plus opt-in record-level checks (`authorize_record`). See
  [authentication-sessions.md](authentication-sessions.md),
  [authorization-attributes.md](authorization-attributes.md).
- **Event bus.** Every CRUD/merge (and `linked`/`unlinked`) emits a
  canonical versioned `Envelope`. Transport is selected per service by
  `<ENTITY>_EVENT_TRANSPORT` (default `memory`); the durable path writes a
  Postgres **outbox** row *inside the entity's transaction* (no committed
  change without its event), later relayed to Fluvio. See
  [event-bus.md](event-bus.md).
- **Cross-service linking.** Each originating service records outbound edges
  in its own `entity_links` table and emits `linked`/`unlinked`; the
  **link-graph aggregator** consumes the stream into a queryable `edges`
  read-model and reconciles against each service's authoritative edges. The
  `EntityRef` URN + edge-kind registry live in the shared `entity-ref` crate.
  See [cross-service-linking.md](cross-service-linking.md).

## Shared design patterns

- **Trait-based seams.** A repository/model layer, a matcher (`*Matcher` /
  the embedded engine), and an `EventProducer` / `EventPublisher` are traits,
  so persistence, matching, and transport are swappable and unit-testable.
- **Shared state.** person-style holds services in `AppState`
  (`api/rest/state.rs`); loco-style reads loco's `AppContext` in each
  controller. Both expose the DB, the matcher, the event publisher, and the
  audit log.
- **Typed errors.** A crate-local `Error` enum (`thiserror`) with a `Result`
  alias throughout; the API layer maps it to the right HTTP status.
- **`#![forbid(unsafe_code)]`** on every crate root.

## Data flows

**Create** → validate → duplicate-detect (search + match) → (409 with
candidates on a hit) → persist → index (Tantivy, where present) → emit
`created` (+ audit) → respond.

**Merge** → fetch survivor + duplicate → transfer data → update survivor →
soft-delete duplicate → re-index → emit `merged` (with `merged_from`) →
respond.

**Link** → validate the edge kind + `EntityRef` → upsert `entity_links`
(optimistic; no cross-service call) → emit `linked` (durably, in one
transaction, under the outbox transport) → respond. The aggregator verifies
and repoints asynchronously.

See [dataflow.md](dataflow.md) for the per-flow detail and
[rust-loco-stack.md](rust-loco-stack.md) for the dependency stack.
