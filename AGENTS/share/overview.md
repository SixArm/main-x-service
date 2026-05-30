# Project overview

The **Main X Index** family of crates implements a federated identity index — one crate per domain entity, sharing the same architecture, matching algorithms, and operational conventions.

| Crate | Entity | Purpose |
|-------|--------|---------|
| [main-person-service](../../main-person-service-rust-crate) | Person | General person identity registry (healthcare-aware) |
| [main-patient-index](../../main-patient-index-rust-crate) | Patient | Healthcare-specific patient identity registry |
| [main-worker-service](../../main-worker-service-rust-crate) | Worker | Workforce / professional identity registry |
| [main-place-service](../../main-place-service-rust-crate) | Place | Geographic place registry (schema.org/Place) |
| [main-thing-service](../../main-thing-service-rust-crate) | Thing | Generic thing / asset registry |
| [main-event-service](../../main-event-service-rust-crate) | Event | Time-bounded event registry |

## What every crate provides

- **CRUD** on the domain entity with soft-delete and full audit trail
- **Identifier management** (multiple identifiers per record; type + system + value)
- **Identity document management** (passport, driver's license, etc., where relevant)
- **Contact information management** (telecom / address / email)
- **Probabilistic matching** with weighted, configurable scoring
- **Deterministic matching** with short-circuit rules (tax ID, document, GLN, …)
- **Full-text search** via Tantivy with fuzzy and phonetic variants
- **Duplicate detection** (real-time on create, batch via deduplicate scan)
- **Record merging** with link tracking and transferred-data snapshots
- **Data quality validation** (required fields, format checks, ranges)
- **Address & phone normalization** at the boundary
- **Privacy controls**: per-field masking, GDPR data export, consent records
- **Event streaming** of every CRUD operation
- **Audit logging** (HIPAA-style trail for who/what/when)
- **REST API** (Axum) with OpenAPI / Swagger
- **Server-rendered web UI** (Loco / Tera / HTMX / Alpine / Lily Design System HTML Headless)
- **gRPC API** stub (Tonic) for high-throughput callers
- **Observability** (tracing + OpenTelemetry OTLP)
- **PostgreSQL persistence** via SeaORM with migrations

See [stack-for-rust-loco.md](stack-for-rust-loco.md) for the dependency stack, and [web-stack.md](web-stack.md) for the server-rendered UI tier.

## Running

Every subproject ships the same entry points:

```bash
# REST API server (existing)
cargo run --release

# Server-rendered web UI (Loco / Tera / HTMX / Alpine / Lily)
cargo run --bin web

# Tests
cargo test --lib

# Benchmarks (where available)
cargo bench
```

The web UI binds to `http://0.0.0.0:5150` by default (override with `PORT=…`).
