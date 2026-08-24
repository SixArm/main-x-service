# Care Pathway Entity — Living Specification

> **Source of truth — for the cross-subproject contract.** This
> document is the canonical artefact for the **care-pathway entity as
> a whole**: how the trio composes (front-end → service REST API →
> embedded matcher), the DTO contract (the API DTO **is** the
> matcher's `CarePathway` type, persisted as JSONB), shared
> invariants, and entity-wide goals. Each subproject's own `spec/`
> remains the single source of truth **for that subproject's
> internals**. When this spec and a crate spec disagree about crate
> internals, the crate spec wins; when they disagree about the
> integration contract, this spec wins. Open a task in §13 to bring
> the loser in line — do not silently rewrite either spec.
>
> **Three-part PRs.** A behavioural change is one PR: spec edit + code
> edit + test edit. See [`agents/spec-driven-development.md`](../agents/spec-driven-development.md).

Subproject specs:

- [care-pathway-service-with-loco/spec](../care-pathway-service-with-loco/spec/index.md) — registry service (§1–§18, single file today)
- [care-pathway-matcher-rust-crate/spec](../care-pathway-matcher-rust-crate/spec/index.md) — matching library (§1–§25, single file today)
- [care-pathway-front-end-with-svelte/spec](../care-pathway-front-end-with-svelte/spec/index.md) — operator UI (§1–§18, single file today)

For shared infrastructure (technology stack, observability,
compliance), see the project-root [`AGENTS.md`](../../AGENTS.md) and
[`agents/share/*`](../../agents/share/). For entity-level agent
reference (subproject map, models, matching, REST, testing), see
[`agents/`](../agents/).

## Table of contents

1. [Purpose and Vision](01-purpose-and-vision.md)
2. [Scope](02-scope.md)
3. [Stakeholders and Users](03-stakeholders-and-users.md)
4. [Glossary](04-glossary.md)
5. [Domain Model](05-domain-model.md)
6. [Functional Requirements](06-functional-requirements.md)
7. [Non-Functional Requirements](07-non-functional-requirements.md)
8. [Architecture](08-architecture.md)
9. [API Surface](09-api-surface.md)
10. [Persistence](10-persistence.md)
11. [Testing Strategy](11-testing-strategy.md)
12. [Compliance](12-compliance.md)
13. [Tasks](13-tasks.md)
14. [Implementation Status](14-implementation-status.md)
15. [Roadmap](15-roadmap.md)
16. [Open Questions](16-open-questions.md)
17. [References](17-references.md)
18. [Change Control](18-change-control.md)

## Cross-cutting sections

Topics that span several numbered chapters at once and are therefore
kept whole rather than split across them:

- [Time-based analysis (TBA)](time-based-analysis.md) — the time
  dimension of the pathway: recorded segments, the pathway clock, the
  value-adding ratio, cohort percentiles against NHS access standards,
  constraint ranking, and queueing-theory flow analysis. Spans §5, §9,
  §10 and §12.
