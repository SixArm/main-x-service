# Main X Service

@agents/share/overview.md

## Subprojects

| Crate | Entity | Path |
|-------|--------|------|
| Person Service | Person | [person-service-rust-crate](person-service-rust-crate/) |
| Place Service | Place | [place-service-rust-crate](place-service-rust-crate/) |
| Thing Service | Thing | [thing-service-rust-crate](thing-service-rust-crate/) |
| Event Service | Event | [event-service-rust-crate](event-service-rust-crate/) |
| Worker Service | Worker | [worker-service-rust-crate](worker-service-rust-crate/) |

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

See [agents/share/technology.md](agents/share/technology.md) for the full dependency inventory.

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
| [stack-for-rust-loco.md](agents/share/stack-for-rust-loco.md) | Full Rust + Loco dependency stack |
| [technology.md](agents/share/technology.md) | Tech stack summary |
| [match-search-merge.md](agents/share/match-search-merge.md) | Match / search / merge workflows |
| [match.md](agents/share/match.md) | Matching algorithms |
| [search.md](agents/share/search.md) | Search (Tantivy) |
| [merge.md](agents/share/merge.md) | Merge workflow |
| [dataflow.md](agents/share/dataflow.md) | Create / search / merge data flows |
| [privacy.md](agents/share/privacy.md) | Masking, GDPR, consent |
| [auditability.md](agents/share/auditability.md) | Audit logging + event streaming |
| [availability.md](agents/share/availability.md) | Health, scaling |
| [observability.md](agents/share/observability.md) | Tracing + OpenTelemetry (summary) |
| [observability-for-rust-loco.md](agents/share/observability-for-rust-loco.md) | Tracing + OpenTelemetry (full) |
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
