## 6. Functional Requirements

Each requirement names its owning subproject. The entity is
**spec-only; no code exists yet** (§14), so every requirement is a
target tracked in §13 / §15.

There is one plans collection — `/api/plans` — where `{pid}` is a plan
record. All matching, dedup, and merge endpoints are **kind-agnostic**:
any two plans may be compared (§5.5); the optional `kind` label neither
gates nor scores.

### 6.1 Registry CRUD — service (thin record)

- **FR-1** Create a plan: `POST /api/plans` with a `Plan` body (optional
  `kind` label); reject a blank `name` with `422`; return `{pid, name}`.
- **FR-1a** Validate the body (service): reject with `422` a blank
  `name`, a malformed `EntityRef` (`owner_org_id`, `lead_ref` — must
  parse as `<entity_type>:<id>`), a malformed `parent_ref` (must be a
  valid plan `pid`) or a `parent_ref` that would form a **containment
  cycle** (points a plan at itself or at one of its descendants), a
  malformed deterministic `identifiers` entry (UUID / URI / external-PM-id
  shape — rejecting a malformed *deterministic* identifier matters
  because a shared value short-circuits the match to 1.0, R-0), a
  relationship that self-references or names an unknown plan, or a
  malformed `in_language` (BCP-47). All problems are reported in one
  response.
- **FR-2** List active plans: `GET /api/plans` returns
  `{pid, name}` refs, most-recent first, capped at 100. A
  `?parent=<pid>` filter rolls up one plan's children.
- **FR-3** Read: `GET /api/plans/{pid}` returns the stored thin
  `Plan`; `404` for unknown or soft-deleted `pid`.
- **FR-4** Update: `PUT /api/plans/{pid}` replaces the whole
  thin payload (and the denormalised `name` / `parent_pid`); same
  validation as FR-1a.
- **FR-5** Soft delete: `DELETE /api/plans/{pid}` sets
  `deleted_at`; the record and its sub-resources disappear from list /
  read / match.

### 6.2 Name search — service

- **FR-5a** `GET /api/plans/search?q=` — case-insensitive name
  search (Postgres `ILIKE` on the denormalised `name`, cap 50,
  wildcards escaped); blank `q` → `400`. Tantivy full-text over the
  payload is roadmap (§15).

### 6.3 Matching — matcher (algorithm) + service (endpoints)

Algorithm reference: [`agents/matching.md`](../agents/matching.md) and
the matcher [spec §5–§18](../project-portfolio-management-matcher-rust-crate/spec/index.md).

- **FR-6 (kind-agnostic matching)** The matcher compares any two plans
  regardless of their optional `kind` label; there is **no kind gate**.
  Service endpoints feed candidate plans without kind filtering.
- **FR-7** Deterministic short-circuits (matcher): score pins to 1.0
  on —
  - **R-0**: any shared value on a **deterministic identifier scheme**
    (`Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`,
    `MsProjectId`, `GitHubProjectId`, `LinearId`); the owner-scoped
    `Code` / `LocalId` and `Custom` are **excluded**;
  - **R-1**: same non-empty `owner_org_id` + equal normalised `code`;
  - **R-2**: any case-folded `same_as` URL overlap.
