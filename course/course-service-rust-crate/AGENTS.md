# AGENTS — Course Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`AGENTS/`)

| Document | Description |
|---|---|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [AGENTS/models.md](AGENTS/models.md) | Domain model reference (`Course`, `CourseInstance`, schema.org property mapping) |
| [AGENTS/matching.md](AGENTS/matching.md) | Matching algorithm reference (weights, rules, components) |
| [AGENTS/restful.md](AGENTS/restful.md) | REST API + library API reference |
| [AGENTS/testing.md](AGENTS/testing.md) | Testing strategy and guide |

## Shared docs (project root)

Shared reference docs live at the project root under
[`../agents/share/`](../agents/share/).

| Document | Description |
|---|---|
| [overview.md](../agents/share/overview.md) | High-level project overview |
| [architecture.md](../agents/share/architecture.md) | Layered architecture |
| [stack-for-rust-loco.md](../agents/share/stack-for-rust-loco.md) | Full Rust + Loco dependency stack |
| [technology.md](../agents/share/technology.md) | Tech stack summary |
| [match-search-merge.md](../agents/share/match-search-merge.md) | Match / search / merge workflows |
| [restful.md](../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../agents/share/postgresql.md) | PostgreSQL conventions |
| [auditability.md](../agents/share/auditability.md) | Audit-log conventions |
| [privacy.md](../agents/share/privacy.md) | Masking, GDPR, consent |
| [observability.md](../agents/share/observability.md) | Tracing + OpenTelemetry summary |

## Where work lives

| Concern | Location |
|---|---|
| Behavioural truth | [`spec.md`](spec/index.md) (§1–§18; live work queue in §13) |
| Domain models | `src/models/` |
| REST handlers | `src/api/rest/handlers.rs` |
| Database access | `src/db/` |
| Search index | `src/search/` |
| Matcher adapter | `src/matching/` (thin wrapper over [`course-matcher`](../course-matcher-rust-crate/)) |
| Validation | `src/validation/` (T-5, FR-21..FR-28) |
| Privacy | `src/privacy/` (T-10, mask + GDPR Article-15 export) |
| Audit log | `src/db/audit.rs` (T-9) |
| Event streaming | `src/streaming/` (T-9, in-memory MVP; Fluvio under flag deferred) |
| Bridge tests | [`tests/duplicate_detection.rs`](tests/duplicate_detection.rs) (T-11) |
| Integration tests | [`tests/api_integration_test.rs`](tests/api_integration_test.rs) (T-12, `#[ignore]`-tagged) |
| Benchmarks | `benches/` (T-13, three criterion files) |
| OpenAPI | served at `/swagger-ui` + `/api-docs/openapi.json` (T-14) |
| Migrations | `migrations/` |
