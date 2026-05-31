# AGENTS directory — Thing Service

Detailed reference documentation for the Thing Service crate.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline — three-part PRs, spec-section mapping, anti-patterns |
| [models.md](models.md) | Domain model reference (`Thing`, supporting types, invariants) |
| [matching.md](matching.md) | Matching algorithm reference (weights, components, rules) |
| [restful.md](restful.md) | REST API + library API reference |
| [testing.md](testing.md) | Testing strategy and guide (unit, integration, benchmark) |

## Shared documents (project root)

The shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/).

| Document | Description |
|----------|-------------|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [web-stack.md](../../agents/share/web-stack.md) | Loco / Tera / HTMX / Alpine / Lily HTML Headless |
| [web-pages.md](../../agents/share/web-pages.md) | Per-page contracts for all 26 web-tier pages |
| [technology.md](../../agents/share/technology.md) | Tech stack summary |
| [stack-for-rust-loco.md](../../agents/share/stack-for-rust-loco.md) | Full dependency inventory |
| [match-search-merge.md](../../agents/share/match-search-merge.md) | Match / search / merge workflows |
| [match.md](../../agents/share/match.md) | Matching algorithms (cross-crate) |
| [search.md](../../agents/share/search.md) | Search (Tantivy) |
| [merge.md](../../agents/share/merge.md) | Merge workflow |
| [dataflow.md](../../agents/share/dataflow.md) | Create / search / merge data flows |
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent |
| [auditability.md](../../agents/share/auditability.md) | Audit logging and event streaming |
| [availability.md](../../agents/share/availability.md) | Health checks, scaling |
| [observability.md](../../agents/share/observability.md) | Tracing + OpenTelemetry summary |
| [observability-for-rust-loco.md](../../agents/share/observability-for-rust-loco.md) | Tracing + OpenTelemetry full |
| [restful.md](../../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](../../agents/share/locales.md) | i18n & l10n |
| [compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md) | HIPAA, NHS, … |
| [compliance-for-technology.md](../../agents/share/compliance-for-technology.md) | ISO, GDPR, … |

## See also

- [`../spec.md`](../spec.md) — single source of truth for this crate
- [`../AGENTS.md`](../AGENTS.md) — agent-facing entry point
- [`../README.md`](../README.md) / [`../CLAUDE.md`](../CLAUDE.md) — user-facing intro
- [`../index.md`](../index.md) — navigation aid + worked examples
