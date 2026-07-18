# Domain Model Reference — Portfolio Entity

The portfolio entity is two things at once:

1. A **matchable identity** — the `WorkItem` header, discriminated by a
   required `kind` (Portfolio / Project / Product / Program). The matcher
   crate's `WorkItem` is the API DTO, the persisted JSONB payload, and
   the matching input. One shape, end to end — no adapter (mirrors
   care-pathway).
2. A **project-management tool** — a `WorkItem` owns operational
   sub-resources (goals, tasks, issues) and exposes derived read views
   (timeline / Gantt, burndown).

Normative definitions: entity spec [§5](../spec/05-domain-model.md)
and matcher [spec §6](../project-portfolio-management-matcher-rust-crate/spec/index.md).

## The kind discriminator — four matchable record types

A **Portfolio** is the umbrella container; **Project**, **Product**, and
**Program** are distinct record types that sit under a portfolio. They
are modelled as the single canonical `WorkItem` type plus a required
`kind: WorkItemKind` discriminator — the "umbrella kind of work item"
naming — but each kind is a **distinct service table and REST
collection**, not a variant on one shared collection. Child kinds
(Project / Product / Program) carry a `portfolio_ref` to their parent
portfolio; Portfolio records do not. `kind` is also a **hard match gate**
([matching.md](matching.md) R-GATE): cross-kind pairs never match.

## `WorkItem` (matchable identity)

**File:** [`project-portfolio-management-matcher-rust-crate/src/work_item.rs`](../project-portfolio-management-matcher-rust-crate/src/work_item.rs)

| Field | Type | Description |
|---|---|---|
| kind | WorkItemKind | **Required.** Portfolio / Project / Product / Program — the collection / table it lives in; a hard match gate |
| name | String | Work-item title (required; service rejects blank) |
| alternate_names | Vec\<String\> | Aliases, former titles, codenames |
| code | Option\<String\> | Owner-scoped code (e.g. `PROJ-2026`) |
| owner_org_id | Option\<String\> | EntityRef `organization:<id>` — sponsoring / owning org; scopes `code` |
| owner_org_name | Option\<String\> | Owning-org display name (informational-only) |
| lead_ref | Option\<String\> | EntityRef `person:<id>` \| `worker:<id>` — the lead. **Not scored** |
| portfolio_ref | Option\<String\> | Parent portfolio `pid` for Project / Product / Program (the umbrella link); absent / ignored for Portfolio kind |
| status | Option\<WorkItemStatus\> | Lifecycle status — informational-only, NOT a match signal |
| goals | Vec\<Goal\> | In the payload; goal **titles** feed the `Goals` component |
| start_date | Option\<Date\> | Planned / actual start (feeds `Timeframe`) |
| target_date | Option\<Date\> | Planned completion / due date (feeds `Timeframe`) |
| keywords | Vec\<String\> | Descriptive / discovery terms (what it *is*) |
| tags | Vec\<String\> | Operator labels for grouping / workflow |
| identifiers | Vec\<WorkItemIdentifier\> | Typed external identifiers |
| same_as | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| in_language | Option\<String\> | ISO 639-1 language code |
| relationships | Vec\<WorkItemRelationship\> | Typed links to other work items |

## Supporting types (matching surface)

