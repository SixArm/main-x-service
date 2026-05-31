# AGENTS — Place Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec.md). When in doubt, the spec wins. See
[`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`AGENTS/`)

| Document | Description |
|----------|-------------|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [AGENTS/models.md](AGENTS/models.md) | Domain model reference (Place-specific) |
| [AGENTS/matching.md](AGENTS/matching.md) | Matching algorithm reference (weights, components, rules) |
| [AGENTS/restful.md](AGENTS/restful.md) | REST API + library API reference |
| [AGENTS/testing.md](AGENTS/testing.md) | Testing strategy and guide |

## Shared docs (project root)

Shared reference docs live at the project root under
[`../agents/share/`](../agents/share/).

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
PORT=5180 cargo run --bin web

# Tests
cargo test --lib                                # unit
cargo test --tests                              # integration (needs DATABASE_URL)
DATABASE_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## Web UI URL surface

| Path | Returns |
|------|---------|
| `GET /` | Home page (full HTML) |
| `GET /places` | Entity index (full HTML) |
| `GET /places/search/partial?q=…` | HTMX fragment for live search |
| `GET /static/css/lily.css` | NHS UK theme that styles the Lily HTML Headless components |
| `GET /static/js/htmx.min.js` | HTMX 2.0.4 |
| `GET /static/js/alpine.min.js` | Alpine 3.14.8 |

See [`../agents/share/web-stack.md`](../agents/share/web-stack.md) for
the full web tier reference and the complete per-entity URL surface.

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` / `CLAUDE.md` | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `AGENTS/*.md` | How to work in the repo + per-topic reference |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |
