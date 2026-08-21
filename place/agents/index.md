# AGENTS directory — Place entity

Entity-level reference documentation for the **place** trio:
[place-service-with-loco](../place-service-with-loco/),
[place-matcher-rust-crate](../place-matcher-rust-crate/),
[place-front-end-with-svelte](../place-front-end-with-svelte/).

These docs orient an agent across the trio and point down to the
per-subproject docs — they do not replace them.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at entity level — which spec wins where, three-part PRs across subprojects |
| [subprojects.md](subprojects.md) | The trio: responsibilities, dependency direction, how to run each, where their docs live |
| [models.md](models.md) | The three representations of `Place` and the adapter contract |
| [matching.md](matching.md) | Matching at entity level — canonical matcher vs in-service scorer, weights, thresholds |
| [restful.md](restful.md) | REST surface + front-end consumption summary |
| [testing.md](testing.md) | Test pyramids per subproject + the contract seams |

## Per-subproject AGENTS docs

| Subproject | Entry point | Topic docs |
|---|---|---|
| place-service | [agents/index.md](../place-service-with-loco/agents/index.md) | [models](../place-service-with-loco/agents/models.md) · [matching](../place-service-with-loco/agents/matching.md) · [restful](../place-service-with-loco/agents/restful.md) · [testing](../place-service-with-loco/agents/testing.md) · [SDD](../place-service-with-loco/agents/spec-driven-development.md) |
| place-matcher | [AGENTS.md](../place-matcher-rust-crate/AGENTS.md) | [architecture](../place-matcher-rust-crate/agents/architecture.md) · [matching-algorithm](../place-matcher-rust-crate/agents/matching-algorithm.md) · [normalization](../place-matcher-rust-crate/agents/normalization.md) · [testing](../place-matcher-rust-crate/agents/testing.md) · [security-and-privacy](../place-matcher-rust-crate/agents/security-and-privacy.md) · [SDD](../place-matcher-rust-crate/agents/spec-driven-development.md) |
| place-front-end | [AGENTS.md](../place-front-end-with-svelte/AGENTS.md) | [index](../place-front-end-with-svelte/agents/index.md) · [testing](../place-front-end-with-svelte/agents/testing.md) · [SDD](../place-front-end-with-svelte/agents/spec-driven-development.md) |

## Shared documents (project root)

The shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/) — see
[`../../agents/share/index.md`](../../agents/share/index.md) for the
full table. Most relevant to this entity:
[overview](../../agents/share/overview.md) ·
[architecture](../../agents/share/architecture.md) ·
[match-search-merge](../../agents/share/match-search-merge.md) ·
[postgresql](../../agents/share/postgresql.md) (PostGIS) ·
[locales](../../agents/share/locales.md) ·
[privacy](../../agents/share/privacy.md) ·
[auditability](../../agents/share/auditability.md) ·
[compliance-for-technology](../../agents/share/compliance-for-technology.md).

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living spec
  (source of truth for the cross-subproject contract)
- [`../../AGENTS.md`](../../AGENTS.md) — repo-root agent guide
