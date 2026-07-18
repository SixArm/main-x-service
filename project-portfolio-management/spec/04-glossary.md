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
| **WorkItem** | The canonical matchable type — a portfolio, project, product, or program; the unit of work an organisation funds and reports on. A Portfolio is the umbrella *kind* of work item |
| **WorkItemKind** | The required discriminator on every `WorkItem`: `Portfolio`, `Project`, `Product`, `Program`. A **closed** set — it maps to fixed tables / collections; no `Custom` arm. A hard match gate (§5.5) |
| **Kind / collection** | One of the four work-item kinds and its corresponding service table + REST collection (`portfolios`, `projects`, `products`, `programs`) |
| **Umbrella / child** | Portfolio is the **umbrella** kind; Project / Product / Program are **child** kinds that carry a `portfolio_ref` to a parent portfolio |
| **Thin matchable record** | The `WorkItem` matcher type — the identity-bearing fields that feed deduplication; distinct from the operational sub-resources |
| **Operational sub-resource** | High-volume child data of a work item (task, issue; and goal) — lives in its own service table, keyed by `(parent_kind, parent_pid)` |
| **Goal** | A work-item objective `{ title, description?, target_date?, status? }`; the one sub-resource **also** carried in the matcher payload (goal titles are a match signal) |
| **Task** | A unit of work under a work item `{ title, assignee_ref, status, estimate, remaining, due_date, … }`; operational only |
| **Issue** | A bug / risk / blocker / question / improvement raised under a work item; operational only |
| **Derived view** | A computed projection — **timeline** (Gantt from goal milestones + task date ranges) or **burndown** (remaining-vs-estimate over time) |
| **WorkItemStatus** | `Proposed`, `Active`, `OnHold`, `Completed`, `Cancelled`, `Custom(String)` — informational-only, not a match signal |
| **Owner-scoped code** | `code` (e.g. `PROJ-2026`) — unique only within the owning `owner_org_id`; never matched across owners |
| **Parent portfolio** | `portfolio_ref` — the parent portfolio's `pid` on a child kind; an exact supporting match signal for child kinds (§5.5); absent / ignored for the Portfolio kind |
| **EntityRef** | An opaque URN naming a record in another service: `<entity_type>:<id>` (e.g. `person:0c4f…`, `organization:9a2f…`) — the one shared cross-service contract ([cross-service-linking.md §3](../../agents/share/cross-service-linking.md)) |
| **Deterministic identifier** | Globally unique identifier (URI, UUID, Jira project key, Asana GID, Trello board id, MS Project id, GitHub project id, Linear id); a shared value pins the match score to 1.0 |
| **pid** | The public UUID of a stored work-item record (route param; distinct from the row's internal `id`) |
| **`data`** | The `<collection>.data` JSONB column holding the full thin `WorkItem` payload verbatim |
| **DTO contract** | The API body for the thin record **is** `project_portfolio_management_matcher::WorkItem` — no separate service model, no adapter |
| **Match** | A comparison between two work items of the same kind yielding a 0.00–1.00 score, `Confidence` band, `is_match`, and per-component breakdown |
| **Kind gate (R-GATE)** | The matcher's first rule: two work items of different `kind` never match (score 0.0) — they are distinct record types in distinct collections |
| **Check-duplicates** | `POST …/check-duplicates` — match a query against stored work items of the same kind, return ranked hits above threshold |
| **Tag** | A short operator-applied label for grouping / workflow (e.g. `priority-1`, `q3-review`); a **supporting** match signal via set Jaccard; distinct from keywords |
| **Keyword** | A descriptive / discovery term about *what the work item is*; distinct from operator tags |
| **Relationship** | A typed work-item-to-work-item link `{ relation, work_item_id }` (ParentOf / DependsOn / Supersedes / SimilarTo / RelatedTo / …) — within-entity; a supporting match signal |
| **Cross-service link** | An `entity_links` edge from a work item / goal / task / issue to **any** index entity ([cross-service-linking.md](../../agents/share/cross-service-linking.md)) — **never** a match signal |
| **Soft delete** | Retention with `deleted_at` set; never `DELETE FROM` |
| **Stable key** | The bulk-import upsert key — a deterministic external PM id, the owner-scoped `(owner_org_id, code)`, or `pid` ([bulk-import-export.md §6, §10](../../agents/share/bulk-import-export.md)) |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link, cookie session + PASETO v4 public token ([authentication-sessions.md](../../agents/share/authentication-sessions.md), supersedes RS256-JWT) |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
