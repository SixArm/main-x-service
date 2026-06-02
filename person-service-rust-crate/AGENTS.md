# AGENTS — Person Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec.md). When in doubt, the spec wins. See
[`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`AGENTS/`)

| Document | Description |
|----------|-------------|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [AGENTS/models.md](AGENTS/models.md) | Domain model reference (`Person`, `HumanName`, supporting types) |
| [AGENTS/matching.md](AGENTS/matching.md) | Matching algorithm reference (weights, rules, components) |
| [AGENTS/restful.md](AGENTS/restful.md) | REST API + FHIR R5 + library API reference |
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

# Tests
cargo test --lib                                # unit
cargo test --tests                              # integration (needs DATABASE_URL)
DATABASE_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` / `CLAUDE.md` | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `AGENTS/*.md` | How to work in the repo + per-topic reference |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |
