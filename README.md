# Main X Index Rust crates

@agents/share/overview.md

## Subprojects

| Crate | Entity | Path |
|-------|--------|------|
| Main Person Index | Person | [main-person-index-rust-crate](main-person-index-rust-crate/) |
| Main Patient Index | Patient | [main-patient-index-rust-crate](main-patient-index-rust-crate/) |
| Main Worker Index | Worker | [main-worker-index-rust-crate](main-worker-index-rust-crate/) |
| Main Place Index | Place | [main-place-index-rust-crate](main-place-index-rust-crate/) |
| Main Thing Index | Thing | [main-thing-index-rust-crate](main-thing-index-rust-crate/) |
| Main Event Index | Event | [main-event-index-rust-crate](main-event-index-rust-crate/) |

Each crate is self-contained: it owns its REST API, its persistence schema, its matching algorithm, and its web UI. They share an architecture and a documentation layout, not code.

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
- **Server-rendered web UI** — Loco / Tera / HTMX / Alpine / Lily Design System
- **gRPC stub** (Tonic) for high-throughput callers
- **Observability** — `tracing` + OpenTelemetry OTLP

See [agents/share/web-stack.md](agents/share/web-stack.md) for the web UI tier and [agents/share/technology.md](agents/share/technology.md) for the full dependency inventory.

## Running

From any subproject root:

```bash
# REST + gRPC API
cargo run --release

# Server-rendered web UI (Loco / Tera / HTMX / Alpine / Lily)
cargo run --bin web
# → http://0.0.0.0:5150 (override with PORT=…)

# Tests
cargo test --lib

# Benchmarks (where available)
cargo bench
```

## Endpoints (web UI)

| Path | Returns |
|------|---------|
| `GET /` | Home page (full HTML, Tera-rendered) |
| `GET /{plural}` | Entity index (full HTML) |
| `GET /{plural}/search/partial?q=…` | HTMX fragment for live search |
| `GET /static/css/lily.css` | Lily Design System styles |
| `GET /static/js/htmx.min.js` | HTMX 2.0.4 |
| `GET /static/js/alpine.min.js` | Alpine 3.14.8 |

`{plural}` is `persons` / `patients` / `workers` / `places` / `things` / `events`.

## Documentation

Top-level reference docs in [`agents/share/`](agents/share/):

| File | Purpose |
|------|---------|
| [overview.md](agents/share/overview.md) | High-level project overview |
| [architecture.md](agents/share/architecture.md) | Layered architecture |
| [stack-for-rust-loco.md](agents/share/stack-for-rust-loco.md) | Full Rust + Loco dependency stack |
| [web-stack.md](agents/share/web-stack.md) | Loco / Tera / HTMX / Alpine / Lily |
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
│ Client (browser / curl / SDK)                               │
└────────────────────────────┬────────────────────────────────┘
                             │
            ┌────────────────┴────────────────┐
            │                                 │
┌───────────▼────────────┐       ┌────────────▼─────────────┐
│ Web UI (Loco/Tera/     │       │ REST API (Axum)          │
│ HTMX/Alpine/Lily)      │       │  + OpenAPI/Swagger UI    │
│ /, /{plural}, /static  │       │ /api/<plural>/…          │
└───────────┬────────────┘       └────────────┬─────────────┘
            │                                 │
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

All six crates compile cleanly and pass their lib tests (629 tests total). The web binary boots from each crate and serves the home page, entity index, HTMX partial, and static assets.

## License

Each crate is dual-licensed; see the individual `Cargo.toml` for terms.
