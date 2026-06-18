# AGENTS directory — Organization Entity

Entity-level reference documentation for the **organization** trio:
service crate + matcher crate + front-end. Start with the entity spec,
[`../spec/index.md`](../spec/index.md) — the source of truth for the
cross-subproject contract.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at entity level — which spec governs what, three-part PRs, anti-patterns |
| [subprojects.md](subprojects.md) | The trio: responsibilities, dependency direction, how to run each, where each one's docs live |
| [models.md](models.md) | The canonical `Organization` DTO, identifier schemes, persistence row, TS mirror |
| [matching.md](matching.md) | Matching summary — weights, deterministic rules, confidence; links to the matcher's detailed guides |
| [restful.md](restful.md) | REST endpoint reference + front-end consumption map |
| [testing.md](testing.md) | Test inventory and commands per subproject; seam coverage |

## Subproject docs

| Subproject | Spec | Agent guide | Detailed guides |
|---|---|---|---|
| [organization-service-with-loco](../organization-service-with-loco/) | [spec/index.md](../organization-service-with-loco/spec/index.md) (§1–§18, single file) | [AGENTS.md](../organization-service-with-loco/AGENTS.md) | — (thin; spec §13 / entity T-1 queue the `AGENTS/` set) |
| [organization-matcher-rust-crate](../organization-matcher-rust-crate/) | [spec/index.md](../organization-matcher-rust-crate/spec/index.md) (§1–§25, single file) | [AGENTS.md](../organization-matcher-rust-crate/AGENTS.md) | [AGENTS/index.md](../organization-matcher-rust-crate/AGENTS/index.md) — matching-algorithm, normalization, SDD, testing |
| [organization-front-end-with-svelte](../organization-front-end-with-svelte/) | [spec/index.md](../organization-front-end-with-svelte/spec/index.md) (§1–§18, single file) | [AGENTS.md](../organization-front-end-with-svelte/AGENTS.md) | — (front-ends ship the thin doc set by design) |

## Shared documents (project root)

The shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/); full directory in
[`index.md`](../../agents/share/index.md).

| Document | Description |
|----------|-------------|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [match-search-merge.md](../../agents/share/match-search-merge.md) | Match / search / merge workflows (the parity target) |
| [auditability.md](../../agents/share/auditability.md) | Audit logging and event streaming |
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent |
| [availability.md](../../agents/share/availability.md) | Health checks, scaling |
| [observability.md](../../agents/share/observability.md) | Tracing + OpenTelemetry summary |
| [restful.md](../../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](../../agents/share/locales.md) | i18n & l10n |
| [rust-loco-stack.md](../../agents/share/rust-loco-stack.md) | Rust / Loco dependency stack |
| [compliance-for-technology.md](../../agents/share/compliance-for-technology.md) | ISO, GDPR, UK DPA |

## See also

- [`../spec/index.md`](../spec/index.md) — the entity-level living spec
- [Person entity spec](../../person/spec/index.md) — the entity-level exemplar
- Root [`AGENTS.md`](../../AGENTS.md) — monorepo directory
