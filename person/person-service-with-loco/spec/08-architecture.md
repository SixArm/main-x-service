## 8. Architecture

### 8.1 Module layout

```
src/
├── bin/main.rs              # binary entry: cli::main::<App, Migrator>()
├── app.rs                   # loco App Hooks (routes, after_routes, queue)
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   ├── rest/                # REST API (Axum) — 15 endpoints, mounted at /api
│   ├── fhir/                # FHIR R5 Person + bundle stubs
│   └── grpc/                # Tonic stub
├── models/                  # Person, HumanName, Identifier, …
├── db/                      # SeaORM entities + repositories + audit
├── matching/                # algorithms + scoring + phonetic
├── search/                  # Tantivy index + query
├── streaming/               # EventProducer trait + InMemoryEventPublisher
├── validation/              # boundary validators + normalisers
├── privacy/                 # masking + GDPR export + consent
├── config/                  # env loading + Config struct
├── observability/           # OTLP setup
├── error.rs
└── lib.rs
```

The crate boots through **loco.rs**. The binary target is
`src/bin/main.rs`, whose `main()` calls
`loco_rs::cli::main::<App, Migrator>()` — loco parses the subcommands
(`start`, `db migrate`, `task`, …) and dispatches against the crate's
`App` hooks (`src/app.rs`) and the migration crate's `Migrator`. There
is no hand-rolled `Config::from_env → AppState → serve` path.

`App` (in `src/app.rs`) implements loco's `Hooks`:

- `routes()` registers the loco `Routes` — `persons_routes()` (the `/api`
  surface) and `metrics_routes()` (root `/metrics.prom`).
- `after_routes()` builds the boot-time singletons (domain `Config` via
  `Config::from_env`, the Tantivy `SearchEngine`, the
  `ProbabilisticMatcher`), constructs `AppState`, inserts it into loco's
  `shared_store`, and merges the Swagger UI plus a permissive CORS layer
  onto loco's Axum router.

Migrations run via the loco CLI (`cargo loco db migrate`) and are
**auto-run in development** (`auto_migrate`, per
[`README.md`](../README.md) / `config/development.yaml`); production
applies them explicitly. The hand-written `create_router` /
`SeaOrmPersonRepository`-backed `AppState` is retained for the
integration tests.

### 8.2 Layering rules

- `api/*` depends on `db`, `matching`, `search`, `streaming`,
  `validation`, `privacy`.
- `matching` and `search` MUST NOT depend on `api` or `db`
  repositories — they take values, not connections.
- `db` MUST NOT depend on `api`.
- `models` are leaves — they depend on `serde`, `chrono`, `uuid` only.

### 8.3 Trait-based abstraction

| Trait | Implementations |
|---|---|
| `PersonRepository` | `SeaOrmPersonRepository` |
| `PersonMatcher` | `ProbabilisticMatcher`, `DeterministicMatcher` |
| `EventProducer` | `InMemoryEventPublisher` (Fluvio planned) |
| `EventConsumer` | stub |

### 8.4 Application state

`AppState` (`src/api/rest/state.rs`) holds:
`db`, `person_repository: Arc<dyn PersonRepository>`,
`event_publisher: Arc<dyn EventProducer>`,
`audit_log: Arc<AuditLogRepository>`,
`search_engine: Arc<SearchEngine>`,
`matcher: Arc<dyn PersonMatcher>`,
`config: Arc<Config>`.

### 8.5 Data flow

**Create:** HTTP POST → Validation → Duplicate detection → Repository
INSERT → Search Index → Event Publish → Audit Log → Response.

**Match:** HTTP POST → Search engine (blocking candidates) → Repository
GET → `Matcher::find_matches` → score + classify → Response.

**Merge:** HTTP POST → fetch both → transfer data → update main →
soft-delete duplicate → update index → publish `Merged` → Response.

### 8.6 Cross-service entity links (write side)

Per [cross-service linking](../../../agents/share/cross-service-linking.md),
the Person Service originates outbound cross-service edges (§5.4) without
calling the target service. The write path is **optimistic**:

**Link:** HTTP POST `/api/persons/{pid}/links` → validate edge kind +
`to_ref` → upsert into `entity_links` (§10.4) → publish `linked` event →
Response. No cross-service call.

**Unlink:** HTTP DELETE `/api/persons/{pid}/links/{id}` → soft-delete
the row (`deleted_at`) → publish `unlinked` event → Response.

The `linked` / `unlinked` events are two new `kind` values on the
**existing** event envelope and reuse the same `EventProducer` /
outbox path — no new transport
([cross-service linking §4.2](../../../agents/share/cross-service-linking.md)).
The envelope's `entity`/`pid` are the **from** (person) side; the edge
detail (`edge_id`, `from_ref`, `to_ref`, `edge_kind`, `role`,
`confidence`, `provenance`, `valid_from`/`valid_to`) rides in `data`.

The matching adapter (`src/matching/adapter.rs`) MUST NOT read
`entity_links` — cross-service links are never a match signal (the
partition rule, §5.1; [cross-service linking §7](../../../agents/share/cross-service-linking.md)).

