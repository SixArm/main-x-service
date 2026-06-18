# AGENTS directory — Worker entity

Entity-level reference documentation for the Worker trio:
[worker-service-with-loco](../worker-service-with-loco/),
[worker-matcher-rust-crate](../worker-matcher-rust-crate/),
[worker-front-end-with-svelte](../worker-front-end-with-svelte/).

These docs orient an agent at the entity level and point down into
the per-subproject docs; they do not duplicate crate detail.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at the entity level — authority model, three-part PRs across subprojects, section mapping |
| [subprojects.md](subprojects.md) | The trio: responsibilities, dependency direction, how to run each, where each subproject's docs live |
| [models.md](models.md) | The two `Worker` shapes (service vs matcher) and the adapter between them |
| [matching.md](matching.md) | The two matching layers and which doc owns which detail |
| [restful.md](restful.md) | Entity-level REST summary — service endpoints + front-end routes |
| [testing.md](testing.md) | Per-subproject suites and the contract tests at the seams |

## Per-subproject AGENTS docs

| Subproject | Entry point | Highlights |
|---|---|---|
| worker-service-with-loco | [AGENTS/index.md](../worker-service-with-loco/AGENTS/index.md) | [models.md](../worker-service-with-loco/AGENTS/models.md), [matching.md](../worker-service-with-loco/AGENTS/matching.md), [restful.md](../worker-service-with-loco/AGENTS/restful.md), [testing.md](../worker-service-with-loco/AGENTS/testing.md) |
| worker-matcher-rust-crate | [AGENTS.md](../worker-matcher-rust-crate/AGENTS.md) | [matching-algorithm.md](../worker-matcher-rust-crate/AGENTS/matching-algorithm.md), [normalization.md](../worker-matcher-rust-crate/AGENTS/normalization.md), [national-person-identifiers.md](../worker-matcher-rust-crate/AGENTS/national-person-identifiers.md), [security-and-privacy.md](../worker-matcher-rust-crate/AGENTS/security-and-privacy.md) |
| worker-front-end-with-svelte | [AGENTS.md](../worker-front-end-with-svelte/AGENTS.md) | Ground rules: Svelte 5 runes only, SPA mode, drift accepted |

## Shared documents (project root)

The shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/) — overview, architecture,
dataflow, match/search/merge, privacy, auditability, availability,
observability, locales, compliance, PostgreSQL, REST conventions,
Rust/Loco stack. Index:
[`agents/share/index.md`](../../agents/share/index.md).

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living spec
  (source of truth for the cross-subproject contract)
- [`../../AGENTS.md`](../../AGENTS.md) — monorepo entry point
