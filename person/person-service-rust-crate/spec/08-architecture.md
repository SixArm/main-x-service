## 8. Architecture

### 8.1 Module layout

```
src/
├── main.rs                  # binary entry: Config::from_env → AppState → api::rest::serve
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

`src/main.rs` is the binary target (`cargo run --release` /
`target/release/person-service`). It calls `Config::from_env()` (reads
the env-var table documented on `Config::from_env`, with defaults), opens
the database pool via `db::create_connection`, opens / creates the
Tantivy index at `config.search.index_path`, constructs the
`ProbabilisticMatcher`, builds `AppState`, and hands off to
`api::rest::serve(state)`. Migrations are NOT auto-run — the bring-up
sequence in [`README.md`](../README.md) shows how to apply
`migrations/*` before launching.

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