- **FR-8** Probabilistic components (matcher), renormalised weighted
  average over the components both records carry:

  | Component | Weight | Algorithm |
  |---|---:|---|
  | Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names`; Soundex +0.05 bonus capped at 0.95 |
  | Goals | 0.15 | Jaccard over folded goal *titles* |
  | Code | 0.15 | Same `owner_org_id`: 1.0/0.0; differing or missing owner: skipped |
  | Owner org | 0.10 | `owner_org_id` exact `EntityRef`; skipped when either unset |
  | Parent | 0.08 | `parent_ref` exact parent-plan match; skipped when either unset |
  | Timeframe | 0.07 | Date proximity over `start_date` / `target_date`; skipped when no dates |
  | Keywords | 0.05 | Jaccard over folded sets |
  | Relationships | 0.05 | Typed-set Jaccard over `(relation, plan_id)`; skipped when either empty |
  | Tags | 0.05 | Set Jaccard over case-folded sets; skipped when either empty |

  Weights sum to 1.00. (`status` and the optional `kind` label are
  informational-only and carry no weight; the parent component replaces
  the plan-family `plan_type` weight.)
- **FR-9** Explainability (matcher): every result carries `score`,
  `Confidence` (`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`),
  `is_match` (threshold 0.85 default; `strict` 0.95 / `lenient`
  0.70), and a per-component `MatchBreakdown`.
- **FR-10** Ad-hoc ranking (service): `POST /api/plans/match`
  scores a `{query, candidates}` set without persistence, returning
  ranked `(index, MatchResult)` pairs.
- **FR-11** Duplicate check (service):
  `POST /api/plans/check-duplicates` matches a query against
  stored plans and returns hits above threshold as
  `{pid, name, score, confidence, is_match}`, sorted by score
  descending.
- **FR-11a** Real-time duplicate detection on create (service):
  `POST /api/plans` returns `409 Conflict` with candidate
  matches when a likely duplicate is detected (family baseline,
  [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md));
  a `force` flag bypasses for deliberate near-duplicates.
- **FR-11b** Record merge (service): `POST /api/plans/merge`
  folds a confirmed-duplicate plan into a surviving one (any two plans;
  no kind restriction) — union the list fields, keep the duplicate's
  name as an `alternate_names` entry, **re-home the duplicate's
  sub-resources** (tasks / issues re-keyed to the survivor's `pid`),
  soft-delete the duplicate, write a `merge_records` history row
  (snapshot of the transferred payload), and publish a `Merged` event.
  Equal `main_pid`/`duplicate_pid` → `422`; unknown pid → `404`.
  `GET /api/plans/merges/recent` lists the history.

### 6.4 Operational sub-resources — service (project-management tool)

Each is a child resource of a plan, keyed by `parent_pid`, in its own
Postgres table (§10.1). None enters the matcher payload (§5.6). All
sub-resource writes emit events and audit rows (§6.6). The sub-resources
hang off **any** plan.

- **FR-12 Goals.** CRUD goals under a plan
  (`/api/plans/{pid}/goals`); goal writes mutate `data.goals[]` on
  the parent so the matchable payload and the sub-resource stay
  consistent (§5.3, §10.2). `Goal { title, description?, target_date?,
  status }`.
- **FR-13 Tasks.** CRUD tasks under a plan
  (`/api/plans/{pid}/tasks`). `Task { pid, parent_pid, title,
  description?, assignee_ref? (EntityRef), status:
  Todo|InProgress|InReview|Done|Blocked, goal_id?, parent_task_id?,
  estimate?, remaining?, due_date? }`. A task may attach to a goal and
  nest under a parent task.
- **FR-14 Issues.** CRUD issues under a plan
  (`/api/plans/{pid}/issues`). `Issue { pid, parent_pid, title,
  kind: Bug|Risk|Blocker|Question|Improvement,
  severity: Low|Med|High|Critical, status:
  Open|InProgress|Resolved|Closed, assignee_ref? }`.
- **FR-15 Timeline (derived).** `GET /api/plans/{pid}/timeline`
  returns a Gantt-style projection: goals with a `target_date` as
  milestones + tasks with `due_date` / date ranges as bars. Read-only;
  computed, not stored.
- **FR-16 Burndown (derived).** `GET /api/plans/{pid}/burndown`
  returns remaining-vs-estimate over time, from periodic snapshots of
  task `estimate` / `remaining`. Read-only; computed from snapshots.

### 6.4a Collaboration, automation, and prioritisation — service

Delivered 2026-07-22 (service spec §9.4a). None of this enters the
matcher payload (§5.6); every mutation audits (§6.6).

- **FR-16a Collaborative review.** Delegate an idea / proposal / plan
  to one internal or external expert (`/api/reviews`), track
  accept / decline, collect a verdict (`score` 0–100 optional,
  `recommendation` advance|hold|reject), and read the consensus.
  Reviewers are `EntityRef` URNs, **never raw email** — an external
  expert is a record in the person registry — and `reviewer_scope`
  (internal / external) is recorded explicitly, because disclosure to an
  outsider is a decision, not an inference. Only a reviewer who
  **accepted** may submit, so an unanswered invitation never becomes
  evidence; consensus reports a **strict** majority only (a tie or a
  plurality reports none), a `null` mean until somebody scores, and the
  count still outstanding.
- **FR-16b Assignee management.** Assign / unassign a task
  (`…/tasks/{t_pid}/assign`; `null` unassigns) and read the open
  workload per assignee, which surfaces the **unassigned** pile rather
  than dropping it.
- **FR-16c Workflow automation.** Rules configured once
  (`/api/automations`) fire as work crosses the Kanban board (or when a
  plan review is submitted) and apply one action: assign, add label,
  notify, schedule an action, or set a task status. Action shapes are
  validated at **write** time; a failing rule is recorded as a `failed`
  run and **never** undoes the operator's move; actions are applied
  without re-entering the engine, so automations cannot cascade. Every
  firing — applied, skipped, or failed — is logged.
- **FR-16d Set and forget.** A deadline configured once
  (`/api/scheduled-actions`) is held until due and fired **exactly
  once**: the sweep claims each row with a conditional update, so the
  optional in-process ticker and a manual sweep cannot double-fire.
  Notifications are **in-app only** — no email, no push.
- **FR-16e Smart Score (data-driven prioritisation).** A derived,
  renormalised weighted average over ROI, strategic alignment, expert
  review, risk (inverted), demand, MoSCoW priority, and momentum, with a
  full per-component breakdown. **Absent evidence is dropped and
  disclosed** (`missing` + `coverage`), never scored zero; no evidence at
  all ⇒ `null` / `unscored`, sorted last rather than as a zero. ROI is
  computed only within a single currency (no FX conversion). Weights are
  deployment-tunable as a complete basis-point map; anything else falls
  back to the documented defaults with a warning. Nothing is stored, so
  the score cannot drift from its inputs.
- **FR-16f Bird's-eye visibility.** The challenge funnel
  (`/api/lifecycle`) reports every phase even at zero, with live and
  stalled counts, and items in an unresolvable phase counted separately.
  Per plan (`/api/plans/{pid}/lifecycle`), readiness for the next gate
  is a five-check **checklist** — each check reported with the count
  behind it — so "ready" means every check ran and passed.

### 6.4b Full-suite capabilities — service (§1.4–§1.6)

Committed by §1.4–§1.6; **not yet built** (§2.3). None enters the
matcher payload (§5.6); every mutation audits and emits its event
(§6.6). Types in §5.9, endpoints in §9.2b, tables in §10.6.

- **FR-26 Custom workflows.** CRUD workflow configurations
  (`/api/workflows`), deployment-wide or scoped to one plan, for tasks
  and for issues. Each state declares one of `todo` / `active` /
  `waiting` / `done`; **a state without a category is refused `422`**,
  never defaulted, so the board, burndown, timeline and every
  time-based-analysis figure stay computable over a custom vocabulary.
  A workflow must declare exactly one initial state and at least one
  `done` state, every transition must name declared states, and a state
  holding live work cannot be deleted. The built-in vocabularies become
  the default workflows, so a plan with nothing configured is unchanged.
  A task or issue move to a state its workflow does not permit from the
  current one is `422`, and — as today — a refused move writes **no**
  transition row.
- **FR-27 Objectives and key results.** CRUD key results under a goal
  (`/api/plans/{pid}/goals/{gid}/key-results`) and dated check-ins under
  a key result. Progress, objective score and plan score are **derived
  on read**, never stored. An objective with no measurable key result
  scores `null` / `unmeasured` and sorts last — never `0`.
  `start_value` is captured once and never recomputed. Check-in
  confidence is recorded and **never blended into the score**. Alignment
  rolls up through `parent_ref`; there is no second OKR hierarchy.
  Currency-valued key results compare within one currency only.
- **FR-28 Time tracking.** CRUD time entries against a plan and
  optionally a task (`/api/plans/{pid}/time-entries`), with roll-ups per
  task, per plan, and per assignee. Recorded effort is **never**
  substituted for elapsed time in a flow ratio, and is **never** an
  input to Smart Score (FR-16e) or to any Flow Framework metric
  (FR-31). Per-assignee roll-ups serve capacity, cost, "who should be
  asked about this", and **per-person utilisation** (FR-35). Every
  roll-up is labelled as **asserted**, distinguishing it from the
  observed transition log — which matters more now that a per-person
  figure is published from it.
- **FR-29 Sprint ceremonies.** CRUD sprints and the four ceremonies
  (planning / daily / review / retrospective) with categorised notes.
  Planning writes a **commitment snapshot** of the task set at sprint
  start, so later scope change reads as change rather than as a moved
  goalpost. Daily blockers convert into issues (FR-14); `action` and
  `feedback` retrospective notes convert into tasks, so an improvement
  gets an owner. Sprint burndown and velocity stay **sprint-scoped and
  count-based**, and are neither derived from nor merged into the
  item-scoped Flow Framework metrics.
- **FR-30 Project phases.** A plan carries `phase` (§5.9.4) and a
  transition log. Advancement is **one step at a time** — a skip is
  `422`. A backward move is permitted and **records a reason**; only a
  silent backward move is refused. `GET /api/plans/{pid}/phase-history`
  reports the transitions and the duration spent in each phase, with
  every phase present even at zero. **Phase never gates an operational
  write**: tasks may be created in Initiating and issues raised in
  Closing, because refusing writes on that basis would teach operators
  to misreport the phase. No constraint is enforced between `phase`, the
  lifecycle funnel, and the gate stage (§1.5.1); divergence surfaces as
  a readiness finding (FR-16f), not as a refused write.
- **FR-31 Flow Distribution.** `GET /api/plans/{pid}/flow-distribution`
  reports the feature / defect / risk / debt mix of completed work over
  a window, per plan and rolled up across a containment subtree. Items
  are classified from records already held (§5.9.5); an unclassifiable
  item is reported as **`unclassified` and counted separately**, never
  folded into `feature`. Where a deployment declares an intended mix the
  gap is shown against it; absent one, the mix is reported without
  judgement. The other four Flow Framework metrics are **already
  delivered** under time-based-analysis vocabulary and are not
  reimplemented — Flow Time is cycle/lead time, Flow Velocity is
  throughput, Flow Efficiency is flow efficiency, Flow Load is WIP
  (§1.6).
- **FR-32 Automation breadth.** Extends FR-16c within its existing
  invariants: additional triggers (a field change, a **phase
  transition**, a date arriving, an SLE breach) and **more than one
  action per rule**, applied in declared order with each action's
  outcome logged separately. Unchanged and non-negotiable: a failing
  rule never undoes the operator's action, actions are applied without
  re-entering the engine so automations cannot cascade, and every firing
  is logged whether applied, skipped or failed.

### 6.4c Value realization and strategic performance — service

The realized-gains and strategic-performance metric set. Types in
§5.9.6, endpoints in §9.2c, tables in §10.6. **Every figure is derived
on read, never stored**, so it cannot drift from the evidence it claims
to summarise (the Smart Score rule, §6.4a).

Honest starting position: `benefits`, `budget_lines`, `risks`,
`allocations` and a basis-point ROI helper already exist (PPM-10/11/12,
2026-07-18). These requirements add the measurement layer plus the three
missing inputs — adoption, business-case targets, and stakeholder
sentiment.

- **FR-33 Realized gains.** `GET /api/plans/{pid}/value-realization`
  reports, per plan and rolled up across a containment subtree:
  - **Transformation ROI** = `(realized value − total investment) /
    total investment`, where investment is the plan's recorded costs
    (budget-line actuals, plus recorded effort where time tracking is
    configured). Reported as a ratio *and* its numerator and denominator,
    never a bare percentage.
  - **Value Realization Rate** = completed initiatives that delivered
    their projected value ÷ completed initiatives. **This is a count of
    initiatives**, deliberately distinct from Benefit Realization Rate
    (FR-34), which is a ratio of *value*. A portfolio can score 90% on
    one and 40% on the other, and that difference is the finding.
  - **Time to Value** = `approved_at` → the first `ValuePoint` flagged
    `is_first_measurable`. Reported as a distribution with percentiles
    across a cohort, never as a mean (the percentiles rule,
    [time-based-analysis.md](time-based-analysis.md) §7.1).
  - **Adoption Rate** = `active_users / target_users` from the latest
    snapshot, returned **with** its stored `definition` and
    `window_days`, and refused rather than computed where
    `target_users` is zero or absent.
  - **Performance to Business Case** = actual against each
    `BusinessCaseTarget`, per metric, with the promised date and the
    approval that set it.

  **Absent evidence is disclosed, never scored zero** — a plan with no
  value points reports `unrealized` with a reason and sorts last, not
  `0%`, which would read as measured failure. ROI is computed **within a
  single currency only** (no FX), the same restriction Smart Score
  already carries.

- **FR-34 Strategic performance metrics.** `GET /api/plans/{pid}/performance`
  and the portfolio-wide `GET /api/performance`, organised in the six
  dimensions the source framework uses:

  | Dimension | Metric | Formula | State |
  |---|---|---|---|
  | Strategic | Strategic Alignment Index | aligned deliverables ÷ strategic objectives × 100 | new; objectives exist (§5.9.2) |
  | Strategic | Benefit Realization Rate | realized ÷ planned benefits × 100 | substrate delivered (PPM-11) |
  | Financial | ROI | net gain ÷ investment × 100 | helper delivered (`roi_basis_points`) |
  | Financial | Cost Performance Index | earned value ÷ actual cost | new; needs a phased budget baseline |
  | Financial | Budget Variance | budgeted − actual | delivered (financial variance view) |
  | Financial | Net Present Value | Σ net cash flow ÷ (1 + discount rate)^t | new; discount rate is deployment config |
  | Schedule | Schedule Performance Index | earned value ÷ planned value | new; same baseline dependency |
  | Schedule | Time-to-Value · Lead Time | FR-33 · already delivered (TBA §6.1) | partly delivered |
  | People | Capacity Utilization | effective hours ÷ available capacity × 100 | new — **see the constraint below** |
  | Quality & Risk | Defect Density · Technical Debt | defects ÷ unit of output · open `tech_debt` register effort | partly delivered (risk register) |
  | Quality & Risk | Risk Exposure · Mitigation Effectiveness | Σ probability × impact · residual reduction | substrate delivered (PPM-12) |
  | Stakeholder | NPS · CSAT | standard instrument scoring | new (§5.9.6) |

  **SPI and CPI report `null` with a reason on a plan with no phased
  budget baseline**, never `1.0` — a project with no baseline is
  unmeasured, not on track. NPV's discount rate is deployment
  configuration and is echoed in the response, because an NPV whose rate
  is not visible is not reviewable.

- **FR-35 Utilization, including per person.** Report effective effort
  against available capacity at **plan, team, and individual** level
  (`/api/capacity/utilization?by=plan|team|person`). Effort comes from
  `time_entries` (FR-28); available capacity from `allocations` plus the
  deployment's working-time configuration.

  **This reverses a standing refusal, deliberately.**
  [time-based-analysis.md](time-based-analysis.md) §12.4 refused
  per-person measurement and §2.4 declined utilisation; both were
  amended on 2026-08-25 to record this decision (§12.4a). The reversal
  is **scoped to utilisation**: per-assignee **cycle time, throughput,
  and flow efficiency remain refused**, and no endpoint returns them.

  Five obligations make the figure honest rather than merely available,
  and each is testable:

  1. **The denominator is declared and returned with the number** —
     never assumed at 100%. The response carries effective hours,
     available capacity, and the configuration that produced it.
  2. **Non-working time is excluded from the denominator**, not counted
     as idle: leave, holiday and non-project duty are absence of
     capacity, not failure to use it.
  3. **Small denominators are suppressed** — below a configured floor of
     capacity-days in the window the figure is `null` with a reason, the
     same posture as the flow gauges' small-board suppression.
  4. **It is never the sole ranking key of a person list**, and ships
     beside its numerator, its denominator, and the same period's queue
     and wait figures — because a reading near 100% is a warning about
     the queue (§2.3 there), not an achievement.
  5. **Effort stays labelled as asserted** (FR-28, §5.9.3): a person can
     move this number by logging more hours or declaring less capacity,
     which the transition log's incidental collection protected against
     and a timesheet does not.

- **FR-38 Controls — the Controlling process (§5.9.8).** CRUD controls
  under a plan (`/api/plans/{pid}/controls`), record readings, and
  record the action a failing reading provokes. This gives the
  **Controlling** phase (§1.5) a mechanism rather than only a name, and
  implements the four process steps as records: **set a standard**
  (`ControlStandard`), **measure** (`ControlReading`), **compare**
  (the derived verdict and gap), **act** (`ControlAction`).

  **What a failing control may do is fixed by its timing**, and this is
  the requirement, not a convention:
  - **Feedforward** controls may **block** a write (`422` naming the
    control) — acting before the fact is their entire purpose.
  - **Concurrent** controls may **warn and escalate** but must **never**
    silently undo the operator's action, the invariant FR-16c already
    holds for automations.
  - **Feedback** controls may only **record**. The work they judge is
    finished, and a control that rewrites history is not a control.

  Rules: a control whose `source` names a metric the service does not
  produce is refused at **write** (`422`), never left permanently
  `Unmeasured`; `Unmeasured` is a third verdict and is excluded from
  pass rates rather than counted as either; a failing reading with
  neither an action nor an explicit `Accept` is reported as
  **unanswered**; actions convert into existing tasks and issues rather
  than a fifth work store; readings are **append-only** and audited.

- **FR-39 Control coverage.** `GET /api/plans/{pid}/controls/coverage`
  and the portfolio-wide `GET /api/controls/coverage` report what is
  **not** being controlled — the question a register exists to answer:
  controls that have **never produced a reading**, controls whose last
  reading is older than their cadence, phases with **no controls at
  all**, and failing readings still unanswered. Every phase and timing
  appears even at zero, the same honesty rule as the lifecycle funnel
  (FR-16f): an empty cell is a finding, not a row to omit.

- **FR-37 Total Project Control (§5.9.7).** Record TPC observations per
  plan (`/api/plans/{pid}/tpc`) and report the derived view:
  - **DIPP** = EMV ÷ CEC, in basis points. Above `10_000` (1.0) the
    remaining value exceeds the remaining cost.
  - **DIPP Progress Index** = actual ÷ baseline DIPP, a Postgres
    `GENERATED ALWAYS` column so it cannot disagree with its own
    numerator and denominator.
  - **Divergence check**: the stored `dipp` against EMV ÷ CEC, reported
    as a finding rather than resolved by preferring either.
  - **Triage ordering**: `GET /api/tpc` ranks plans by DIPP descending,
    which is the use Devaux intends — scarce resources go to the highest
    DIPP, and **sunk cost appears nowhere in the figure**.

  Rules: money in minor units, ratios in basis points, **no float**;
  `CEC = 0` yields `null` with a reason, never infinity; a **negative
  EMV is legitimate and is not clamped**, because a project that is
  worth less than nothing to finish is exactly what the metric exists to
  expose; comparisons and rankings are **within one currency only**;
  every figure is labelled **asserted**, being an estimate rather than
  an observation.

- **FR-36 Stakeholder satisfaction.** Record NPS / CSAT responses
  against a plan (`/api/plans/{pid}/satisfaction`) and report the
  standard aggregate with its **response count and response rate**. A
  score without them is not reportable — an NPS of 100 from two
  respondents is not a finding. Responses store a role, never an
  identity (§5.9.6).

### 6.5 Cross-service links & bulk — service

- **FR-17 Cross-service links.** A plan / goal / task / issue can
  link to **any** index entity. The service ships the **write-side**:
  `POST`/`GET`/`DELETE /api/plans/{pid}/links` over an `entity_links`
  table and `linked` / `unlinked` events, per
  [`agents/share/cross-service-linking.md` §4](../../agents/share/cross-service-linking.md).
  Links are **never** a match signal (§7 there). The read-model
  aggregator (`link-graph-service`) is out of scope here (§2.3).
- **FR-18 Bulk import / export.** Async, job-based bulk per
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md):
  the five endpoints on `bg_pg`, JSONL / CSV / Parquet codecs,
  upsert-by-stable-key + dedupe-to-review, per-row error report, and
  export masking + audit, on the one plans collection.
  Portfolio-specific stable keys and CSV columns are in §9.6.

### 6.6 Auditability — service

- **FR-19** Audit log: a best-effort `audit_logs` row per create /
  update / delete / merge on a plan **and on every sub-resource
  write** (action + JSON snapshot + actor + timestamp), durable in
  Postgres; read at `/api/plans/audit/recent` and
  `/api/plans/{pid}/audit`. Per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
- **FR-20** Event streaming: a `PlanEvent`
  (`created`/`updated`/`deleted`/`merged`, plus sub-resource and
  `linked`/`unlinked` events) per write to an in-memory stream (MVP),
  read at `/api/plans/events/recent`; the durable bus
  ([`agents/share/event-bus.md`](../../agents/share/event-bus.md)) is
  roadmap (§15).

### 6.7 Security — service

- **FR-21** Offline PASETO v4 public token verification against the
  central auth-service's published Ed25519 key (embeds
  `authentication-verifier`, now a PASETO verifier); `whoami`
  protected; the audit / merge `actor` is stamped from the token when
  present. Blanket `/api/*` enforcement and paseto-keys-over-HTTP fetch are
  follow-ups (§13, awaiting the coordinated family SSO rollout). Auth
  source of truth (supersedes the RS256-JWT model):
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

### 6.8 Operator UI — front-end

- **FR-22** List active plans at `/plans`; create at `/plans/new`;
  detail at `/plans/[pid]` (render, edit, delete, check-duplicates);
  edit at `/plans/[pid]/edit`. A plan detail rolls up its child plans.
- **FR-23** Sub-resource workspaces under `/plans/[pid]`: goals, tasks
  (board + list), issues; plus the timeline and burndown views (§9.3).
- **FR-24** Check-duplicates posts the current record and lists matches
  (name, score, confidence), excluding the record itself; a merge
  action initiates `POST /api/plans/merge` (roadmap leg of the
  front-end).
- **FR-25** The plan form edits the full thin DTO: an optional `kind`
  label selector, comma-list inputs for names / keywords / tags /
  sameAs, row editors for goals, identifiers, and relationships, an
  `EntityRef` picker for `owner_org_id` / `lead_ref`, and a
  parent-plan picker for `parent_ref`.