| Type | Variants / shape |
|---|---|
| `WorkItemKind` | `Portfolio`, `Project`, `Product`, `Program` — **closed set** (no `Custom`, not `#[non_exhaustive]`); maps to fixed tables / collections |
| `WorkItemStatus` | `Proposed`, `Active`, `OnHold`, `Completed`, `Cancelled`, `Custom(String)` — informational-only |
| `WorkItemRelationship` | `{ relation: RelationKind, work_item_id: String }` (`work_item_id` = `pid` or URI) |
| `RelationKind` | `ParentOf` / `ChildOf` (inverses), `DependsOn` / `BlockedBy` (inverses), `Supersedes` / `SupersededBy` (inverses), `SimilarTo` (symmetric), `RelatedTo` (symmetric), `Custom(String)` |
| `WorkItemIdentifier` | `{ scheme: IdentifierScheme, value: String }` |
| `IdentifierScheme` | Deterministic: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId` · Owner-scoped: `Code`, `LocalId` · `Custom(String)` |

Deterministic schemes pin a match to 1.0 on a shared value;
owner-scoped schemes never do (see [matching.md](matching.md)).

## Operational sub-resources

These hang off any `WorkItem` (portfolio / project / product / program)
and make it a project-management tool. They are **not** part of the
matching surface (except goal *titles* — [matching.md](matching.md)
`Goals` component). Each is owned by its parent work item, has its own
table, and is reached under `/api/{collection}/{pid}/…`.

| Sub-resource | Key fields | Notes |
|---|---|---|
| `Goal` | `title`, `description`, `target_date`, `status: GoalStatus` | Titles feed the `Goals` match component; also a payload field via `data.goals[]` (the goals bridge) |
| `Task` | `title`, `description`, `status: TaskStatus`, `assignee_ref`, `goal_id?`, `parent_task_id?`, `estimate`, `remaining`, `due_date` | Self-nesting for sub-tasks; feeds burndown |
| `Issue` | `title`, `description`, `kind: IssueKind`, `severity: IssueSeverity`, `status: IssueStatus`, `assignee_ref` | Bugs / risks / blockers / questions / improvements |

| Enum | Variants |
|---|---|
| `GoalStatus` | `NotStarted`, `InProgress`, `Achieved`, `Missed`, `Custom(String)` |
| `TaskStatus` | `Todo`, `InProgress`, `InReview`, `Done`, `Blocked` |
| `IssueKind` | `Bug`, `Risk`, `Blocker`, `Question`, `Improvement` |
| `IssueSeverity` | `Low`, `Medium`, `High`, `Critical` |
| `IssueStatus` | `Open`, `InProgress`, `Resolved`, `Closed` |

`*_ref` fields are opaque references (person `pid`, worker `pid`,
organization `pid`, auth user id) resolved by the consuming front-end /
link aggregator — the portfolio service stores them verbatim.

> Posts / comments / members are **not** core sub-resources for the
> portfolio entity (deferred; roadmap only if a collaboration surface is
> needed). The matchable surface and the three sub-resources above are
> the whole model.

## Derived views (read-only)

Computed from the work item + its sub-resources; never stored as
canonical state.

| View | Computed from | Endpoint |
|---|---|---|
| `Timeline` | `start_date`/`target_date` + task `due_date`/`estimate` + goal `target_date` | `GET …/{pid}/timeline` (Gantt-shaped rows) |
| `Burndown` | task `status` transitions + `remaining` over the timeframe | `GET …/{pid}/burndown` (remaining-work series) |

## Service persistence model

**Files:**
[`src/models/`](../project-portfolio-management-service-with-loco/src/models/),
[`migration/src/`](../project-portfolio-management-service-with-loco/migration/src/)

One core table **per work-item kind** — `portfolios`, `projects`,
`products`, `programs` — each `{id` (PK)`, pid` (public UUID)`, name`
(denormalised from `data.name`)`, data` (JSONB `WorkItem`)`, active,
deleted_at` (soft delete)`}`. The child kinds (`projects` / `products` /
`programs`) additionally carry a denormalised `portfolio_pid` column.
Model helpers per table: `create`, `find_by_pid`, `list(limit)`,
`to_work_item()` (deserialise), `update_data`, `soft_delete`.

Each sub-resource gets its own child table (`tasks`, `goals`, `issues`)
keyed by the parent `(parent_kind, parent_pid)`. Supporting tables:
`audit_logs`, `merge_records`, `entity_links`, `review_queue`, plus a
deferred `bulk_jobs`.

## Wire DTOs (service controller)

**File:** [`src/controllers/`](../project-portfolio-management-service-with-loco/src/controllers/)

| Type | Shape | Used by |
|---|---|---|
| `WorkItemRef` | `{ pid, name }` | create / update / list responses |
| `MatchRequest` | `{ query: WorkItem, candidates: [WorkItem] }` | `POST …/match` |
| `ScoredRef` | `{ pid, name, score, confidence, is_match }` | `POST …/check-duplicates` |

## Front-end TypeScript mirror

**File:** [`src/lib/api/types.ts`](../project-portfolio-management-front-end-with-svelte/src/lib/api/types.ts)
— `WorkItem`, `WorkItemKind`, `WorkItemStatus`, `Goal`, `GoalStatus`,
`Task`, `TaskStatus`, `Issue`, `IssueKind`, `IssueSeverity`,
`IssueStatus`, `WorkItemRelationship`, `RelationKind`,
`IdentifierScheme`, `WorkItemRef`, `ScoredRef`. Hand-mirrored; MUST be
updated in the same change cycle as any matcher-type change (entity
spec §18).
