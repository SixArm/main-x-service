# Domain Model Reference — Portfolio Entity

The portfolio entity is two things at once:

1. A **matchable identity** — the `Plan` header, with an optional
   descriptive `kind` label (Portfolio / Project / Product / Program /
   Practice / Process / Purpose / Pathway / Proposal). The matcher
   crate's `Plan` is the API DTO, the persisted JSONB payload, and the
   matching input. One shape, end to end — no adapter (mirrors
   care-pathway).
2. A **project-management tool** — a `Plan` owns operational
   sub-resources (goals, tasks, issues) and exposes derived read views
   (timeline / Gantt, burndown).

Normative definitions: entity spec [§5](../spec/05-domain-model.md)
and matcher [spec §6](../project-portfolio-management-matcher-rust-crate/spec/index.md).

## The optional kind label + recursive containment

Every record is a `Plan`. **Portfolio**, **Project**, **Product**,
**Program**, **Practice**, **Process**, **Purpose**, **Pathway**, and
**Proposal** are optional values of the descriptive `kind` label — used
for display and grouping only. `kind` is **not required**, **not a
discriminator**, does **not** fix a collection, and does **not** gate
matching ([matching.md](matching.md) — matching is kind-agnostic).

Containment is **recursive**: any plan may contain any other plan via
`parent_ref`, forming a tree (`parent_ref` replaces the former
`portfolio_ref`). The service rejects a `parent_ref` that points a plan
at itself or at one of its descendants (a containment cycle) with
HTTP `422`.

## `Plan` (matchable identity)

**File:** [`project-portfolio-management-matcher-rust-crate/src/plan.rs`](../project-portfolio-management-matcher-rust-crate/src/plan.rs)

| Field | Type | Description |
|---|---|---|
| kind | Option\<PlanKind\> | **Optional.** Portfolio / Project / Product / Program / Practice / Process / Purpose / Pathway / Proposal — a descriptive display / grouping label; not a discriminator, not a gate |
| name | String | Plan title (required; service rejects blank) |
| alternate_names | Vec\<String\> | Aliases, former titles, codenames |
| code | Option\<String\> | Owner-scoped code (e.g. `PROJ-2026`) |
| owner_org_id | Option\<String\> | EntityRef `organization:<id>` — sponsoring / owning org; scopes `code` |
| owner_org_name | Option\<String\> | Owning-org display name (informational-only) |
| lead_ref | Option\<String\> | EntityRef `person:<id>` \| `worker:<id>` — the lead. **Not scored** |
| parent_ref | Option\<String\> | Parent plan `pid` (recursive containment); absent for a root plan |
| status | Option\<PlanStatus\> | Lifecycle status — informational-only, NOT a match signal |
| goals | Vec\<Goal\> | In the payload; goal **titles** feed the `Goals` component |
| start_date | Option\<Date\> | Planned / actual start (feeds `Timeframe`) |
| target_date | Option\<Date\> | Planned completion / due date (feeds `Timeframe`) |
| keywords | Vec\<String\> | Descriptive / discovery terms (what it *is*) |
| tags | Vec\<String\> | Operator labels for grouping / workflow |
| identifiers | Vec\<PlanIdentifier\> | Typed external identifiers |
| same_as | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| in_language | Option\<String\> | ISO 639-1 language code |
| relationships | Vec\<PlanRelationship\> | Typed links to other plans |

Construct with `Plan::new(name)` — `kind` defaults to `None`.

## Supporting types (matching surface)

| Type | Variants / shape |
|---|---|
| `PlanKind` | `Portfolio`, `Project`, `Product`, `Program`, `Practice`, `Process`, `Purpose`, `Pathway`, `Proposal` — **closed set** (no `Custom`, not `#[non_exhaustive]`); an optional descriptive label |
| `PlanStatus` | `Proposed`, `Active`, `OnHold`, `Completed`, `Cancelled`, `Custom(String)` — informational-only |
| `PlanRelationship` | `{ relation: RelationKind, plan_id: String }` (`plan_id` = `pid` or URI) |
| `RelationKind` | `ParentOf` / `ChildOf` (inverses), `DependsOn` / `BlockedBy` (inverses), `Supersedes` / `SupersededBy` (inverses), `SimilarTo` (symmetric), `RelatedTo` (symmetric), `Custom(String)` |
| `PlanIdentifier` | `{ scheme: IdentifierScheme, value: String }` |
| `IdentifierScheme` | Deterministic: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId` · Owner-scoped: `Code`, `LocalId` · `Custom(String)` |

Deterministic schemes pin a match to 1.0 on a shared value;
owner-scoped schemes never do (see [matching.md](matching.md)).

## Operational sub-resources

These hang off any `Plan` and make it a project-management tool. They are
**not** part of the matching surface (except goal *titles* —
[matching.md](matching.md) `Goals` component). Each is owned by its
parent plan, has its own table, and is reached under
`/api/plans/{pid}/…`.

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
link aggregator — the plan service stores them verbatim.

> Posts / comments / members are **not** core sub-resources for the
> portfolio entity (deferred; roadmap only if a collaboration surface is
> needed). The matchable surface and the three sub-resources above are
> the whole model.

## Derived views (read-only)

Computed from the plan + its sub-resources; never stored as
canonical state.

| View | Computed from | Endpoint |
|---|---|---|
| `Timeline` | `start_date`/`target_date` + task `due_date`/`estimate` + goal `target_date` | `GET …/{pid}/timeline` (Gantt-shaped rows) |
| `Burndown` | task `status` transitions + `remaining` over the timeframe | `GET …/{pid}/burndown` (remaining-work series) |

## Service persistence model

**Files:**
[`src/models/`](../project-portfolio-management-service-with-loco/src/models/),
[`migration/src/`](../project-portfolio-management-service-with-loco/migration/src/)

One core table `plans` — `{id` (PK)`, pid` (public UUID)`, name`
(denormalised from `data.name`)`, kind` (nullable)`, parent_pid`
(nullable, the recursive containment link)`, data` (JSONB `Plan`)`,
active, deleted_at` (soft delete)`}`. Model helpers: `create`,
`find_by_pid`, `list(limit)`, `to_plan()` (deserialise), `update_data`,
`soft_delete`.

Each sub-resource gets its own child table (`tasks`, `goals`, `issues`)
keyed by the parent `plan_pid`. Supporting tables: `audit_logs`,
`merge_records`, `entity_links`, `review_queue`, plus a deferred
`bulk_jobs`.

## Wire DTOs (service controller)

**File:** [`src/controllers/`](../project-portfolio-management-service-with-loco/src/controllers/)

| Type | Shape | Used by |
|---|---|---|
| `PlanRef` | `{ pid, name }` | create / update / list responses |
| `MatchRequest` | `{ query: Plan, candidates: [Plan] }` | `POST …/match` |
| `ScoredRef` | `{ pid, name, score, confidence, is_match }` | `POST …/check-duplicates` |

## Front-end TypeScript mirror

**File:** [`src/lib/api/types.ts`](../project-portfolio-management-front-end-with-svelte/src/lib/api/types.ts)
— `Plan`, `PlanKind`, `PlanStatus`, `Goal`, `GoalStatus`,
`Task`, `TaskStatus`, `Issue`, `IssueKind`, `IssueSeverity`,
`IssueStatus`, `PlanRelationship`, `RelationKind`,
`IdentifierScheme`, `PlanRef`, `ScoredRef`. Hand-mirrored; MUST be
updated in the same change cycle as any matcher-type change (entity
spec §18).
