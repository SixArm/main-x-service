# Project overview

The **Main X Index** family of crates implements a federated identity index — one crate per domain entity, sharing the same architecture, matching algorithms, and operational conventions.

### Service crates

| Crate | Entity | Purpose |
|-------|--------|---------|
| [person-service](../../person-service-rust-crate) | Person | General person identity registry |
| [worker-service](../../worker-service-rust-crate) | Worker | Workforce / professional identity registry |
| [place-service](../../place-service-rust-crate) | Place | Geographic place registry (schema.org/Place) |
| [thing-service](../../thing-service-rust-crate) | Thing | Generic thing / asset registry (schema.org/Thing) |
| [event-service](../../event-service-rust-crate) | Event | Time-bounded event registry (schema.org/Event) |

### Matcher crates

Reusable, dependency-light Rust libraries for pairwise record
comparison. Each is usable standalone and is the canonical reference
implementation embedded in the corresponding service crate's
`src/matching/` layer. Their per-crate `spec.md` follows a distinct
§1–§25 SDD shape (research basis, algorithm specifications, normalization
specifications, …) tailored to library-style work.

| Crate | Entity | Purpose |
|-------|--------|---------|
| [person-matcher](../../person-matcher-rust-crate) | Person | Demographic + multinational national-identifier matching |
| [worker-matcher](../../worker-matcher-rust-crate) | Worker | Workforce / professional identity matching |
| [place-matcher](../../place-matcher-rust-crate) | Place | Geographic / postal-address / venue matching |
| [thing-matcher](../../thing-matcher-rust-crate) | Thing | Generic thing / asset matching |
| [event-matcher](../../event-matcher-rust-crate) | Event | Time-bounded event matching with window-overlap |

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
- **gRPC API** stub (Tonic) for high-throughput callers
- **Observability** (tracing + OpenTelemetry OTLP)
- **PostgreSQL persistence** via SeaORM with migrations

See [stack-for-rust-loco.md](stack-for-rust-loco.md) for the dependency stack.

## Running

Every subproject ships the same entry points:

```bash
# REST API server
cargo run --release

# Tests
cargo test --lib

# Benchmarks (where available)
cargo bench
```
