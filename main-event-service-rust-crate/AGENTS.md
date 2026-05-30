# AGENTS — Main Event Service

Reference documentation for this crate.

## Crate-local docs (`AGENTS/`)

| Document | Description |
|----------|-------------|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/models.md](AGENTS/models.md) | Domain model reference (FHIR/event-style record) |
| [AGENTS/matching.md](AGENTS/matching.md) | Matching algorithm reference |
| [AGENTS/restful.md](AGENTS/restful.md) | RESTful API and library API reference |
| [AGENTS/testing.md](AGENTS/testing.md) | Testing strategy and guide |

## Shared docs (project root)

The shared reference docs live at the project root under [`../agents/share/`](../agents/share/), not under `AGENTS/share/`.

| Document | Description |
|----------|-------------|
| [overview.md](../agents/share/overview.md) | High-level project overview |
| [architecture.md](../agents/share/architecture.md) | Layered architecture |
| [stack-for-rust-loco.md](../agents/share/stack-for-rust-loco.md) | Full Rust + Loco dependency stack |
| [technology.md](../agents/share/technology.md) | Tech stack summary |
| [web-stack.md](../agents/share/web-stack.md) | Loco / Tera / HTMX / Alpine / Lily HTML Headless |
| [match-search-merge.md](../agents/share/match-search-merge.md) | Match / search / merge workflows |
| [match.md](../agents/share/match.md) | Matching algorithms |
| [search.md](../agents/share/search.md) | Search (Tantivy) |
| [merge.md](../agents/share/merge.md) | Merge workflow |
| [dataflow.md](../agents/share/dataflow.md) | Create / search / merge data flows |
| [privacy.md](../agents/share/privacy.md) | Masking, GDPR, consent |
| [auditability.md](../agents/share/auditability.md) | Audit logging and event streaming |
| [availability.md](../agents/share/availability.md) | Health checks, scaling |
| [observability.md](../agents/share/observability.md) | Tracing + OpenTelemetry (summary) |
| [observability-for-rust-loco.md](../agents/share/observability-for-rust-loco.md) | Tracing + OpenTelemetry (full) |
| [restful.md](../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](../agents/share/locales.md) | i18n & l10n |
| [compliance-for-healthcare.md](../agents/share/compliance-for-healthcare.md) | HIPAA, NHS, … |
| [compliance-for-technology.md](../agents/share/compliance-for-technology.md) | ISO, GDPR, … |

## Running this crate

```bash
# REST + gRPC API
cargo run --release

# Server-rendered web UI (Loco / Tera / HTMX / Alpine / Lily)
cargo run --bin web         # → http://0.0.0.0:5150
# Override port:
PORT=5180 cargo run --bin web

# Tests
cargo test --lib

# Benchmarks
cargo bench
```

## Web UI URL surface

| Path | Returns |
|------|---------|
| `GET /` | Home page (full HTML) |
| `GET /events` | Entity index (full HTML) |
| `GET /events/search/partial?q=…` | HTMX fragment for live search |
| `GET /static/css/lily.css` | NHS UK theme that styles the Lily HTML Headless components |
| `GET /static/js/htmx.min.js` | HTMX 2.0.4 |
| `GET /static/js/alpine.min.js` | Alpine 3.14.8 |

See [`../agents/share/web-stack.md`](../agents/share/web-stack.md) for the full web tier reference.
