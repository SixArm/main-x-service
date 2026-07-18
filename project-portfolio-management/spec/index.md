# Portfolio Entity — Living Specification

> **Source of truth — for the cross-subproject contract.** This
> document is the canonical artefact for the **portfolio entity as a
> whole**: how the trio composes (front-end → service REST API →
> embedded matcher), the DTO contract (the API DTO **is** the
> matcher's `WorkItem` type, persisted as JSONB), shared invariants,
> the partition between the thin matchable `WorkItem` record and the
> high-volume operational sub-resources (goals, tasks, issues), the
> **four matchable kinds** (Portfolio, Project, Product, Program) as
> distinct collections, the within-kind match gate, and entity-wide
> goals. Each subproject's own `spec/` remains the single source of
> truth **for that subproject's internals**. When this spec and a
> crate spec disagree about crate internals, the crate spec wins; when
> they disagree about the integration contract, this spec wins. Open a
> task in §13 to bring the loser in line — do not silently rewrite
> either spec.
>
> **§5 is the canonical domain model.** This entity-level spec owns
> the canonical `WorkItem` domain model, the `WorkItemKind`
> discriminator, the kind gate, and the sub-resource partition; the
> matcher and service crate specs reference §5 rather than redefining
> it.
>
> **Three-part PRs.** A behavioural change is one PR: spec edit + code
> edit + test edit. See [`AGENTS/spec-driven-development.md`](../AGENTS/spec-driven-development.md).

Subproject specs:

- [project-portfolio-management-service-with-loco/spec](../project-portfolio-management-service-with-loco/spec/index.md) — registry service + project-management tool (§1–§18)
- [project-portfolio-management-matcher-rust-crate/spec](../project-portfolio-management-matcher-rust-crate/spec/index.md) — matching library (§1–§25)
- [project-portfolio-management-front-end-with-svelte/spec](../project-portfolio-management-front-end-with-svelte/spec/index.md) — operator UI (§1–§18)

For shared infrastructure (technology stack, observability,
compliance), see the project-root [`AGENTS.md`](../../AGENTS.md) and
[`agents/share/*`](../../agents/share/). For entity-level agent
reference (subproject map, models, matching, REST, testing), see
[`AGENTS/`](../AGENTS/).

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
