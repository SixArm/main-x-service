# Main X Service

@agents/share/overview.md

## Subprojects

Subprojects are grouped one directory per entity. Each entity
directory holds a front-end web app, a matcher (or verifier) library
crate, a service API crate, and entity-level `spec/` + `AGENTS/`
umbrella docs.

| Entity | Service | Library | Front-end | Umbrella |
|--------|---------|---------|-----------|----------|
| Person | [person-service-rust-crate](person/person-service-rust-crate/) | [person-matcher-rust-crate](person/person-matcher-rust-crate/) | [person-front-end-with-svelte](person/person-front-end-with-svelte/) | [spec](person/spec/index.md) |
| Worker | [worker-service-rust-crate](worker/worker-service-rust-crate/) | [worker-matcher-rust-crate](worker/worker-matcher-rust-crate/) | [worker-front-end-with-svelte](worker/worker-front-end-with-svelte/) | [spec](worker/spec/index.md) |
| Place | [place-service-rust-crate](place/place-service-rust-crate/) | [place-matcher-rust-crate](place/place-matcher-rust-crate/) | [place-front-end-with-svelte](place/place-front-end-with-svelte/) | [spec](place/spec/index.md) |
| Thing | [thing-service-rust-crate](thing/thing-service-rust-crate/) | [thing-matcher-rust-crate](thing/thing-matcher-rust-crate/) | [thing-front-end-with-svelte](thing/thing-front-end-with-svelte/) | [spec](thing/spec/index.md) |
| Event | [event-service-rust-crate](event/event-service-rust-crate/) | [event-matcher-rust-crate](event/event-matcher-rust-crate/) | [event-front-end-with-svelte](event/event-front-end-with-svelte/) | [spec](event/spec/index.md) |
| Course | [course-service-rust-crate](course/course-service-rust-crate/) | [course-matcher-rust-crate](course/course-matcher-rust-crate/) | [course-front-end-with-svelte](course/course-front-end-with-svelte/) | [spec](course/spec/index.md) |
| Organization | [organization-service-rust-crate](organization/organization-service-rust-crate/) | [organization-matcher-rust-crate](organization/organization-matcher-rust-crate/) | [organization-front-end-with-svelte](organization/organization-front-end-with-svelte/) | [spec](organization/spec/index.md) |
| Care pathway | [care-pathway-service-rust-crate](care-pathway/care-pathway-service-rust-crate/) | [care-pathway-matcher-rust-crate](care-pathway/care-pathway-matcher-rust-crate/) | [care-pathway-front-end-with-svelte](care-pathway/care-pathway-front-end-with-svelte/) | [spec](care-pathway/spec/index.md) |
| Authentication | [authentication-service-rust-crate](authentication/authentication-service-rust-crate/) | [authentication-verifier-rust-crate](authentication/authentication-verifier-rust-crate/) | [authentication-front-end-with-svelte](authentication/authentication-front-end-with-svelte/) | [spec](authentication/spec/index.md) |

Each crate is self-contained: it owns its REST API, its persistence schema, and its matching algorithm. They share an architecture and a documentation layout, not code.

## What every crate does

- **CRUD** on its entity with soft delete and an audit trail
- **Probabilistic + deterministic matching** with weighted, configurable scoring
- **Full-text search** via Tantivy (fuzzy, phonetic where applicable)
- **Duplicate detection** real-time on create and via batch deduplicate scans
- **Record merging** with link tracking and a transferred-data snapshot
- **Data validation** at the create/update boundary (returns `422` on failure)
- **Privacy controls** — per-field masking, GDPR export, consent records
- **Event streaming** of CRUD operations
- **REST API** (Axum) with OpenAPI/Swagger
- **gRPC stub** (Tonic) for high-throughput callers
- **Observability** — `tracing` + OpenTelemetry OTLP

See [agents/share/rust-loco-stack.md](agents/share/rust-loco-stack.md) for the full dependency inventory.

## Running

From any subproject root:

```bash
# REST + gRPC API
cargo run --release

# Tests
cargo test --lib

# Benchmarks (where available)
cargo bench
```

## Documentation

Top-level reference docs in [`agents/share/`](agents/share/):

| File | Purpose |
|------|---------|
| [overview.md](agents/share/overview.md) | High-level project overview |
| [architecture.md](agents/share/architecture.md) | Layered architecture |
| [rust-loco-stack.md](agents/share/rust-loco-stack.md) | Full Rust + Loco dependency stack |
| [loco.md](agents/share/loco.md) | Loco framework (backend-only conventions) |
| [match-search-merge.md](agents/share/match-search-merge.md) | Match / search / merge workflows |
| [match.md](agents/share/match.md) | Matching algorithms |
| [search.md](agents/share/search.md) | Search (Tantivy) |
| [merge.md](agents/share/merge.md) | Merge workflow |
| [dataflow.md](agents/share/dataflow.md) | Create / search / merge data flows |
| [privacy.md](agents/share/privacy.md) | Masking, GDPR, consent |
| [auditability.md](agents/share/auditability.md) | Audit logging + event streaming |
| [availability.md](agents/share/availability.md) | Health, scaling |
| [observability.md](agents/share/observability.md) | Tracing + OpenTelemetry (summary) |
| [rust-tracing-opentelemetry-stack.md](agents/share/rust-tracing-opentelemetry-stack.md) | Tracing + OpenTelemetry (full) |
| [restful.md](agents/share/restful.md) | REST API conventions |
| [postgresql.md](agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](agents/share/locales.md) | i18n & l10n |
| [compliance-for-healthcare.md](agents/share/compliance-for-healthcare.md) | HIPAA, NHS, … |
| [compliance-for-technology.md](agents/share/compliance-for-technology.md) | ISO, GDPR, … |

Per-crate reference docs live in `<crate>/AGENTS/`:

- `index.md` — directory of the crate's local docs
- `models.md` — domain model reference
- `matching.md` — per-crate matching tuning
- `restful.md` — REST API surface
- `testing.md` — test layout

## Architecture snapshot

```
┌─────────────────────────────────────────────────────────────┐
│ Client (curl / SDK / gRPC client)                           │
└────────────────────────────┬────────────────────────────────┘
                             │
            ┌────────────────▼────────────────┐
            │ REST API (Axum) + gRPC (Tonic)  │
            │ + OpenAPI/Swagger UI            │
            │ /api/<plural>/…                 │
            └────────────────┬────────────────┘
                             │
            ┌────────────────▼────────────────┐
            │ Application logic               │
            │  • Validation & normalization   │
            │  • Matching (probabilistic +    │
            │     deterministic)              │
            │  • Privacy (masking, GDPR)      │
            │  • Audit log emission           │
            └────────────────┬────────────────┘
                             │
        ┌────────────────────┼─────────────────────┐
        │                    │                     │
┌───────▼──────┐    ┌────────▼────────┐   ┌────────▼────────┐
│ PostgreSQL   │    │ Tantivy index   │   │ Event stream    │
│ (SeaORM)     │    │ (full-text +    │   │ (Fluvio /       │
│              │    │  fuzzy/phonetic)│   │  in-memory)     │
└──────────────┘    └─────────────────┘   └─────────────────┘
```

## Status

Backend-only Rust services. All crates compile cleanly and pass their lib tests.

## License

Each crate is multi-licensed; see the individual `Cargo.toml` for terms.
