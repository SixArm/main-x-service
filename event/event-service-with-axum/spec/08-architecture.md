## 8. Architecture

### 8.1 Module layout

```
src/
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   ├── rest/                # /api/* — 18 endpoints
│   └── grpc/                # real Tonic server (T-6): mod.rs (serve + generated
│                             # proto module) + service.rs (EventGrpcService)
├── controllers/
│   └── fhir.rs              # mounted FHIR R5 Appointment routes + /fhir/metadata
├── fhir/                    # FHIR resources, conversions, OperationOutcome, Bundle, CapabilityStatement
├── models/                  # Event, Location, Party, Offer, …
├── db/                      # SeaORM entities + repositories + audit + outbox
├── matching/                # algorithms + scoring + phonetic
├── search/                  # Tantivy index + query
├── streaming/               # legacy EventProducer trait + InMemoryEventPublisher (ring buffer) + the canonical Envelope/EventTransport
├── relay.rs                 # durable-bus relay: EventSink trait, LoggingSink, FluvioSink (fluvio feature)
├── compliance/               # keyed integrity: mac, record_integrity, audit_integrity
├── validation/              # boundary validators + normalisers
├── privacy/                 # masking + GDPR export + consent
├── config/                  # env loading + Config struct
├── observability/           # OTel Resource + JSON tracing subscriber (OTLP export not yet wired)
├── metrics.rs               # Prometheus text-exposition (GET /metrics.prom)
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
| `EventRepository` | `SeaOrmEventRepository` |
| `EventMatcher` | `ProbabilisticMatcher`, `DeterministicMatcher` |
| `EventProducer` (legacy in-process ring buffer) | `InMemoryEventPublisher` — production delivery went a different route, below (the dead `FluvioProducer` stub was removed, PRO-H4) |
| `EventConsumer` | none — the dead `FluvioConsumer` stub was removed (PRO-H4) |
| `EventSink` (durable-bus relay, `src/relay.rs`, T-11) | `LoggingSink` (default, no broker); `FluvioSink` (BUS-3, real broker, behind the `fluvio` Cargo feature — off by default) |

### 8.4 Application state

`AppState` (`src/api/rest/state.rs`) holds `db`, `event_repository`,
`event_publisher`, `audit_log`, `search_engine`, `matcher`, `config`.

### 8.5 Data flow

**Create:** HTTP POST → Validation → Duplicate detection (blocking on
name + start-date) → Repository INSERT → Search Index → Event Publish
→ Audit Log → Response.

**Match:** HTTP POST → Search engine (date-window candidates) →
Repository GET → `Matcher::find_matches` → score + classify → Response.

**Merge:** HTTP POST → fetch both → transfer data → update survivor →
soft-delete duplicate → update index → publish `Merged` → Response.

