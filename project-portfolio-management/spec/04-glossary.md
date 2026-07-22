## 4. Glossary

Entity-level terms. Per-subproject vocabularies: service
[spec §4](../project-portfolio-management-service-with-loco/spec/index.md), matcher
[spec §3](../project-portfolio-management-matcher-rust-crate/spec/index.md), front-end
[spec §4](../project-portfolio-management-front-end-with-svelte/spec/index.md).

| Term | Meaning |
|---|---|
| **Entity** | One domain concept (here: Portfolio) delivered as a trio of subprojects in one directory |
| **Trio** | The three subprojects: service crate, matcher crate, front-end project |
| **Entity-level spec** | This document set — source of truth for the cross-subproject contract and the canonical §5 domain model |
| **Crate spec** | A subproject's own `spec/` — source of truth for that subproject's internals |
| **Plan** | The canonical matchable type — the unit of work an organisation funds and reports on; it may carry an optional `kind` label (Portfolio, Project, Product, Program, Practice, Process, Purpose, Pathway, or Proposal) |
| **PlanKind** | The **optional** descriptive label on a `Plan`: `Portfolio`, `Project`, `Product`, `Program`, `Practice`, `Process`, `Purpose`, `Pathway`, `Proposal`. Used for description / display / grouping — **not** a discriminator, does **not** gate matching or fix a collection |
| **Collection** | The single REST collection `/api/plans` and its `plans` service table; every plan lives here regardless of its `kind` label |
| **Parent / child plan** | Any plan may **contain** any other plan via `parent_ref` (a recursive tree); the container is the **parent**, the contained plan a **child** |
| **Thin matchable record** | The `Plan` matcher type — the identity-bearing fields that feed deduplication; distinct from the operational sub-resources |
| **Operational sub-resource** | High-volume child data of a plan (task, issue; and goal) — lives in its own service table, keyed by `parent_pid` |
| **Goal** | A plan objective `{ title, description?, target_date?, status? }`; the one sub-resource **also** carried in the matcher payload (goal titles are a match signal) |
| **Task** | A unit of work under a plan `{ title, assignee_ref, status, estimate, remaining, due_date, … }`; operational only |
| **Issue** | A bug / risk / blocker / question / improvement raised under a plan; operational only |
| **Derived view** | A computed projection — **timeline** (Gantt from goal milestones + task date ranges) or **burndown** (remaining-vs-estimate over time) |
| **PlanStatus** | `Proposed`, `Active`, `OnHold`, `Completed`, `Cancelled`, `Custom(String)` — informational-only, not a match signal |
| **Owner-scoped code** | `code` (e.g. `PROJ-2026`) — unique only within the owning `owner_org_id`; never matched across owners |
| **Parent plan** | `parent_ref` — the parent plan's `pid`; an exact supporting match signal (§5.5); absent for a root plan |
| **EntityRef** | An opaque URN naming a record in another service: `<entity_type>:<id>` (e.g. `person:0c4f…`, `organization:9a2f…`) — the one shared cross-service contract ([cross-service-linking.md §3](../../agents/share/cross-service-linking.md)) |
| **Deterministic identifier** | Globally unique identifier (URI, UUID, Jira project key, Asana GID, Trello board id, MS Project id, GitHub project id, Linear id); a shared value pins the match score to 1.0 |
| **pid** | The public UUID of a stored plan record (route param; distinct from the row's internal `id`) |
| **`data`** | The `plans.data` JSONB column holding the full thin `Plan` payload verbatim |
| **DTO contract** | The API body for the thin record **is** `project_portfolio_management_matcher::Plan` — no separate service model, no adapter |
| **Match** | A comparison between two plans yielding a 0.00–1.00 score, `Confidence` band, `is_match`, and per-component breakdown |
| **Containment cycle** | A `parent_ref` that points a plan at itself or at one of its descendants; rejected `422` (§6.1) |
| **Check-duplicates** | `POST …/check-duplicates` — match a query against stored plans, return ranked hits above threshold |
| **Tag** | A short operator-applied label for grouping / workflow (e.g. `priority-1`, `q3-review`); a **supporting** match signal via set Jaccard; distinct from keywords |
| **Keyword** | A descriptive / discovery term about *what the plan is*; distinct from operator tags |
| **Relationship** | A typed plan-to-plan link `{ relation, plan_id }` (ParentOf / DependsOn / Supersedes / SimilarTo / RelatedTo / …) — within-entity; a supporting match signal |
| **Cross-service link** | An `entity_links` edge from a plan / goal / task / issue to **any** index entity ([cross-service-linking.md](../../agents/share/cross-service-linking.md)) — **never** a match signal |
| **Soft delete** | Retention with `deleted_at` set; never `DELETE FROM` |
| **Stable key** | The bulk-import upsert key — a deterministic external PM id, the owner-scoped `(owner_org_id, code)`, or `pid` ([bulk-import-export.md §6, §10](../../agents/share/bulk-import-export.md)) |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link, cookie session + PASETO v4 public token ([authentication-sessions.md](../../agents/share/authentication-sessions.md), supersedes RS256-JWT) |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
