## 8. Architecture

### 8.1 Module layout

```
src/
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   ├── rest/                # REST API (Axum) — 15 endpoints
│   ├── fhir/                # FHIR R5 Worker resource
│   └── grpc/                # Tonic stub
├── models/                  # Worker, HumanName, Identifier, …
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

### 8.2 Layering rules

- `api/*` depends on `db`, `matching`, `search`, `streaming`,
  `validation`, `privacy`.
- `matching` and `search` MUST NOT depend on `api` or `db`
  repositories.
- `db` MUST NOT depend on `api`.
- `models` are leaves.

### 8.3 Trait-based abstraction

| Trait | Implementations |
|---|---|
| `WorkerRepository` | `SeaOrmWorkerRepository` |
| `WorkerMatcher` | `ProbabilisticMatcher`, `DeterministicMatcher` |
| `EventProducer` | `InMemoryEventPublisher` (Fluvio planned) |
| `EventConsumer` | stub |

### 8.4 Application state

`AppState` (`src/api/rest/state.rs`) holds `db`, `worker_repository`,
`event_publisher`, `audit_log`, `search_engine`, `matcher`, `config`.

### 8.5 Data flow

**Create:** HTTP POST → Validation → Duplicate detection → Repository
INSERT → Search Index → Event Publish → Audit Log → Response.

**Match:** HTTP POST → Search engine (blocking candidates) →
Repository GET → `Matcher::find_matches` → score + classify → Response.

**Merge:** HTTP POST → fetch both → transfer data → update survivor →
soft-delete duplicate → update index → publish `Merged` → Response.

**Link:** HTTP POST `/api/v1/workers/{pid}/links` → validate edge kind +
`EntityRef` → upsert `entity_links` row (optimistic; **no** cross-service
call) → publish `linked` → Response. Unlink (DELETE) soft-deletes the row
and publishes `unlinked`. See §8.6.

### 8.6 Cross-service link events

Worker originates cross-service edges (§5.4) and publishes them on the
**existing** event envelope via the existing `EventProducer` — no new
transport, no new outbox path. Two `kind` values are added alongside the
CRUD/merge events:

- `linked` — emitted on edge create/upsert. Envelope `entity` = `worker`,
  `pid` = the worker; the edge detail (`edge_id`, `from_ref`, `to_ref`,
  `edge_kind`, `role`, `confidence`, `provenance`, `valid_from`,
  `valid_to`) rides in `data`.
- `unlinked` — emitted on soft-delete; carries `{edge_id}` and the refs
  so the read-side aggregator can remove the edge.

The matching/adapter layer MUST NOT project `entity_links` into matcher
input (partition rule, §5.1). See
[cross-service linking §4.2](../../../agents/share/cross-service-linking.md).

