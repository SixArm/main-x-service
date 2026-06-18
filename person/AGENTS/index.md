# AGENTS directory — Person Entity

Entity-level reference documentation for the **person** trio:
[person-service-with-loco](../person-service-with-loco/),
[person-matcher-rust-crate](../person-matcher-rust-crate/),
[person-front-end-with-svelte](../person-front-end-with-svelte/).

These docs orient an agent at the entity level and point down into the
per-subproject AGENTS sets — they do not duplicate them.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at the entity level — authority model, two-level three-part PRs, which spec to edit |
| [subprojects.md](subprojects.md) | The trio: responsibilities, dependency direction, how to run each, where each subproject's docs live |
| [models.md](models.md) | Domain-model orientation — the three representations of `Person` and the adapter contract |
| [matching.md](matching.md) | Matching orientation — in-service stack vs embedded canonical matcher, where weights and schemes live |
| [restful.md](restful.md) | API-surface orientation — REST / FHIR / metrics summary + front-end routes |
| [testing.md](testing.md) | Test-pyramid orientation — per-subproject suites and the two integration seams |

## Per-subproject AGENTS sets

| Subproject | Entry point | Highlights |
|---|---|---|
| person-service | [AGENTS/index.md](../person-service-with-loco/AGENTS/index.md) | [models.md](../person-service-with-loco/AGENTS/models.md), [matching.md](../person-service-with-loco/AGENTS/matching.md), [restful.md](../person-service-with-loco/AGENTS/restful.md), [testing.md](../person-service-with-loco/AGENTS/testing.md), [spec-driven-development.md](../person-service-with-loco/AGENTS/spec-driven-development.md) |
| person-matcher | [AGENTS.md](../person-matcher-rust-crate/AGENTS.md) | [matching-algorithm.md](../person-matcher-rust-crate/AGENTS/matching-algorithm.md), [normalization.md](../person-matcher-rust-crate/AGENTS/normalization.md), [national-person-identifiers.md](../person-matcher-rust-crate/AGENTS/national-person-identifiers.md), [security-and-privacy.md](../person-matcher-rust-crate/AGENTS/security-and-privacy.md), [testing.md](../person-matcher-rust-crate/AGENTS/testing.md) |
| person-front-end | [AGENTS.md](../person-front-end-with-svelte/AGENTS.md) | Ground rules: Svelte 5 runes only, SPA mode, drift accepted, what lives where |

## Shared documents (project root)

The shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/) — see
[`agents/share/index.md`](../../agents/share/index.md) for the full
table (architecture, dataflow, match/search/merge, privacy,
auditability, availability, observability, restful, postgresql,
locales, compliance, stack).

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living
  specification (source of truth for the cross-subproject contract)
- [`../../AGENTS.md`](../../AGENTS.md) — monorepo entry point
