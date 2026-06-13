# AGENTS directory — Case Entity

Entity-level reference documentation for the case trio: service crate +
matcher crate + front-end.

## Documents in this directory

| Document | Description |
|---|---|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at entity level — authority model, three-part PRs, section mapping |
| [subprojects.md](subprojects.md) | The trio — responsibilities, dependency direction, how to run each |
| [models.md](models.md) | Domain model reference — the `Case` DTO and its persistence |
| [matching.md](matching.md) | Matching algorithm reference — weights, components, deterministic rules |
| [restful.md](restful.md) | REST API + front-end consumption + matcher library API |
| [testing.md](testing.md) | Testing strategy across the trio |

## Subproject docs

| Subproject | Spec | Agent guide | Detailed guides |
|---|---|---|---|
| [case-service-rust-crate](../case-service-rust-crate/) | [spec/index.md](../case-service-rust-crate/spec/index.md) | [AGENTS.md](../case-service-rust-crate/AGENTS.md) | — (thin; see entity spec §13 T-13) |
| [case-matcher-rust-crate](../case-matcher-rust-crate/) | [spec/index.md](../case-matcher-rust-crate/spec/index.md) | [AGENTS.md](../case-matcher-rust-crate/AGENTS.md) | [AGENTS/](../case-matcher-rust-crate/AGENTS/index.md) (algorithm, normalization, SDD, testing) |
| [case-front-end-with-svelte](../case-front-end-with-svelte/) | [spec/index.md](../case-front-end-with-svelte/spec/index.md) | [AGENTS.md](../case-front-end-with-svelte/AGENTS.md) | — (thin) |

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
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent (priority for this entity — §12) |
| [auditability.md](../../agents/share/auditability.md) | Audit logging and event streaming |
| [availability.md](../../agents/share/availability.md) | Health checks, scaling |
| [observability.md](../../agents/share/observability.md) | Tracing + OpenTelemetry |
| [restful.md](../../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](../../agents/share/locales.md) | i18n & l10n |
| [compliance-for-technology.md](../../agents/share/compliance-for-technology.md) | ISO, GDPR, … |
| [compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md) | HIPAA, NHS, GDPR (health/social-care cases) |

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living spec
  (source of truth for the cross-subproject contract)
- [`../../AGENTS.md`](../../AGENTS.md) — repo-root agent entry point
- [`../../care-pathway/spec/index.md`](../../care-pathway/spec/index.md)
  — the closest sibling entity spec, useful as a parity reference
