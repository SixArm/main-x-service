# AGENTS directory — Portfolio Entity

Entity-level reference documentation for the portfolio trio:
service crate + matcher crate + front-end.

## Documents in this directory

| Document | Description |
|---|---|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at entity level — authority model, three-part PRs, section mapping |
| [subprojects.md](subprojects.md) | The trio — responsibilities, dependency direction, how to run each |
| [models.md](models.md) | Domain model reference — the `WorkItem` DTO, its four kinds, sub-resources and derived views |
| [matching.md](matching.md) | Matching algorithm reference — the kind gate, weights, components, deterministic rules |
| [restful.md](restful.md) | REST API (4 collections) + front-end consumption + matcher library API |
| [testing.md](testing.md) | Testing strategy across the trio |

## Subproject docs

| Subproject | Spec | Agent guide | Detailed guides |
|---|---|---|---|
| [project-portfolio-management-service-with-loco](../project-portfolio-management-service-with-loco/) | [spec/index.md](../project-portfolio-management-service-with-loco/spec/index.md) | [AGENTS.md](../project-portfolio-management-service-with-loco/AGENTS.md) | — (thin; see entity spec §13 T-1) |
| [project-portfolio-management-matcher-rust-crate](../project-portfolio-management-matcher-rust-crate/) | [spec/index.md](../project-portfolio-management-matcher-rust-crate/spec/index.md) | [AGENTS.md](../project-portfolio-management-matcher-rust-crate/AGENTS.md) | [AGENTS/](../project-portfolio-management-matcher-rust-crate/AGENTS/index.md) (algorithm, normalization, SDD, testing) |
| [project-portfolio-management-front-end-with-svelte](../project-portfolio-management-front-end-with-svelte/) | [spec/index.md](../project-portfolio-management-front-end-with-svelte/spec/index.md) | [AGENTS.md](../project-portfolio-management-front-end-with-svelte/AGENTS.md) | — (thin) |

## Shared documents (project root)

The shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/).

| Document | Description |
|---|---|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [rust-loco-stack.md](../../agents/share/rust-loco-stack.md) | Dependency stack |
| [loco.md](../../agents/share/loco.md) | Loco framework conventions |
| [match-search-merge.md](../../agents/share/match-search-merge.md) | Match / search / merge workflows |
| [dataflow.md](../../agents/share/dataflow.md) | Create / search / merge data flows |
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent |
| [auditability.md](../../agents/share/auditability.md) | Audit logging and event streaming |
| [availability.md](../../agents/share/availability.md) | Health checks, scaling |
| [observability.md](../../agents/share/observability.md) | Tracing + OpenTelemetry |
| [restful.md](../../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](../../agents/share/locales.md) | i18n & l10n |
| [compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md) | HIPAA, NHS, GDPR, … |
| [compliance-for-technology.md](../../agents/share/compliance-for-technology.md) | ISO, GDPR, … |

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living spec
  (source of truth for the cross-subproject contract)
- [`../../AGENTS.md`](../../AGENTS.md) — repo-root agent entry point
- [`../../care-pathway/spec/index.md`](../../care-pathway/spec/index.md) —
  the closest-shape sibling entity spec (DTO = matcher type as JSONB,
  no adapter), useful as a parity reference
