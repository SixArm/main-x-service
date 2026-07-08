# AGENTS directory — Event Entity

Entity-level reference documentation for the **event** entity of the
Main X Index: the trio of
[event-service-with-loco](../event-service-with-loco/),
[event-matcher-rust-crate](../event-matcher-rust-crate/), and
[event-front-end-with-svelte](../event-front-end-with-svelte/).

These docs orient an agent across the trio and point down to the
per-subproject AGENTS docs — they summarise shape, not detail.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at the entity level — authority model, three-part PRs, anti-patterns |
| [subprojects.md](subprojects.md) | The trio: responsibilities, dependency direction, how to run each, where each subproject's docs live |
| [models.md](models.md) | Entity-level domain-model orientation (canonical Event, matcher DTO, front-end types) |
| [matching.md](matching.md) | The two matching surfaces and the adapter bridge between them |
| [restful.md](restful.md) | `/api` REST surface summary + front-end consumption |
| [testing.md](testing.md) | Test pyramids per subproject + the seam tests that pin the contracts |

## Entity-level spec

[`../spec/index.md`](../spec/index.md) — single source of truth for
the **cross-subproject contract** (§1–§18; live tasks in §13). Each
subproject's own `spec/` remains the source of truth for that
subproject's internals.

## Subproject AGENTS docs

| Subproject | Entry point | Highlights |
|---|---|---|
| Service | [AGENTS/index.md](../event-service-with-loco/AGENTS/index.md) | [models](../event-service-with-loco/AGENTS/models.md) · [matching](../event-service-with-loco/AGENTS/matching.md) · [restful](../event-service-with-loco/AGENTS/restful.md) · [testing](../event-service-with-loco/AGENTS/testing.md) |
| Matcher | [AGENTS.md](../event-matcher-rust-crate/AGENTS.md) | [architecture](../event-matcher-rust-crate/AGENTS/architecture.md) · [matching-algorithm](../event-matcher-rust-crate/AGENTS/matching-algorithm.md) · [normalization](../event-matcher-rust-crate/AGENTS/normalization.md) · [testing](../event-matcher-rust-crate/AGENTS/testing.md) · [security-and-privacy](../event-matcher-rust-crate/AGENTS/security-and-privacy.md) |
| Front-end | [AGENTS.md](../event-front-end-with-svelte/AGENTS.md) | [AGENTS/index.md](../event-front-end-with-svelte/AGENTS/index.md) · [testing](../event-front-end-with-svelte/AGENTS/testing.md) |

## Shared documents (project root)

The shared reference docs live at
[`../../agents/share/`](../../agents/share/) — see
[`index.md`](../../agents/share/index.md) for the full table.
Most relevant here:
[overview](../../agents/share/overview.md) ·
[architecture](../../agents/share/architecture.md) ·
[dataflow](../../agents/share/dataflow.md) ·
[match-search-merge](../../agents/share/match-search-merge.md) ·
[privacy](../../agents/share/privacy.md) ·
[auditability](../../agents/share/auditability.md) ·
[availability](../../agents/share/availability.md) ·
[locales](../../agents/share/locales.md) ·
[compliance-for-technology](../../agents/share/compliance-for-technology.md) ·
[compliance-for-healthcare](../../agents/share/compliance-for-healthcare.md)
