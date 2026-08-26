## 5. Domain Model

This section is the **canonical home** of the portfolio domain model.
The matcher and service crate specs reference it rather than redefining
it. Two load-bearing ideas:

- **One recursive type, an optional label.** A single matchable type,
  `Plan`, carries an **optional** `kind: Option<PlanKind>` label
  (`Portfolio` | `Project` | `Product` | `Program` | `Practice` |
  `Process` | `Purpose` | `Pathway` | `Proposal`) used for description /
  display / grouping. Any plan may **contain** any other plan via
  `parent_ref` (a recursive tree). `kind` is **not** a discriminator: it
  does not map to a table / collection and does not gate matching —
  every plan lives in one `plans` table and matching is
  **kind-agnostic** (§5.5).
- **The partition.** The **thin matchable `Plan` record** (the matcher
  crate's `Plan` type) is the API DTO, the persisted JSONB payload, and
  the matching input — one shape end to end, **no separate service model
  and no adapter to drift** (exactly the care-pathway posture); the
  **operational sub-resources** (tasks, issues; and goals, which are
  *also* in the payload) are high-volume child data, held in **separate
  service tables** keyed by `parent_pid`, and are **never** part of the
  matcher payload (§5.6).

### 5.1 Canonical `Plan` (matcher crate) — the thin matchable record

Defined in `project-portfolio-management-matcher-rust-crate/src/plan.rs`; normative
reference: matcher
[spec §6](../project-portfolio-management-matcher-rust-crate/spec/index.md).

| Field | Type | Notes |
|---|---|---|
| `kind` | Option\<PlanKind\> | **Optional** descriptive label `Portfolio` \| `Project` \| `Product` \| `Program` \| `Practice` \| `Process` \| `Purpose` \| `Pathway` \| `Proposal`; not a discriminator, not a match gate (§5.5); defaults to `None` |
| `name` | String | Required (service rejects blank) |
| `alternate_names` | Vec\<String\> | Aliases, former titles, codenames |
| `code` | Option\<String\> | Owner-scoped code, e.g. `PROJ-2026` |
| `owner_org_id` | Option\<String\> | `EntityRef` `organization:<id>` — sponsoring / owning org (scopes `code`) |
| `owner_org_name` | Option\<String\> | Owning organisation display name (informational-only) |
| `lead_ref` | Option\<String\> | `EntityRef` `person:<id>` \| `worker:<id>` — the lead |
| `parent_ref` | Option\<String\> | Parent plan `pid` (the containment link); an exact supporting match signal (§5.5); absent for a root plan |
| `status` | Option\<PlanStatus\> | See enum below — **informational-only**, not a match signal |
| `goals` | Vec\<Goal\> | Plan objectives — **part of the payload**; goal *titles* feed matching (§5.4) |
| `start_date` | Option\<Date\> | Planned / actual start |
| `target_date` | Option\<Date\> | Planned completion / due date |
| `keywords` | Vec\<String\> | Descriptive / discovery terms (what the plan *is*) |
| `tags` | Vec\<String\> | Operator-applied labels for grouping / workflow — see below |
| `identifiers` | Vec\<PlanIdentifier\> | `{ scheme: IdentifierScheme, value: String }` |
| `same_as` | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| `in_language` | Option\<String\> | ISO 639-1 code — see [`agents/share/locales.md`](../../agents/share/locales.md) |
| `relationships` | Vec\<PlanRelationship\> | Typed plan-to-plan links — `{ relation: RelationKind, plan_id: String }` |

**`EntityRef` fields.** `owner_org_id`, `lead_ref` (and the
sub-resources' `assignee_ref`) hold an **`EntityRef` URN** —
`<entity_type>:<id>` — per
[`agents/share/cross-service-linking.md` §3](../../agents/share/cross-service-linking.md).
They are references, not matching strings: `owner_org_id` is an exact
match signal (§5.5), `lead_ref` is **not** scored (federation
boundary). They are stored as plain strings; the service does not call
the target service on the write path. `parent_ref` is a parent
plan's `pid` (an in-entity reference into the `plans`
collection), scored as an exact supporting signal (§5.5).

### 5.2 Supporting enums

- `PlanKind`: `Portfolio`, `Project`, `Product`, `Program`, `Practice`,
  `Process`, `Purpose`, `Pathway`, `Proposal`. An **optional** label on
  every `Plan` (the field is `Option<PlanKind>`, defaulting to `None`);
  it is descriptive only — it does not map to a table / collection and
  does not gate matching.
- `PlanStatus`: `Proposed`, `Active`, `OnHold`, `Completed`,
  `Cancelled`, `Custom(String)`. Informational-only — never scored.
- `GoalStatus`: `NotStarted`, `InProgress`, `Achieved`, `Missed`,
  `Custom(String)`.
- `IdentifierScheme`:
  - **deterministic** (a shared value pins the match to 1.0 — R-0,
    §5.5): `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
    `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`;
  - **owner-scoped** (never globally unique, excluded from R-0):
    `Code`, `LocalId`;
  - plus `Custom(String)`.
- `RelationKind`: `ParentOf` / `ChildOf` (**inverses** — programme /
  portfolio hierarchy), `DependsOn` / `BlockedBy` (**inverses**),
  `Supersedes` / `SupersededBy` (**inverses**), `SimilarTo`
  (**symmetric**), `RelatedTo` (**symmetric**); plus `Custom(String)`.

### 5.3 `Goal` (in the payload)

`Goal { title: String, description: Option<String>, target_date:
Option<Date>, status: Option<GoalStatus> }`.

A `Goal` is the **one** sub-resource that is also a payload field:
goals describe *what the plan is trying to achieve*, which is
identity-bearing, so `goals[]` rides in the JSONB `data` and reaches
the matcher. The matcher scores the **folded set of goal titles** by
Jaccard (§5.4 / §6.3). The service additionally exposes goals as a CRUD
sub-resource (§6.4) so they can be managed without rewriting the whole
plan; goal writes update the same `data.goals[]` array (§10.2), so
the payload and the sub-resource never diverge.

### 5.4 Relationships, keywords, tags — the supporting signals

**Relationships** — typed plan-to-plan links:
`relationships: Vec<PlanRelationship>`, each `{ relation,
plan_id }` **referencing another `Plan` in the registry**.
`relation` is a `RelationKind`:

- **`ParentOf`** / **`ChildOf`** (**inverses** — programme/portfolio
  hierarchy: A `ParentOf` B ⇔ B `ChildOf` A);
- **`DependsOn`** / **`BlockedBy`** (**inverses** — A `DependsOn` B ⇔ B
  `BlockedBy` A);
- **`Supersedes`** / **`SupersededBy`** (**inverses** — charter
  versioning: A `Supersedes` B ⇔ B `SupersededBy` A);
- **`SimilarTo`** (**symmetric**), a comparable plan;
- **`RelatedTo`** (**symmetric**), a loosely associated plan.

Relationships are a **supporting** match signal — a typed-set Jaccard
over the `(relation, plan_id)` pairs — never an identifying field
on their own. They are distinct from the `parent_ref` containment link
(§5.1): `parent_ref` is the recursive containment tree and is its own
exact signal, while `relationships[]` are arbitrary within-entity links
(including a `ParentOf` / `ChildOf` hierarchy among plans).

**Keywords** are descriptive / discovery terms about *what the plan
is*. **Tags** (`tags: Vec<String>`) are short free-text operator
labels for grouping, filtering, triage, or workflow (e.g.
`priority-1`, `q3-review`, `archived-2026`, `fast-track`). **Any
`Plan` can carry tags.** Each tag is a short, trimmed, non-empty
string; the list is unordered, de-duplicated **case-insensitively**,
and defaults to empty. Tags are distinct from keywords — keywords are
domain vocabulary about what the record *is*; tags are **user-applied
operational labels**. The two coexist; neither replaces the other.

Both `keywords` and `tags` **are** supporting match signals: they
round-trip through the JSONB payload (§5.6) and reach the matcher
unchanged, scored as plain set Jaccard over the case-insensitively
normalised sets (`score = |A ∩ B| / |A ∪ B|`), each weighted (§6.3, FR-8).
Like `relationships`, they are **supporting** signals, never
identifying on their own; each contributes `None` (does not
participate) when either side's set is empty.

### 5.5 Match input — what makes two plans "the same"

**Kind-agnostic.** Matching compares two thin `Plan` records
regardless of their optional `kind` label — two plans may match whether
or not their labels agree (there is no kind gate). The defining
signals (§6.3 / FR-8 has the weight table) are:

- **Name** (+ alternate names) — the heaviest weight.
- **Goal titles** — the second signal; what the plan sets out to do.
- **Owner-scoped `code`** — exact, but **only within the same
  `owner_org_id`**; never matched across owners.
- **`owner_org_id`** — exact `EntityRef` match (same sponsor); skipped
  if either side is unset.
- **`parent_ref`** — exact parent-plan match (same container); skipped
  if either side is unset.
- **timeframe** (`start_date` / `target_date` proximity), **keywords**,
  **relationships**, **tags** — supporting.

`status` is **informational-only** and never scored; `kind` is a
descriptive label and carries no weight — it neither gates nor scores
(the old plan-family `plan_type` weight is gone). `lead_ref` and the
sub-resource `EntityRef`s are **not** scored: "same lead" is not
sameness evidence, and cross-service references are never a match signal
([`agents/share/cross-service-linking.md` §7](../../agents/share/cross-service-linking.md)).

### 5.6 Persistence model (JSONB) and the partition

The thin record is stored verbatim in one row of the `plans` table:

| Column | Type | Purpose |
|---|---|---|
| `id` | serial PK | Internal row id |
| `pid` | UUID unique | Public id (route param) |
| `name` | string | Denormalised from the payload for cheap listing |
| `data` | JSONB | The full thin `Plan` payload (incl. optional `kind`, `goals[]`) |
| `parent_pid` | UUID null | Denormalised `data.parent_ref` for cheap roll-up of a plan's children |
| `active` | boolean (default true) | Registry flag |
| `deleted_at` | timestamptz null | Soft delete |

`Model::to_plan()` deserialises `data` into the matcher type;
`Model::create()` / `update_data()` serialise it in. The `name` column
MUST equal `data.name`, and the `parent_pid` column MUST equal
`data.parent_ref` (the model layer writes them together).

Because the matcher's `Plan` **is** the persisted payload and the
matching input (no adapter), every field — including the optional
`kind`, `goals[]`, `relationships[]`, `keywords`, `tags`, `parent_ref`
— round-trips verbatim through `data` and reaches the matcher
unchanged. There is **no lossy-drop list**; the only fields outside the
JSONB payload are the registry-plumbing columns (`id`, `pid`, `active`,
`deleted_at`) and the two denormalised projections (`name`,
`parent_pid`).

**The partition.** The operational sub-resources — **tasks, issues,
sprints, ceremonies, time entries, workflow configuration, and the phase
transition log** (§5.9) — are **not** in `data` and **never** enter the
matcher. They live in their own tables keyed by `parent_pid` (§10.1,
§10.6), because they are high-volume (a plan may have thousands of tasks
and tens of thousands of time entries) and are not identity-bearing.
`goals[]` is the sole crossover: it is in the payload **and** exposed as
a sub-resource, with goal writes mutating `data.goals[]` (§10.2) so the
two views stay consistent.

**The one exception is `phase`** (§5.9.4), which is a payload field
rather than a sub-resource: it is a single small enum describing the
plan as a whole, it is wanted on every list response, and unlike a task
it is one value per plan. It is nonetheless **not a match signal** — for
the same reason `status` is not (§5.5): two records of the same
initiative may sit in different phases, and the phase is precisely the
field most likely to differ between two systems describing one plan.

### 5.7 Front-end TypeScript types

The front-end mirrors the wire shape in `src/lib/api/types.ts`
(`Plan`, `PlanKind`, `Goal`, `PlanStatus`, `GoalStatus`,
`IdentifierScheme`, `PlanIdentifier`, `PlanRelationship`,
`RelationKind`, `PlanRef`, `ScoredRef`, plus the sub-resource types
`Task`, `Issue`, `Sprint`, `Ceremony`, `TimeEntry`, `Workflow`,
`WorkflowState`, `PhaseTransition` and the derived `Timeline` /
`Burndown` / `FlowDistribution` shapes). The
matcher type is upstream for the thin record: if a `Plan` field
changes in the matcher crate, the service inherits it automatically
(re-serialisation) and the front-end types MUST be fixed in the same
change cycle. The sub-resource types are owned by the service crate
spec (they have no matcher counterpart).

### 5.8 Shared invariants

All subprojects MUST uphold:

- `kind`, when present on a `Plan`, is one of the four label values; it
  is **optional** (`None` allowed) and does not fix a collection — every
  plan lives in the one `plans` table.
- Matching is **kind-agnostic**: two plans may match regardless of their
  optional `kind` labels (§5.5 / §6.3) — end to end, in the matcher and
  in every service endpoint.
- `name` is non-empty; the stored `name` column equals `data.name`.
- When `parent_ref` is set the denormalised `parent_pid` column equals
  it; a `parent_ref` may point at any other plan but never forms a
  containment cycle (points a plan at itself or a descendant → `422`,
  §6.1).
- The JSONB payload round-trips losslessly:
  `serde_json::from_value(to_value(w)) == w`.
- Owner-scoped codes (`code`, `Code`, `LocalId`) are never treated as
  globally unique — no cross-owner short-circuit, end to end. They
  short-circuit (R-1, §6.6) **only** within an equal, non-empty
  `owner_org_id`.
- `EntityRef` fields hold a valid `<entity_type>:<id>` URN or are
  absent; the service does not call the target service on the write path
  (optimistic, per [`agents/share/cross-service-linking.md`
  §5](../../agents/share/cross-service-linking.md)).
- A `PlanRelationship` references an **existing** `Plan`; **no plan
  relates to itself**. `ParentOf`/`ChildOf`, `DependsOn`/`BlockedBy`,
  and `Supersedes`/`SupersededBy` stay **acyclic** (no plan is its own
  ancestor / dependency / predecessor, directly or transitively) and
  **inverse-consistent** (A `ParentOf` B ⇔ B `ChildOf` A; likewise the
  other two pairs); `SimilarTo` and `RelatedTo` are **symmetric**.
- Each `tags` entry is short, trimmed, and non-empty; the list is
  de-duplicated case-insensitively and defaults to empty.
- Operational sub-resources (tasks, issues) are **never** serialised
  into `data` and **never** reach the matcher; `goals[]` is the only
  payload-and-sub-resource field, and goal writes mutate `data.goals[]`.
- A sub-resource always belongs to exactly one live (non-soft-deleted)
  plan; soft-deleting a plan hides its sub-resources from read paths.
- Cross-service links (`entity_links`) are never stored in
  `relationships` and never fed to any matcher
  ([cross-service-linking.md
  §7](../../agents/share/cross-service-linking.md)).
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown and `Confidence` band.
- Soft delete (`deleted_at`) is the only delete: the service never
  row-deletes, and the front-end never offers hard delete.

### 5.9 Operational model extensions (§1.4–§1.6)

The types behind the full-PM-suite commitment. **None of these enters
the matcher payload** (§5.6); every one lives in its own service table
(§10.6). The single exception is `phase`, a payload field that is still
not a match signal.

#### 5.9.1 Workflow configuration

```
Workflow { pid, plan_pid?, applies_to: Task|Issue, name,
           states: Vec<WorkflowState>, transitions: Vec<Transition>,
           is_default: bool }

WorkflowState { key: String, label: String,
                category: Todo|Active|Waiting|Done,
                wip_limit: Option<u32>, is_initial: bool, is_terminal: bool }

Transition { from: String, to: String }
```

`plan_pid` absent ⇒ a deployment-wide workflow; present ⇒ that plan
overrides it. Exactly one workflow per `applies_to` scope is
`is_default`.

**`category` is mandatory and is the load-bearing field.** The board,
the burndown, the timeline, and every time-based-analysis figure are
computed from what a state *means*, not from its name. A state without a
category is refused `422` at write time rather than defaulted, matching
the existing posture that an unknown status is refused and never coerced
([`time-based-analysis.md` §5.1](time-based-analysis.md)).

Further constraints, each of which prevents a board that cannot be
analysed: exactly one `is_initial` state; at least one state with
category `Done`; every `Transition` endpoint naming a declared state;
and no state deletable while a live task or issue sits in it.

The built-in vocabularies (task `Todo|InProgress|InReview|Done|Blocked`,
issue `Open|InProgress|Resolved|Closed`) become the **default
workflows**, so a plan with nothing configured behaves exactly as
before.

#### 5.9.2 Objectives and key results (the OKR engine)

> **Corrected 2026-08-25, before implementation.** This section first
> hung key results off `goals[]` via a `goal_id`. Checking the tree
> found that **`Goal` has no identifier** — it is a bare
> `{title, description?, target_date?, status?}` in the JSONB payload,
> addressable only by array position — and that **no goals sub-resource
> exists** (FR-12 is specified in §6.4 and built nowhere, §14.2). A
> key result anchored to an array index would be orphaned by any
> reordering.
>
> The service already has the right anchor: an **`objectives`** table
> with a `pid`, a `period` (the OKR cycle), and weighted plan alignment
> through `objective_links`. That is the O in OKR, and it is where key
> results belong.

**The split that keeps matching intact:**

- **`goals[]` in the payload** stays the plan's own objective list. Its
  *titles* are identity-bearing and feed the matcher (§5.3). Unchanged.
- **`objectives` + `objective_links`** are the organisation-level OKR
  structure: an objective exists once, and aligns to many plans with a
  **weight**.
- **Key results and check-ins** hang off an objective, live in their own
  tables, and **never reach the matcher**.

```
KeyResult { pid, objective_pid, title,
            metric: Number|Percent|Currency|Boolean,
            start_value, target_value, current_value,
            direction: Increase|Decrease|Maintain,
            unit?, currency?, owner_ref?, due_date?, tolerance? }

CheckIn { pid, key_result_pid, observed_at, value,
          confidence: Option<u8>  /* 0-100 */, note?, actor }
```

Derived, never stored (so it cannot drift from its evidence — the same
rule as Smart Score, §6.4a):

- **Key-result progress** = `(current − start) / (target − start)`,
  clamped to `[0, 1]`, with `Decrease` inverting the sign and
  `Maintain` scoring 1.0 while inside a declared tolerance band.
- **Objective score** = the mean of its key results' progress.
- **Plan score** = its aligned objectives' scores, **weighted by
  `objective_links.weight`** — the alignment weight that already
  exists, rather than a second notion of importance.

Four rules the engine must hold, and the reasoning for each:

- **A key result without a metric is not a key result.** An objective
  with no measurable key result scores `null` / `unmeasured` and sorts
  last — never `0`, which would read as "measured and failing"
  (matching the absent-evidence rule in §6.4a).
- **`start_value` is captured at creation and never recomputed**,
  because progress measured from a moving baseline is not progress.
- **Confidence is recorded but never blended into the score.** A
  self-reported number and a measured one are different kinds of thing,
  and averaging them makes the measured half unfalsifiable.
- **Alignment is the existing `objective_links` weight and the
  `parent_ref` tree**, not a second OKR hierarchy that could disagree
  with them.

Currency-valued key results are compared **only within a single
currency** — no FX conversion, the same restriction ROI already carries.

#### 5.9.3 Sprints, ceremonies, and recorded effort

```
Sprint { pid, plan_pid, name, starts_on, ends_on, goal?,
         status: Planned|Active|Closed }

Ceremony { pid, sprint_pid,
           kind: Planning|Daily|Review|Retrospective,
           held_at, facilitator_ref?, notes: Vec<CeremonyNote> }

CeremonyNote { category: WentWell|Improve|Action|Feedback
                        | Committed|Accepted|Blocker,
               text, converted_task_pid? }

TimeEntry { pid, plan_pid, task_pid?, actor_ref, spent_on: Date,
            minutes: u32, category: Capex|Opex|Unclassified,
            billable: bool, note? }
```

`Planning` writes a **commitment snapshot** — the set of task `pid`s
committed at sprint start — so a later scope change reads as a change
rather than as a moved goalpost. `Daily` blockers become `Issue`s
(FR-14) rather than a second parallel store.

**`TimeEntry` is an assertion, not an observation**, and is labelled as
such wherever it is reported. Effort is entered by a person; the task
transition log is a by-product of the work. Consequences that are
requirements, not preferences (§1.4.3):

- Recorded effort is **never** substituted for elapsed time in a flow
  ratio.
- Per-`actor_ref` roll-ups serve capacity, cost, and "who should be
  asked about this", and feed **per-person utilisation** (FR-35) under
  the obligations in
  [time-based-analysis.md](time-based-analysis.md) §12.4a. They are
  **never** an input to Smart Score or to any Flow Framework metric, and
  per-assignee cycle time / throughput / flow efficiency remain
  refused.

#### 5.9.4 Project phase

```
PlanPhase = Initiating | Planning | Executing | Controlling | Closing

PhaseTransition { pid, plan_pid, from: Option<PlanPhase>, to: PlanPhase,
                  occurred_at, actor?, reason? }
```

`phase: Option<PlanPhase>` is a **payload field** (§5.6) — one small
enum per plan, wanted on every list response. It is **informational-only
and never scored**, exactly like `status` (§5.5).

The transition log is append-only and is what makes per-phase *duration*
measurable rather than only the current value. Rules in §1.5:
one-step-at-a-time advancement (a skip is `422`), backward moves allowed
but explicitly recorded with a reason, every phase reported even at
zero, and **phase never gates an operational write**.

`PlanPhase` is a **third ordered vocabulary**, disjoint from the
lifecycle funnel (`idea` … `closed`) and the gate stage (`g0` … `g5`).
The three are uncoupled by design (§1.5.1); no cross-vocabulary
constraint is enforced.

#### 5.9.5 Work-type classification (Flow Distribution)

Flow Distribution (§1.6) needs each completed item classified into one
of the Flow Framework's four types.

> **Corrected 2026-08-25, before implementation.** This section first
> claimed the classification could be *derived* from records already
> held — feature from a task's `goal_id`, defect and risk from an
> issue's `kind`. Checking the tree rather than the spec showed neither
> exists: `tasks` has no `goal_id` (objectives link to **plans**, via
> `objective_links`), and **there is no `issues` table at all** — FR-14
> is specified across §6, §9 and §10 but was never built (§14.2). The
> derivation would have quietly classified everything as
> `unclassified`, which is the failure this entity's own honesty rules
> exist to prevent.

**The type is declared, not inferred.** A task carries an explicit
`flow_type`, which is also what the Flow Framework itself assumes — it
classifies *work items* by type rather than reconstructing type from
structure.

| Flow type | Source |
|---|---|
| `feature` | A task declared `feature` — work that adds value a customer asked for |
| `defect` | A task declared `defect` |
| `risk` | A task declared `risk`, **plus** risk-register rows categorised `compliance` or `security` |
| `debt` | A task declared `debt`, **plus** risk-register rows categorised `tech_debt` |

The risk register genuinely does carry a usable category
(`delivery` / `tech_debt` / `compliance` / `security` / `other`), so
those rows contribute without a new field.

**A task with no declared `flow_type` is `unclassified` and counted
separately** — never folded into `feature`. An unclassified pile is a
finding about the board, and absorbing it into the largest category
would silently flatter the feature share, which is the one number a
reader is most likely to act on.

**When the issues sub-resource (FR-14) is built**, its `kind` maps
directly — `Bug` → `defect`, `Risk` → `risk` — and joins this
classification without changing it. The design does not depend on it.

An optional per-deployment **intended mix** may be declared, and the gap
is then shown against it. Absent a declared intent the service reports
the mix and says nothing about whether it is right: an unlabelled target
is how a measurement becomes a quota (§1.6).

#### 5.9.6 Value realization and strategic performance

The types behind the realized-gains and strategic-performance metrics
(§6.4c). **Much of the substrate already exists** — `benefits`
(category, metric, baseline, target, expected realization date, recorded
actuals), `budget_lines` (capex/opex), `risks` (probability × impact),
`allocations`, and `strategy::roi_basis_points` all landed 2026-07-18
(PPM-10/11/12). What is new is the **measurement layer over them**, plus
three genuinely absent inputs: adoption, business-case targets, and
stakeholder sentiment.

```
BusinessCaseTarget { pid, plan_pid, metric, baseline_value,
                     target_value, unit?, currency?,
                     promised_by: Date, source: Charter|GateReview,
                     approved_at, approved_by_ref? }

ValuePoint { pid, plan_pid, benefit_pid?, observed_at, value,
             is_first_measurable: bool, method: Measured|Estimated|Asserted,
             evidence_ref?, actor }

AdoptionSnapshot { pid, plan_pid, observed_at,
                   active_users: u64, target_users: u64,
                   window_days: u16, definition: String }

SatisfactionResponse { pid, plan_pid, surveyed_at,
                       instrument: Nps|Csat, score: u8,
                       respondent_role: Sponsor|User|Team|Other,
                       comment? }
```

Four decisions worth not re-litigating:

- **`approved_at` on the business case is the clock start**, captured
  once at approval. Time to Value measured from a date that moves is not
  a measurement — the same rule as an OKR baseline (§5.9.2).
- **`method` is recorded on every value point.** A measured £2m and an
  asserted £2m are different kinds of evidence, and a realized-value
  figure that cannot say which it is has no audit standing. Roll-ups
  report the mix.
- **`AdoptionSnapshot` carries its own `definition` and `window_days`.**
  "Active user" is the most quietly redefinable term in this whole
  section; a rate whose denominator and activity window are not stored
  alongside it cannot be compared across two quarters, let alone two
  departments.
- **`SatisfactionResponse` carries a role, not an identity.** It is
  sentiment about a plan, not a record about a person. Where a
  deployment needs to prevent double submission it does so with a
  per-survey token, not by storing who said what.

**Available capacity** (needed by utilization, FR-35) comes from the
existing `allocations` rows (`person_ref`, `percent`, date range)
combined with a working-time configuration and recorded non-working
time:

```
WorkingTimeConfig { pid, scope_ref?, hours_per_day, working_days,
                    holidays: Vec<Date> }

NonWorkingPeriod { pid, person_ref, starts_on, ends_on,
                   kind: Leave|Holiday|NonProjectDuty, note? }
```

`NonWorkingPeriod` **subtracts from the denominator**; it does not count
as idle capacity. That distinction is the difference between "this
person was on leave" and "this person was underused", and a utilisation
figure that cannot tell them apart is not reportable
([time-based-analysis.md](time-based-analysis.md) §12.4a).

**Earned value** (needed by SPI / CPI, §6.4c) is derived, not stored:
planned value from the `budget_lines` phased spend, earned value from
completed scope against that budget, actual cost from recorded costs and
— where time tracking is configured (§5.9.3) — recorded effort. A plan
without a phased budget baseline reports SPI / CPI as `null` with the
reason, never as `1.0`.

#### 5.9.7 Total Project Control (TPC)

Stephen Devaux's *Total Project Control* metrics, per
[total-project-control/index.md](total-project-control/index.md). They
answer a question the other financial metrics do not: **is the value
still to come worth the money still to spend?** Earned value (SPI / CPI,
§6.4c) looks backwards at conformance to a baseline; DIPP looks forward
at whether continuing is rational.

```
TpcRecord { pid, plan_pid, currency, observed_at,
            dipp,                                  -- Devaux's Index of Project Performance
            dipp_progress_index_numerator,         -- actual DIPP at this point
            dipp_progress_index_denominator,       -- baseline DIPP at this point
            dipp_progress_index_ratio,             -- GENERATED: numerator / denominator
            expected_monetary_value,               -- EMV
            cost_estimate_to_complete }            -- CEC
```

**DIPP = EMV ÷ CEC.** Above 1.0 the remaining value exceeds the
remaining cost and continuing is rational; below 1.0 it is not, whatever
has already been spent. Devaux's point is that this is a **triage**
figure — scarce resources go to the highest DIPP — and that sunk cost
does not appear in it anywhere, by construction.

**DIPP Progress Index = actual DIPP ÷ baseline DIPP** at the same point
in the schedule. At or above 1.0 the project is tracking its own plan.
This is the one field the dictionary makes a Postgres `GENERATED ALWAYS`
column, so the ratio cannot disagree with the numerator and denominator
beside it.

Four modelling decisions:

- **Money is stored in minor units as an integer**, matching
  `strategy::roi_basis_points` and every other money path in the crate,
  and ratios are returned in **basis points**. No float touches a
  currency figure.
- **`dipp` is stored, not generated from EMV ÷ CEC**, because the full
  TPC form carries time-value terms — an acceleration premium or a delay
  cost — that EMV alone does not. The service therefore also computes
  EMV ÷ CEC and **reports the divergence** rather than silently
  preferring one: a stored DIPP that disagrees with its own inputs is a
  finding, not something to overwrite.
- **A negative EMV is legitimate and is not clamped.** A project can be
  worth less than nothing to finish, and that is precisely the case the
  metric exists to expose. This differs from ROI, where a non-positive
  *cost* is undefined.
- **CEC of zero yields `null` with a reason, never infinity.** Nothing
  left to spend is the end of the project, not an infinitely good one.

Every TPC figure is an **estimate asserted by a person**, like a
business-case target and unlike a transition timestamp, and is labelled
as such wherever it is reported (§5.9.6).

#### 5.9.8 Controls — the Controlling process

The management-control model behind §1.5's **Controlling** phase, which
until now was a phase name with no mechanism under it. A control is the
loop *set a standard → measure → compare → act*, and this section makes
each of those four a record rather than a habit.

```
Control { pid, plan_pid, name,
          timing: Feedforward|Concurrent|Feedback,
          standard: ControlStandard,
          source: ControlSource,
          cadence: Option<Cadence>,
          owner_ref?, enabled: bool }

ControlStandard { metric, target_value, unit?, currency?,
                  comparator: AtLeast|AtMost|Within|Equals,
                  tolerance? }

ControlSource = Metric(String)      -- an existing derived figure, by name
              | Query(String)       -- a named stored query
              | Manual              -- an operator reading

ControlReading { pid, control_pid, observed_at, value,
                 verdict: Pass|Fail|Unmeasured, gap?, method }

ControlAction { pid, reading_pid, kind: Correct|Adjust|Retrain|Accept|Escalate,
                description, owner_ref?, due_date?,
                converted_task_pid?, converted_issue_pid?,
                closed_at?, outcome? }
```

##### The three timings, and why the distinction is load-bearing

| Timing | Acts | Example here |
|---|---|---|
| **Feedforward** | **Before** the work, to stop a problem occurring | A gate-review readiness check (FR-16f); a workflow refusing an uncategorised state (§5.9.1); a `parent_ref` cycle rejected at write |
| **Concurrent** | **During** the work, while it can still be steered | A WIP limit refusing a move; an SLE breach firing an automation (FR-32); aging-WIP surfacing an item before it is late |
| **Feedback** | **After**, to inform the next cycle | A retrospective (§5.9.3); a gate review's benefits check; the variance and value-realization views (§6.4c) |

The distinction is not taxonomy for its own sake: **it determines what a
failing control may do.** A feedforward control may *block* — refusing a
write is the whole point of acting before the fact. A concurrent control
may *warn and escalate* but must not silently undo the operator's
action, exactly as the automation engine already guarantees (FR-16c). A
feedback control may only *record*, because the work it judges is
already finished, and a control that rewrites history is not a control.

##### Five rules

- **A control declares its source before it can pass.** A control whose
  `source` names a metric the service does not produce is refused at
  write (`422`), not left to read `Unmeasured` forever. This is the same
  posture as a workflow state without a category (§5.9.1): a check
  nobody can evaluate reads exactly like a check that passes
  ([verification habits — "a gate that never bites"](11-testing-strategy.md)).
- **`Unmeasured` is a third verdict, never a pass.** A reading with no
  value reports `Unmeasured` with a reason and is excluded from pass
  rates, rather than counted either way.
- **A failing reading requires an action or an explicit `Accept`.** A
  fail with neither is reported as **unanswered** and surfaces in the
  readiness checklist. "Fix problems" is the fourth step of the process,
  so a control that only measures is half-built.
- **Actions convert into existing records** — a `Correct` becomes a task,
  an `Escalate` becomes an issue — rather than becoming a fifth parallel
  work store.
- **Every reading and action is audited** (§6.6), and readings are
  **append-only**: a control history that can be rewritten measures
  whatever the editor wanted, the same rule the transition log carries.

##### Relationship to what already exists

Most of the *mechanisms* are built; what is missing is the **register**
that names them as controls and reports whether each one is actually
firing. Gate readiness, WIP limits, automations, the SLE, retrospectives
and the variance views are all already controls in everything but name.
Cataloguing them makes two things visible that are invisible today: a
control that has **never produced a reading**, and a phase with **no
controls at all** — which is precisely what an auditor asks for and what
§12's compliance surface has no answer to yet.
