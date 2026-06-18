# AGENTS directory — Thing Entity

Entity-level reference documentation for the **thing** trio
(service + matcher + front-end). For the cross-subproject contract,
read [`../spec/index.md`](../spec/index.md) first.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | Entity-level SDD — authority model, which spec to edit, three-part PRs |
| [subprojects.md](subprojects.md) | The trio: responsibilities, dependency direction, how to run each |
| [models.md](models.md) | Entity-level domain-model orientation + DTO contract pointer |
| [matching.md](matching.md) | The two matching layers and how they relate |
| [restful.md](restful.md) | REST surface + front-end consumption summary |
| [testing.md](testing.md) | Per-subproject test commands + the bridge-test contract pin |

## Subproject agent docs

| Subproject | Entry point | Reference set |
|---|---|---|
| Service | [CLAUDE.md](../thing-service-with-loco/CLAUDE.md) | [AGENTS/index.md](../thing-service-with-loco/AGENTS/index.md) — models, matching, restful, testing, SDD |
| Matcher | [AGENTS.md](../thing-matcher-rust-crate/AGENTS.md) | [AGENTS/](../thing-matcher-rust-crate/AGENTS/) — architecture, matching-algorithm, normalization, testing, security-and-privacy, coding-style, release, SDD |
| Front-end | [AGENTS.md](../thing-front-end-with-svelte/AGENTS.md) | [AGENTS/](../thing-front-end-with-svelte/AGENTS/) — index, SDD, testing |

## Shared documents (project root)

The shared reference docs live at the repo root under
[`../../agents/share/`](../../agents/share/) — see
[`index.md`](../../agents/share/index.md) for the full inventory.
Most relevant to this entity:

| Document | Description |
|----------|-------------|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [match-search-merge.md](../../agents/share/match-search-merge.md) | Match / search / merge workflows |
| [dataflow.md](../../agents/share/dataflow.md) | Create / search / merge data flows |
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent |
| [auditability.md](../../agents/share/auditability.md) | Audit logging and event streaming |
| [availability.md](../../agents/share/availability.md) | Health checks, scaling |
| [locales.md](../../agents/share/locales.md) | i18n & l10n locale set |
| [compliance-for-technology.md](../../agents/share/compliance-for-technology.md) | GDPR, UK DPA, ISO 27001 / 42001 |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL setup + extensions |
| [rust-loco-stack.md](../../agents/share/rust-loco-stack.md) | Full dependency inventory |

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living spec (cross-subproject contract)
- [`../../AGENTS.md`](../../AGENTS.md) — repo-root agent guide
- [`../thing-service-schema.sql`](../thing-service-schema.sql) — entity reference schema
