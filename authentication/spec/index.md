# Authentication Entity — Living Specification

> **Source of truth — scoped.** Each subproject's own `spec/` remains
> the single source of truth **for that subproject** (the
> [service spec](../authentication-service-rust-crate/spec/index.md) and
> the [front-end spec](../authentication-front-end-with-svelte/spec/index.md),
> and the [verifier spec](../authentication-verifier-rust-crate/spec/index.md)).
> This entity-level spec
> is the source of truth for the **cross-subproject contract**: the SSO
> protocol surface (magic-link flow, JWT claims, JWKS), the
> verifier-library contract peer services embed, and entity-wide goals.
> When this spec and a crate spec disagree about **crate internals**,
> the crate spec wins; about the **integration contract**, this spec
> wins. Open a task in §13 to reconcile — do not silently rewrite
> either document.
>
> **Three-part PRs.** A behavioural change is one PR: spec edit + code
> edit + test edit. See
> [`AGENTS/spec-driven-development.md`](../AGENTS/spec-driven-development.md).

The authentication entity is **different from its siblings**: there is
no matcher crate and nothing to match. Instead of a
service + matcher + front-end trio, it ships a
service + **verifier library** + front-end trio — the verifier is what
peer services embed to verify RS256 tokens offline. Where sibling
entities carry `AGENTS/matching.md`, this entity carries
[`AGENTS/verification.md`](../AGENTS/verification.md).

For shared infrastructure (technology stack, observability,
compliance), see the project-root [`AGENTS.md`](../../AGENTS.md) and
[`agents/share/*`](../../agents/share/). For entity-level reference
detail (subproject map, models, verification contract), see
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
