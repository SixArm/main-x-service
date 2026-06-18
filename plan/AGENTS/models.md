# Domain Model Reference — Plan Entity

The plan entity is two things at once:

1. A **matchable identity** — the `Plan` header (project / product /
   programme / initiative / portfolio / epic). The matcher crate's
   `Plan` is the API DTO, the persisted JSONB payload, and the
   matching input. One shape, end to end — no adapter (mirrors
   care-pathway).
2. A **project-management tool** — the `Plan` owns operational
   sub-resources (goals, tasks, issues, posts, comments, members)
   and exposes derived read views (timeline / Gantt, burndown).

Normative definitions: entity spec [§5](../spec/05-domain-model.md)
and matcher [spec §6](../plan-matcher-rust-crate/spec/index.md).

## `Plan` (matchable identity)

**File:** [`plan-matcher-rust-crate/src/plan.rs`](../plan-matcher-rust-crate/src/plan.rs)

| Field | Type | Description |
|---|---|---|
| name | String | Plan title (required; service rejects blank) |
| alternate_names | Vec\<String\> | Aliases, former titles |
| plan_code | Option\<String\> | Owner-scoped code (e.g. `PROJ-01`) |
| owner_org_id | Option\<String\> | Sponsoring organisation id — scopes `plan_code` |
| owner_org_name | Option\<String\> | Sponsoring organisation display name |
| plan_type | Option\<PlanType\> | Project / product / programme / … |
| plan_status | Option\<PlanStatus\> | Lifecycle status of the plan |
| start_date | Option\<Date\> | Timeframe start (feeds `Timeframe` component) |
| end_date | Option\<Date\> | Timeframe end (feeds `Timeframe` component) |
| goals | Vec\<Goal\> | Plan goals (titles feed the `Goals` component) |
| keywords | Vec\<String\> | Free-text tags |
| tags | Vec\<String\> | Operator-curated labels |
| relationships | Vec\<Relationship\> | Typed links to other plans |
| identifiers | Vec\<PlanIdentifier\> | Typed external identifiers |
| same_as | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| in_language | Option\<String\> | ISO 639-1 language code |

## Supporting types (matching surface)

| Type | Variants / shape |
|---|---|
| `PlanType` | `Project`, `Product`, `Programme`, `Initiative`, `Portfolio`, `Epic`, `Custom(String)` |
| `PlanStatus` | `Proposed`, `Active`, `OnHold`, `Completed`, `Cancelled`, `Archived`, `Custom(String)` |
| `Relationship` | `{ kind: RelationKind, target: String }` (target = `pid` or URI) |
| `RelationKind` | `Parent`, `Child`, `DependsOn`, `Blocks`, `RelatedTo`, `Replaces`, `Custom(String)` |
| `PlanIdentifier` | `{ scheme: IdentifierScheme, value: String }` |
| `IdentifierScheme` | Deterministic: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId` · Owner-scoped: `PlanCode`, `LocalId` · `Custom(String)` |

Deterministic schemes pin a match to 1.0 on a shared value;
owner-scoped schemes never do (see [matching.md](matching.md)).

## Operational sub-resources

These hang off a `Plan` and make it a project-management tool. They
are **not** part of the matching surface (except goal *titles* —
[matching.md](matching.md) `Goals` component). Each is owned by its
parent plan, has its own table, and is reached under
`/api/v1/plans/{pid}/…`.

| Sub-resource | Key fields | Notes |
|---|---|---|
| `Goal` | `title`, `description`, `status: GoalStatus`, `target_date` | Titles feed the `Goals` match component |
| `Task` | `title`, `description`, `status`, `assignee_ref`, `due_date`, `estimate`, `parent_task_id` | Self-nesting for sub-tasks; feeds burndown |
| `Issue` | `title`, `description`, `severity`, `status`, `reporter_ref`, `assignee_ref` | Risks / blockers / defects |
| `Post` | `title`, `body`, `author_ref`, `created_at` | Updates / announcements thread |
| `Comment` | `body`, `author_ref`, `created_at`, `target` (task / issue / post) | Polymorphic parent |
| `Member` | `user_ref`, `role: MemberRole`, `person_ref?`, `worker_ref?` | Links a plan to authenticated users / people |

| Enum | Variants |
|---|---|
| `GoalStatus` | `NotStarted`, `InProgress`, `Achieved`, `Abandoned` |
| `TaskStatus` | `Todo`, `InProgress`, `Blocked`, `Done`, `Cancelled` |
| `MemberRole` | `Owner`, `Manager`, `Contributor`, `Viewer` |

`*_ref` fields are opaque references (auth user id, person `pid`,
worker `pid`, organization `pid`) resolved by the consuming
front-end / link aggregator — the plan service stores them verbatim.

## Derived views (read-only)

Computed from the plan + its sub-resources; never stored as
canonical state.

| View | Computed from | Endpoint |
|---|---|---|
| `Timeline` | `start_date`/`end_date` + task `due_date`/`estimate` + goal `target_date` | `GET …/{pid}/timeline` (Gantt-shaped rows) |
| `Burndown` | task `status` transitions over the timeframe | `GET …/{pid}/burndown` (remaining-work series) |

## Service persistence model

**Files:**
[`src/models/plans.rs`](../plan-service-with-loco/src/models/plans.rs),
[`migration/src/m20220101_000001_plans.rs`](../plan-service-with-loco/migration/src/m20220101_000001_plans.rs)

Core `plans` table: `id` (PK), `pid` (public UUID), `name`
(denormalised from `data.name`), `data` (JSONB `Plan`), `active`,
`deleted_at` (soft delete). Model helpers: `create`, `find_by_pid`,
`list(limit)`, `to_plan()` (deserialise), `update_data`,
`soft_delete`. Each sub-resource gets its own child table keyed by
the parent `plan_id` (`goals`, `tasks`, `issues`, `posts`,
`comments`, `members`).

## Wire DTOs (service controller)

**File:** [`src/controllers/plans.rs`](../plan-service-with-loco/src/controllers/plans.rs)

| Type | Shape | Used by |
|---|---|---|
| `PlanRef` | `{ pid, name }` | create / update / list responses |
| `MatchRequest` | `{ query: Plan, candidates: [Plan] }` | `POST …/match` |
| `ScoredRef` | `{ pid, name, score, confidence, is_match }` | `POST …/check-duplicates` |

## Front-end TypeScript mirror

**File:** [`src/lib/api/types.ts`](../plan-front-end-with-svelte/src/lib/api/types.ts)
— `Plan`, `PlanType`, `PlanStatus`, `Goal`, `Task`, `Issue`, `Post`,
`Comment`, `Member`, `Relationship`, `RelationKind`,
`IdentifierScheme`, `PlanRef`, `ScoredRef`. Hand-mirrored; MUST be
updated in the same change cycle as any matcher-type change (entity
spec §18).
