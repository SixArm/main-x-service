# AGENTS directory — Course Entity

Entity-level reference documentation for the **course** trio:
[course-service-with-loco](../course-service-with-loco/),
[course-matcher-rust-crate](../course-matcher-rust-crate/),
[course-front-end-with-svelte](../course-front-end-with-svelte/).
These docs orient an agent across the trio and point down to the
per-subproject AGENTS docs for detail.

## Documents in this directory

| Document | Description |
|---|---|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at entity level — authority model, three-part PRs, section mapping, anti-patterns |
| [subprojects.md](subprojects.md) | The trio: responsibilities, dependency direction, how to run each, where each subproject's docs live |
| [models.md](models.md) | Entity-level domain-model orientation (`Course`, `CourseInstance`, the three representations, the adapter) |
| [matching.md](matching.md) | Matching orientation — weights, deterministic short-circuits, where the canonical algorithm lives |
| [restful.md](restful.md) | REST + front-end route orientation, wire-contract rules |
| [testing.md](testing.md) | Test layers across the trio; the bridge test as the composition pin |

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living
  specification (source of truth for the cross-subproject contract)
- Subproject AGENTS sets:
  [`../course-service-with-loco/agents/`](../course-service-with-loco/agents/index.md),
  [`../course-matcher-rust-crate/agents/`](../course-matcher-rust-crate/agents/index.md),
  [`../course-front-end-with-svelte/agents/`](../course-front-end-with-svelte/agents/index.md)
- Subproject specs:
  [service](../course-service-with-loco/spec/index.md) (§1–§18),
  [matcher](../course-matcher-rust-crate/spec/index.md) (§1–§25),
  [front-end](../course-front-end-with-svelte/spec/index.md) (§1–§18)
- Shared project-root docs:
  [`../../agents/share/index.md`](../../agents/share/index.md)
- Project-root agent guide: [`../../AGENTS.md`](../../AGENTS.md)
