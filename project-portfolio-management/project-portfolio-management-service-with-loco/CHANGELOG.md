# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added — milestone-due date-arrival trigger (T-21, FR-32)

- A new `milestone_due` trigger kind fires when a milestone's own `due`
  date has arrived — the one dated field in the domain with
  unambiguous "arrived" semantics (a task's `due_at`/`start_at`/
  `finish_at` are each ambiguous about which one "the" date-arrival
  trigger would mean, so this ships narrowly on milestones rather than
  guessing a task-date convention).
- Unlike every other trigger, nobody's write makes a due date "arrive"
  — there is no event to hang the rule on — so this needed its own
  **exactly-once claim** rather than the existing fire-on-write path.
  New table `automation_milestone_fires (automation_pid, milestone_pid)`
  (migration `m20260902_000002_automation_milestone_fires`) with a
  `UNIQUE (automation_pid, milestone_pid)` constraint, claimed via
  `INSERT … ON CONFLICT DO NOTHING`. A suppressed conflict surfaces
  from sea-orm 2.0.2's `exec_with_returning` as `DbErr::RecordNotFound
  ("Failed to find inserted item")` — confirmed by running the claim
  twice against a real Postgres, not assumed from the crate source
  (which also defines a `RecordNotInserted` variant used by a
  different insert code path, `TryInsert`, not this one).
- New `POST /api/automations/milestones/sweep`
  (`sweep_milestone_due`): finds overdue, undone, non-deleted
  milestones (capped at `SWEEP_CAP`), and for each enabled
  `milestone_due` rule matching the milestone's plan, attempts the
  claim before applying the rule's actions — so a rule/milestone pair
  fires exactly once, ever, no matter how many times the sweep runs.
  The optional scheduler ticker now calls this sweep on every tick
  alongside the existing `sweep_due`; the endpoint also works
  standalone for a deployment driving both sweeps from external cron.
- `fire()`'s action-application logic is extracted into a shared
  `apply_rule_actions` helper so the write-triggered path and the new
  sweep log multi-action outcomes identically.
- Verified live: a new request test seeds one overdue and one
  far-future milestone, a matching `milestone_due` rule, sweeps twice,
  and confirms `fired: 1, already_claimed: 0` then `fired: 0,
  already_claimed: 1` — with exactly one `automation_runs` row
  throughout and the far-future milestone never firing.
  `cargo test --lib` 360/360 (was 358, +2); DB-gated suite 76/76 (was
  75, +1).
- **Field-change and SLE-breach triggers are deliberately not
  included** — both need a product decision (which fields count as
  "changed"; which SLE source and a once-only notification schema)
  that a mechanical implementation would otherwise have to invent. Left
  open in spec §13 T-21 pending that decision, on the same reasoning
  basis as the PRO-P33 controls-registration deferral.

### Added — multi-action automation rules (T-21, FR-32)

- A rule may now declare **more than one action**, applied in the
  array's declared order, with **each action's outcome logged
  separately**: `automations.action_kind`/`action_value` (one action)
  is replaced by `actions JSONB NOT NULL DEFAULT '[]'`
  (`[{"kind": …, "value": …}, …]`), and `automation_runs` gains
  `action_index` so a firing of an N-action rule writes N run rows
  instead of one silently overwriting the next. A `CHECK
  (jsonb_array_length(actions) > 0)` refuses an empty list even from a
  direct insert; migration `m20260902_000001_automation_multi_action`
  backfills every existing single-action rule into a one-element array.
- New pure `automation::validate_actions` (5 tests): validates every
  declared action with the existing per-kind `validate_action` and
  names the offending 0-based index on failure; capped at 20 actions
  per rule.
- `POST /api/automations`'s body changed from `action_kind`/
  `action_value` to `actions: [{kind, value}]` — a breaking change with
  **no back-compat shim**: there is no front-end consumer of this
  endpoint yet (repo `tasks.md` PRO-P20) and this service is pre-1.0
  with synthetic data only, so a clean cut was chosen over carrying two
  request shapes.
- Verified live against a fresh Postgres: a new request test seeds a
  two-action rule (`add_label` on an already-labelled plan, then
  `assign`) and confirms the first action logs `skipped` while the
  second still applies, proving one action's non-fatal outcome does
  not block the next. `cargo test --lib` 358/358 (was 353, +5);
  DB-gated suite 75/75 (was 74, +1).

### Added — OpenTelemetry OTLP export (PRO-H12 slice 7 of 7)

- Real OpenTelemetry OTLP export (`src/observability.rs`, repo
  `tasks.md` PRO-H12 slice 7 of 7 — the last): a `tracing-opentelemetry`
  bridge over an OTLP/gRPC span exporter, plus an OTLP/gRPC metric
  exporter feeding an `http.server.request.duration` histogram. On by
  default at `OTLP_ENDPOINT` (default `http://localhost:4317`); set it
  to the empty string to disable export and keep only local logging.
  Ported from case-service's slice 6; confirmed rather than assumed
  that this crate, like the other three loco-idiomatic registries, has
  exactly one router-construction surface, so `trace_mw` is layered
  once in `after_routes`. Proved end to end against a real in-process
  OTLP/gRPC collector (`tests/otlp_export.rs`, `tests/otlp_middleware.rs`),
  not merely against in-process SDK state. This closes repo
  `tasks.md` PRO-H12 — every entity registry in the family now
  exports real OTLP/gRPC traces and metrics.

## [0.3.0] - 2026-08-26

The project-management suite (entity spec §13 T-15 … T-27, landed
2026-08-25/26): custom workflows, the OKR engine, effort and time
tracking with per-person utilisation, sprint ceremonies, project
phases, Flow Distribution, Total Project Control, the controls
register, and value realization / strategic performance — plus the
time-based-analysis completions (TBA-9/10/11) and the earlier
unreleased hardening below. Nine migrations
(`m20260825_000001` … `000006`, `m20260826_000001` … `000003`); the
embedded matcher moves to 0.2 for the `Plan.phase` wire addition.

### Added — custom workflows (T-15 / FR-26)

Pure `src/workflow.rs`; migration `m20260825_000005_workflows` (three
tables: `workflows`, `workflow_states`, `workflow_transitions`);
`src/controllers/workflow.rs` — `POST`/`GET /api/workflows`,
`DELETE /api/workflows/{pid}`, `GET /api/plans/{pid}/workflow`. The
task **create** and **move** paths now validate against the workflow in
force rather than a compile-time constant.

- **Resolution order:** the plan's own workflow, else the deployment
  default, else the built-in vocabulary — a plan with nothing
  configured behaves exactly as before. **An empty transition set means
  unconstrained**; constraint is opt-in.
- **Schema-enforced:** every state's `category` is `NOT NULL` + CHECK
  (an uncategorised state is impossible even by direct insert), and a
  partial unique index makes two initial states impossible.
- **Two defects found by testing, both fixed:** `done_at` was stamped
  on the literal string `"done"`, so a board whose final column was
  renamed (`shipped`) never stamped it — now stamped from the state's
  **category** on both paths. And the TBA flow classes were keyed on
  status names, so a custom vocabulary arrived with no classification;
  `workflow::default_flow_classes` now derives classes from the
  categories, overlaid with `tba::default_classes()` so the built-in
  `in_review` stays *necessary* non-value-adding rather than silently
  raising every untouched board's flow efficiency (pinned by a test).
- **Not built:** a workflow edit route (withdraw and re-register);
  issue workflows are resolvable but unused (no issues sub-resource).

### Added — the OKR engine (T-16 / FR-27)

Pure `src/okr.rs`; migration `m20260825_000006_key_results`
(`key_results` + `key_result_check_ins`); `src/controllers/okr.rs` —
`POST`/`GET /api/objectives/{pid}/key-results`,
`POST`/`GET /api/key-results/{pid}/check-ins`,
`GET /api/plans/{pid}/okr`.

- **Key results hang off `objectives`, not `goals[]`** — the spec was
  corrected first: `Goal` carries no identifier (addressable only by
  array position, so a `goal_id` binding would be orphaned by any
  reorder), while an objective has a `pid`, a `period` (the OKR cycle),
  and weighted plan alignment through `objective_links`. The plan score
  is weighted by that existing link weight, not a second notion of
  importance invented for OKRs.
- A `decrease` key result starts at its baseline and reads **0%**, not
  100%; a check-in moves `current_value` and leaves `start_value`
  untouched; an unmeasured objective is reported, never scored as
  zero; start-equals-target is refused at write. `maintain` without a
  tolerance and a `currency` metric without a currency code are
  CHECK-refused as well as handler-validated.

### Added — effort / time tracking (T-17 / FR-28)

Pure `src/effort.rs`; migration `m20260826_000001_effort`
(`time_entries`, `working_time_configs`, `non_working_periods`);
`src/controllers/effort.rs` — `POST`/`GET
/api/plans/{pid}/time-entries`, `GET /api/plans/{pid}/effort`,
`POST /api/working-time`, `POST /api/non-working`,
`GET /api/capacity/utilization`. Roll-ups per plan, task and assignee,
every one labelled **asserted**; uncategorised effort is reported
separately rather than folded into `opex` (which would flatter the
capitalisable share); an entry over 1440 minutes for one date is
refused — a day cannot hold more.

### Added — per-person utilisation (T-24 / FR-35)

`GET /api/capacity/utilization?by=plan|team|person`, under the five
stated obligations of `agents/share/time-based-analysis.md` §7.1.

- Somebody on leave for the whole window reports `null` with
  `all_non_working`, **never 0%** — leave leaves the denominator, and
  0% would read as measured idleness. No declared capacity is its own
  reason (`no_declared_capacity`): an unknown denominator, not zero.
- Below the suppression floor the figure is withheld with its inputs
  still returned; team utilisation sums numerator and denominator
  (never a mean of ratios); at or over 100% is flagged, not clamped.
- Per-person cycle time, throughput and flow efficiency remain
  **absent from every endpoint** — the family-wide refusal stands.

### Added — sprint ceremonies (T-18 / FR-29)

Migration `m20260826_000002_ceremonies` (`ceremonies` +
`sprint_commitments`); `src/controllers/ceremony.rs` — `POST`/`GET
/api/sprints/{pid}/ceremonies`, `POST /api/sprints/{pid}/commit`,
`GET /api/sprints/{pid}/commitment`. The retrospective already existed
as `sprint_notes`; this adds planning, daily and review, and the
commitment snapshot. **The commitment is written once** — a second
`commit` is refused (handler *and* partial unique index), because a
rewritable commitment would let mid-sprint scope look like scope
committed at the outset; the view **names** what was added and removed
afterwards. Every ceremony kind is reported even at zero, so a sprint
that never retrospected is a finding, not a missing row.

### Added — project phases (T-19 / FR-30)

Pure `src/phase.rs`; migration `m20260825_000003_phase_transitions`
(denormalised `plans.phase` with a CHECK, an append-only
`phase_transitions` log, no backfill); `src/controllers/phase.rs` —
`PUT /api/plans/{pid}/phase`, `GET /api/plans/{pid}/phase-history`.

- One-step advancement; a skip is `422` **and names the phase
  skipped**; a backward move needs an explicit reason; an unknown token
  is refused, not coerced. Per-phase durations partition the elapsed
  time exactly and survive unsorted input and clock skew. `DELETE` on
  the history is `405` — append-only expressed as an absent route.
- The matcher gains `PlanPhase` + `Plan.phase` (0.2), informational-only
  and pinned never-scored, following the `PlanStatus` precedent.
- **Known gap, recorded not hidden:** `plans.phase` is not a separate
  field in the integrity pre-image (its authoritative value rides in
  `data`, which *is* covered); closing it means a
  `RECORD_HASH_VERSION` bump that would read as a false tamper alarm
  estate-wide, so it waits for a change that needs one anyway.

### Added — Flow Distribution (T-20 / FR-31)

Pure `src/distribution.rs`; migration `m20260825_000004_flow_type`
(nullable CHECK-constrained `tasks.flow_type`);
`src/controllers/distribution.rs` —
`GET /api/plans/{pid}/flow-distribution` with the subtree rollup.

- **The work type is declared, not derived** — the spec was corrected
  first: the specified derivation read fields that do not exist
  (`tasks` has no `goal_id`; there is no `issues` table), and would
  have classified everything `unclassified` while appearing to work.
- `unclassified` is **not storable** (it is the *absence* of a
  declaration); a declared intended mix produces gaps only for the
  types it names; a malformed intent is warned about and ignored
  wholesale. **A bug the tests caught:** an out-of-range intended share
  produced a gap of ≈ −9.2e18 via unchecked arithmetic — now
  range-guarded where the arithmetic is.

### Added — Total Project Control (T-25 / FR-37)

Pure `src/tpc.rs` — DIPP (EMV / CEC), the progress index, banding, the
stored-vs-computed divergence, and currency-scoped triage; money in
minor units, ratios in basis points, **no float**. Migration
`m20260825_000001_total_project_control` with
`dipp_progress_index_ratio` as a Postgres **`GENERATED ALWAYS`** column
(`NULLIF` denominator, so a zero baseline is `NULL` rather than an
insert-time division error, and the ratio can never disagree with its
own inputs) and a CHECK refusing a negative cost-estimate-to-complete.
`src/controllers/tpc.rs` — `POST`/`GET /api/plans/{pid}/tpc`,
`GET /api/plans/{pid}/tpc/report`, `GET /api/tpc` (triage).

- `CEC = 0` reports `null` **with a reason**, never infinity; a
  negative EMV is not clamped and bands `value_destroying`; triage sets
  aside a foreign currency and an undefined DIPP rather than ranking
  them as zero; two plans with equal remaining value and cost rank
  identically however much has been sunk into either — the property
  the whole metric exists for. Mapping doc:
  `../spec/total-project-control/index.md`.

### Added — the controls register (T-26 / FR-38, FR-39)

Pure `src/controls.rs` — the three timings (feedforward / concurrent /
feedback) and the response each permits, standard validation against
the metrics the service actually produces, four comparators, and the
coverage rollup. Migration `m20260825_000002_controls` (three tables;
CHECKs on `timing`, `comparator`, `verdict`, `kind` — the `verdict`
CHECK keeps `unmeasured` a real third value, so a typo cannot fall
through as "not a fail"). `src/controllers/controls.rs` — `POST`/`GET
/api/plans/{pid}/controls`, `GET /api/plans/{pid}/controls/coverage`,
`GET /api/controls/coverage`, `DELETE /api/controls/{pid}`,
`POST`/`GET /api/controls/{pid}/readings`,
`POST /api/readings/{pid}/actions`.

- An unknown metric is refused at **registration**, not left
  permanently unmeasured; a valueless reading is `unmeasured`, not a
  pass, and is excluded from the pass rate; a failing reading with no
  action and no explicit `accept` appears as **unanswered**; every
  timing appears in coverage even at zero.
- **Open:** action → task/issue conversion (the `converted_*_pid`
  columns exist, always `NULL` today), and registering the controls
  that already exist in all but name (gate readiness, WIP limits, the
  SLE, retrospectives).

### Added — value realization + strategic performance (T-22 / T-23, FR-33 / FR-34 / FR-36)

Pure `src/value.rs`; migration `m20260826_000003_value`
(`business_case_targets`, `value_points`, `adoption_snapshots`,
`satisfaction_responses`); `src/controllers/value.rs` — `POST
/api/plans/{pid}/{business-case,value-points,adoption,satisfaction}`,
`GET /api/plans/{pid}/{value-realization,performance}`.

- **A plan with no value points is `unrealized`, never a total loss.**
  `approved_at` has no update path, and a second first-measurable value
  point is refused by a partial unique index — the Time-to-Value clock
  stops once, and Time to Value is a distribution (nearest-rank
  p50/p85), never a mean.
- Adoption refuses a zero denominator at write and stores its own
  `definition` and `window_days`; investment is actual cost, and
  **mixed currencies withhold the ROI** rather than adding pounds to
  euros (the `budget_lines` shape carries a currency per line). The
  evidence mix (`measured_share_basis_points`) is disclosed.
- Strategic performance: NPS always carries its response count and
  reports `null` with `no_responses` rather than zero; responses store
  a **role, never an identity**; **SPI and CPI report `null` with
  `no_baseline`, never `1.0`** — a plan without a phased budget
  baseline is unmeasured, not on plan. Not built: the phased baseline
  itself, NPV, Strategic Alignment Index, defect density — each needs
  an input the service does not hold.

### Added — `plan_phase_changed` automation trigger (T-21 partial / FR-32)

Wired into the phase controller; deliberately its **own** trigger
rather than folded into `plan_stage_changed` — the gate stage and the
project phase are separate ordered vocabularies, and one rule firing on
both would fire on the wrong kind of change half the time. Phase
filters validate against **phases**, not task statuses (pinned by
test); the phase change commits before the rule fires, so a failing
rule never undoes the operator's move. **Open:** field-change,
date-arrival and SLE-breach triggers, and multi-action rules.

### Added — request tests for the suite (T-27)

Fourteen `#[ignore]`d request tests (`tests/requests/workflow_phase.rs`,
`tests/requests/metrics_control.rs`, `tests/requests/effort.rs`),
taking the DB-gated suite from 47 to 61 — each one a check previously
performed only by hand against a running service, now in CI, with the
assertions chosen for the *refusals*: undefined is not zero, unmeasured
is not a pass, unclassified is not a feature, a skip names what it
skipped. **It immediately found a real gap:** `tasks.flow_type` existed
and Flow Distribution read it, but nothing could set it — the field was
never added to the task-create payload (the manual pass seeded rows
with `psql`). Fixed: `flow_type` is accepted and validated on create,
with `unclassified` deliberately not accepted as an input.

### Changed — embedded matcher 0.1 → 0.2

`project-portfolio-management-matcher` 0.2.0: `Plan.phase` /
`PlanPhase` (an additive, `#[serde(default)]` wire-format field, pinned
never-scored). No matching behaviour changes; stored payloads
round-trip unchanged.

### Added — cross-plan rollup (TBA-9)

`GET /api/plans/{pid}/rollup`: flow across a plan and everything it
contains. This closes the last open TBA task.

**The combined figures are the union of the subtree's tasks, not an
average of the children's ratios.** Averaging ratios weights a
five-task plan equally with a five-hundred-task one — the same error
the item-level analysis already rejects.

**The per-plan table always ships with it, and for a portfolio it is
usually the more useful half.** A rollup mixes boards whose teams mean
different things by `in_progress` — the classification is
deployment-local by design, and nothing forces two teams to agree — so
*which child differs* is a firmer finding than the combined number.
That is what the §17 open question was really asking, and it is now
recorded as resolved.

**The walk is bounded three ways, for three different reasons.** A
visited set, because a cycle in `parent_ref` would revisit nodes and
expand exponentially rather than merely loop: the write path refuses a
cycle, but a rollup that *trusts* that is one bulk import or one direct
`UPDATE` away from hanging the service. A depth cap and a node cap, so
one enormous portfolio cannot become an unbounded response. Neither is
silent — `truncated` reports a cap firing and `revisits` reports
containment that is not a tree, because a rollup that quietly covers
half an estate reads as if it covered all of it.

The walk itself is a **pure function over an adjacency map**, so the
controller loads the containment map in one query rather than one per
level, and the cycle, depth and cap behaviour is unit-tested without a
database — including a self-parent, a diamond, and a 59-deep chain.

### Added — Monte-Carlo delivery forecasting (TBA-11)

`GET /api/plans/{pid}/forecast` answers both delivery questions at
once, because quoting one without the other is how a forecast gets
misread: *how long will these N items take* and *how many will land in
N periods*.

**It samples throughput, not cycle time — and this corrected an error
in our own spec.** §17 had claimed "the cycle-time distribution is
exactly the input a Monte-Carlo 'when will these 20 items be done'
forecast needs". It is not, and it is the standard error in the field.
Cycle time answers a question about **one item** — which is precisely
what the service level expectation already reports. A **batch**
forecast needs the **throughput** distribution: how many items the team
actually finished per period. Building it from cycle times implicitly
assumes items are worked one at a time, so summing twenty cycle times
for a team running five in parallel is roughly five times too
pessimistic. The spec entry is struck through rather than deleted, so
the correction stays visible.

Three properties are load-bearing:

1. **The conservative percentile reverses between the two questions.**
   For *how long*, higher is more conservative — 85% of simulated runs
   finished by the p85. For *how many*, it is the **15th** percentile:
   "at least this many, with 85% confidence". Quoting the p85 there
   would promise the best case while sounding careful, so the field is
   named `at_least_items` for what it means rather than for the
   percentile it came from, and both responses repeat the direction.
2. **It is deterministic.** The seed is an input, fixed unless
   supplied. A forecast that changes every time you reload it is not
   one anybody will act on — and determinism is what made the whole
   simulation testable.
3. **It refuses rather than guessing.** Below six periods of history it
   returns a reason instead of a number; an all-zero history returns
   *"the honest answer is `never`, not a number"*; and a per-trial
   ceiling turns what would otherwise be an unbounded accumulation loop
   into a reported `trials_hit_ceiling`, so a percentile that is really
   a floor says so.

Sampling is with replacement, which is what makes the output a
distribution rather than a replay of the past in its original order. A
zero seed is replaced, since xorshift from a zero state emits only
zeroes and would collapse every trial to the same answer.

### Added — time-based-analysis flow gauges (TBA-10)

A default-off Prometheus gauge family, `ppm_flow_*`: flow efficiency,
p85 cycle time, work in progress, first-pass yield and the over-cap
column count per plan, refreshed by a background loop
(`PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_SECS`, unset ⇒ the loop
never starts and the family never appears).

**Periodic refresh, not scrape-time computation** — computing on scrape
turns a 15-second scrape into a 15-second full-estate query on an
endpoint that needs no token, and updating on write cannot work because
these figures change as an item sits in a column, with no write to hang
an update on.

**Two bounds, because `/metrics.prom` is on the public allow-list.**
Per-plan series are **capped** (default 50, largest board first), and
small boards are **suppressed** (default floor 5): a flow efficiency
over two tasks describes two people's week, which §12.4 refuses to
measure, and reaching it by arithmetic through an unauthenticated
endpoint is the same thing. Neither bound is silent.

**Per-column occupancy is deliberately not exported.** It would be the
most useful detail and also five series per plan — the single biggest
cardinality contributor here. The over-cap *count* carries the
alertable fact ("a column on this plan is over its limit") in one
series, and the detail is one API call away.

**The p85 gauge inherits the service level expectation's refusal.**
Below `MIN_SLE_SAMPLE` finished items the SLE is null, and the gauge
stays absent rather than re-deciding the question with a number from
noise — rendering it as `0` would turn a refusal to forecast into a
claim of instant delivery.

The label is the plan **pid**, never its name (a rename would fork the
series); labelled series are reset each pass, so a plan that drops out
loses its series rather than keeping a stale value that looks live.

### Added — time-based analysis (TBA-1 … TBA-7)

The time dimension of delivery. `tasks` carried `status_changed_at` (when
the *current* status began) and `done_at`, so the moment a task moved
twice the first interval was gone — and the one question time-based
analysis exists to ask, *of the time this took, how much was somebody
actually working on it?*, could not be answered.

Flow efficiency in knowledge work typically measures **5–15%**, the same
order as Dr. R. C. Barker's finding that value-adding time is **8–14%**
of an NHS patient journey. That figure inverts the usual improvement
instinct: if an item is worked on 6% of its life, making the work 20%
faster improves delivery by about 1%, while removing half the waiting
improves it by nearly half. Velocity, utilisation and story points all
measure the 6%. Full contract:
[`spec/time-based-analysis.md`](../spec/time-based-analysis.md).

- **`task_transitions`** — the durable, append-only status-transition
  log, written by the **existing** `POST /api/plans/{pid}/tasks` and
  `PATCH …/{t_pid}` calls, **in the same transaction** as the change
  that caused it. No new recording endpoint, and no edit or delete: a
  method that asks engineers to log hours gets logged hours, not true
  ones, and an editable flow log measures whatever the editor wanted.
- **A labelled backfill** — one synthetic transition per live task, so
  an existing board is analysable immediately, flagged `backfilled` and
  surfaced in every analysis. Writing it without the flag would have
  been the same code and a lie.
- **`src/tba.rs`** — the pure analysis: interval derivation, cycle
  versus lead time, per-status and per-category splits, rework and
  rolled first-pass yield, handoffs, nearest-rank percentiles, the
  service level expectation, constraint ranking, aging WIP, and Little's
  Law. No I/O and no clock read (`as_of` is a parameter), so all 24 of
  its tests run without a database.
- **Endpoints** — `GET /api/plans/{pid}/{time-analysis,constraints,aging-wip,flow,cumulative-flow}`,
  `GET /api/plans/{pid}/tasks/{t_pid}/{transitions,time-analysis}`, and
  `GET /api/flow-classes`. All read-only, OpenAPI-documented, behind the
  blanket guard.
- **`cumulative-flow`** — the board's composition sampled daily, added
  with the front-end view. It is the one figure here that cannot be
  assembled client-side: it needs every task's whole history at once,
  and an API that shipped the log to the browser to re-derive it would
  be sending far more data to compute what the server already indexes.
  Every status band is present at every sample including at zero, so a
  stacked chart never has to decide whether a missing band means zero;
  a task does not appear before it was created, and one whose history
  predates its first recorded transition reads as `todo` rather than
  vanishing and reappearing mid-chart.

Five decisions are load-bearing and pinned by tests rather than left to
comments:

1. **Cycle time and lead time are different numbers, and both are always
   returned.** An item that sat in `todo` for three weeks and was built
   in two days has a cycle time of 2 days and a lead time of 23. Quoting
   the first as "our delivery time" is a tenfold flattering misreport
   and the commonest error in flow reporting, so the API returns them
   together and the response says why.
2. **The statuses partition the lead time, not the cycle time.** The
   backlog dwell is real time the requester waited and has to land
   somewhere — time that belongs to no status is time a report can
   quietly lose. Flow efficiency is still measured against cycle time,
   since the team cannot be held to how long the backlog sat.
3. **An unclassified status counts against you.** A board column nobody
   classified falls back to `unnecessary_non_value_adding`, so adding a
   column cannot silently improve the flow efficiency. The `in_review`
   argument is real and local, so the whole map is overridable via
   `PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES` — applied whole or not at
   all, and echoed on every response.
4. **Throughput never travels without first-pass yield.** A team whose
   throughput rises while yield falls is not going faster; it is
   shipping work back to itself.
5. **Nothing is per-person.** No per-assignee cycle time, throughput or
   flow efficiency — a stated refusal, not an unbuilt feature. It
   measures the wrong 6%, it is confounded by what the item was and who
   else was needed, and — because collection is a by-product of moving
   the card — it supplies the one reason anybody would have to distort
   the data. Handoff counts describe the item's journey.

There is no business-hours discounting and no clock pause: a weekend in
review really was a weekend in review, and working-hours arithmetic is
the standard way to make queues disappear from a report while the
customer still waits.






### Added — cargo-fuzz harness for the request-path logic (FUZZ-2)

A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate with
three coverage-guided libFuzzer targets. Until now the harnesses covered
only the dependency-light libraries; the services had none, despite
carrying the surface that actually faces the network.

- **`validate_json`** — the real request path: arbitrary bytes →
  `serde_json` → `Plan` → `validation::problems`. Never-panic,
  deterministic, and a **bounded problem report**.
- **`validate_built`** — the validator driven directly, building the
  `Plan` from raw bytes so the fuzzer controls array cardinality and
  entry contents without first having to learn JSON. A run of NUL bytes
  becomes a run of blank entries — the exact SEC-M8 shape.
- **`merge_plans`** — the merge fold over two arbitrary payloads:
  never-panic; the survivor keeps its `name`; deterministic; and
  **absorbing**, so a retried merge cannot inflate the record.

The sub-crate declares an empty `[workspace]` table: this crate is a
workspace root, so `fuzz/` would otherwise be pulled in as a member, and
a cargo-fuzz build needs its own sanitizer flags and lockfile. Nightly
only, so it is exempt from the repo MSRV. See
[`fuzz/README.md`](./fuzz/README.md).

### Fixed — an over-long array produced one `422` problem string per entry (SEC-M8)

`validation::problems` reported an over-long array's cardinality
violation once and then still walked **every** entry, so a payload with
ten thousand blank `keywords` or `tags` came back with ten thousand problem strings — which
the controller joins into a single `422` body. A small request bought a
large response.

Every per-entry loop now walks a new `inspected()` helper, which yields
at most `MAX_ARRAY_LEN` entries. The cardinality problem already rejects
the payload, so inspecting the tail decides nothing; bounding the
**report** is the same input-bounding rule (SEC-M1) as bounding the work.
The helper is named rather than inlined at each call site so a per-entry
loop added later without it reads as different from the ones that have
it. Pinned by a test.

Case landed this first as the reference; this is the roll-out
(repo `tasks.md` SEC-M8b).

Thirteen per-entry loops across `problems` and `push_size_cap_problems`
were involved, which is precisely why the cap is a named helper rather
than thirteen inline `.take(…)` calls.

### Fixed — the search index built a new Tantivy writer on every write

`SearchEngine::index_plan` (and `delete_*` / `clear`) called
`self.index.writer(WRITER_HEAP_MB)` per call. Tantivy's `IndexWriter`
allocates its whole 50 MB arena and spawns merge threads on
construction, so **every create, update, merge, and soft-delete paid
that setup synchronously**, on the request path. Measured at ~155 ms per
indexed document against a fresh index; holding one writer for the
process brings it to ~78 ms, the remainder being the durable commit and
reader reload that indexing-on-write inherently costs.

It was also a concurrency hazard, not only a slow one: an `IndexWriter`
holds the index directory's exclusive lock, so taking and releasing it
per call left two simultaneous writes able to collide on it. One owner
for the process cannot.

The engine now holds a `Mutex<IndexWriter>` created in `new()`. A
poisoned lock recovers the guard rather than failing for ever — the only
operations held across it are `delete_term` / `add_document` / `commit`,
and a permanently dead index would be the worse outcome.

Found by the new benchmark, which is the point of having one.

### Added — Criterion benchmarks

- `benches/service_bench.rs`, covering the CPU-bound halves of a request
  — the part a database benchmark hides behind I/O. Three groups:
  **validation** (every create and update pays it; the `oversized_arrays`
  case exercises the SEC-M1 input caps, because rejecting an abusive
  payload has to be cheap or the caps are not doing their job),
  **merge** (a whole-record fold, with a scaling case showing the cost
  sits in the collections it unions), and **search** (indexing one
  document — what every write pays synchronously — plus exact / fuzzy /
  phonetic retrieval and the `candidates` blocking query a duplicate
  check actually calls, against a populated index). The search group benchmarks the `kind` filter with and without a value, because `kind` narrows **retrieval** only and never gates matching — its cost belongs here and nowhere near the matcher.
- `criterion` is a new dev-dependency; test-only, so it is not in any
  release artefact.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

## [0.2.0] - 2026-08-05
### Added — pagination on the five operational sub-resource lists (PG-1, 2026-08-05)

Closes the sub-bullet left open when `GET /api/plans` gained pagination
(2026-08-01): `automations`, automation runs, the deadline queue,
delegations, and one inbox were still hard-capped at `LIST_CAP = 200`.

- **`GET /api/automations`, `GET /api/automations/runs`,
  `GET /api/scheduled-actions`, `GET /api/reviews`, and
  `GET /api/notifications` now take `?limit=`/`?offset=`**, reporting
  `X-Total-Count`/`X-Limit`/`X-Offset` (`agents/share/restful.md`) —
  the identical contract `GET /api/plans` already carries. Bodies stay
  bare arrays. Defaults reproduce the old cap (`LIST_CAP` = 200 on all
  five), `limit` clamps to 500, and an `offset` past 10 000 is a `400`.
- **The deadline queue's soonest-first `due_at` order holds under
  paging** — a page is a contiguous slice of the full sorted order, not
  a reshuffled one; a dedicated test assertion pins this rather than
  just re-checking the total count.
- **`controllers::pagination`** (new `src/controllers/mod.rs` module) —
  the `Page` struct and `with_page_headers` helper, promoted out of
  `controllers/plans.rs` so all six paginated collection reads (the
  original `/api/plans` + `/api/plans/search`, plus these five) share
  one implementation. `controllers/plans.rs` now imports from the
  shared module instead of carrying its own private copy; behaviour is
  unchanged (its own `list_and_search_are_paginated` test still passes
  untouched).
- DB-gated: two new tests in `tests/requests/capabilities.rs` —
  `automation_lists_are_paginated` (covers all three automation-side
  endpoints, including the soonest-first-under-paging pin) and
  `collaboration_lists_are_paginated` (delegations + one inbox).
- Front-end: `project-portfolio-management-front-end-with-svelte` wires
  the consuming routes to `ApiClient.getPage()`/`listPage()` — see that
  crate's own `CHANGELOG.md`.

### Added — Durable event bus, real-broker sink (BUS-3, following BUS-1's case-service reference, 2026-08-03)

`FluvioSink` (`src/relay.rs`) — the Phase-3 relay's real-broker
`EventSink`, behind this crate's own `fluvio` Cargo feature (off by
default; `fluvio` 0.50). One producer per topic, partitioned by record
`pid` per `agents/share/event-bus.md` §7. New env vars:
`PROJECT_PORTFOLIO_MANAGEMENT_FLUVIO_ENDPOINT` (unset ⇒ unchanged
`LoggingSink` default) and `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TOPIC`
(default `mxi.plan.events` — the `plan` streaming-entity token, per
`src/streaming.rs::ENTITY`, not the `portfolio` token `src/auth.rs`
uses for ABAC action naming). An endpoint configured without the
`fluvio` feature refuses to start the relay rather than silently
falling back to `LoggingSink` — that fallback would mark outbox rows
`published_at` without ever reaching the broker the operator asked
for. `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
SC+SPU broker (`mxi-project-portfolio-management-fluvio-*` container
names) for opt-in manual runs (not part of any automated CI stage);
`tests/fluvio_relay.rs` is a feature-gated, `#[ignore]`d live-broker
round-trip, verified by compiling under `--features fluvio` rather
than an actual execution (no broker is stood up in this repo's CI).
This crate carries no `compliance/soup.tsv`, so no SOUP register
update was needed (unlike case).

### Added — Privacy: field masking + GDPR export (2026-08-02)

Repo tasks.md P-4 (as P-1/organization; lower sensitivity). The
thinnest of the four privacy modules in the family: most of a `Plan`
(`name`, `code`, `goals`, `status`, dates, `identifiers`, `tags`,
`relationships`, `parent_ref`) is operational content, not personal
data.

- `src/privacy.rs` — `mask_plan` + `export_plan`. `lead_ref` (the plan
  lead, a `person:`/`worker:` `EntityRef` — the most directly personal
  field on a `Plan`) is **dropped entirely** rather than partially
  shown: unlike a phone number or a provider name, a partial UUID has
  no "still recognisable" value when the plan is already identified by
  its `name`/`code`. `owner_org_id`/`owner_org_name` (the sponsoring
  organisation — institutional, not personal, but still worth an
  obligation-driven redaction) are masked to their tail, the same
  treatment organization gives `telephone`/`email` and care-pathway
  gives `provider_name`/`provider_id`.
- `GET /api/plans/{pid}` was previously **unguarded** — no caller
  parameter, no ABAC check of any kind, even though `auth::authorize_record`
  + `auth::plan_resource_attrs` already existed and were wired into
  `PUT /{pid}` (PPM-3, the phase-gate `stage` attribute). Now honours
  the `mask` obligation the same way the other three entity services'
  `GET /{pid}` do.
- New `GET /api/plans/{pid}/masked` (always-redacted, no policy needed)
  and `GET /api/plans/{pid}/export` (the GDPR envelope, audited via the
  plain `AuditModel::record` — this crate has no HIPAA-style disclosure
  module, unlike case/care-pathway, so it follows organization's
  posture instead).
- DB-gated: `tests/masking.rs` (new, its own test binary from the
  start — case's P-3 found out the hard way that two masking-flavoured
  tests cannot share one binary's `policy()`/`require_auth()`
  `OnceLock`s) + 2 new tests in `tests/requests/plans.rs`
  (`masked_view_and_export_are_served`,
  `masked_view_and_export_are_404_for_unknown_pid`) + 6 new DB-free
  unit tests in `src/privacy.rs`.
- No SOUP register change (no new dependency; this crate carries no
  `compliance/soup.tsv` at all).
- **P-1 through P-4 are now complete** — every entity service in the
  family that carries personal or institutional data has field masking
  wired to the ABAC `mask` obligation.

### Added — Tantivy full-text/fuzzy/phonetic search (2026-08-02)

Repo tasks.md S-4: transfers the care-pathway/case Tantivy pattern
(S-2/S-3) whole — index module, streaming seam, reindex task with a
boot rebuild, duplicate detection blocked on the index instead of
scanning a capped 1000 rows. Portfolio adds one new wrinkle: the
optional `kind` label.

- `src/search/` — `PlanIndexSchema`/`PlanIndex` (`pid` STORED;
  `name`/`alternate_names`/`name_phonetic`/`identifiers`/`keywords`/
  `tags`/`goals`/`owner_org_name` TEXT; `code`/`owner_org_id`/`kind`/
  `status`/`active` STRING) and `SearchEngine` (`search_page`,
  `candidates`). A plan's defining attribute is what it is trying to
  achieve — goal titles are now searchable, alongside tags, the owner
  org, and every identifier scheme.
- `GET /api/plans/search?q=` is now Tantivy-backed with `?fuzzy=true`,
  `?phonetic=true`, and a new `?kind=` filter; `X-Total-Count` comes
  from Tantivy's `Count` collector rather than a SQL `COUNT(*)`.
  Replaces the Postgres `ILIKE` name search.
- **`kind` is a search filter, never a dedup gate.** `kind` is indexed
  as an exact-match field so `?kind=project` narrows a *search* — but
  `check-duplicates`' blocking query (`SearchEngine::candidates`)
  deliberately never applies it. The matcher is kind-agnostic by design
  (`project-portfolio-management-matcher` AGENTS.md: "do not reintroduce
  a kind gate — two plans with different kind labels may still be the
  same identity"), and this service's own golden rule 5 says the same:
  dedup / check-duplicates / merge are not scoped by kind. Gating the
  blocking query by kind would have silently reintroduced exactly the
  per-kind collection boundary the data model unification removed.
  `search::tests::candidates_ignore_kind` and the DB-gated
  `check_duplicates_blocks_on_identifier_alone_and_ignores_kind` pin
  this: a `Program`-labelled stored plan and a `Project`-labelled query
  still block against each other.
- `POST /api/plans/check-duplicates` now scores a **blocked** candidate
  set (fuzzy name, exact identifier, phonetic name — up to 200) from the
  index instead of an in-memory scan capped at 1000 rows. (The identical
  cap still backs `governance.rs`'s separate proposal-duplicate scan,
  which has no index of its own yet.)
- Both endpoints respond `503` (never a silent "no results") when the
  index is unavailable.
- `streaming.rs`'s `*_and_emit` seam indexes/deindexes best-effort after
  every commit, so no write path can skip it.
- `tasks/search.rs` — the `search_reindex` CLI task plus a
  rebuild-if-empty on boot.
- No SOUP register change — this crate carries no `compliance/soup.tsv`
  (personal-data sensitivity here is lower than case/care-pathway; see
  spec P-4).
- `.gitignore` gains `/data/` (the index's default on-disk path) — a
  derived, rebuildable artifact that must never be committed (a lesson
  from S-2, whose own `.gitignore` fix landed alongside S-3).
- DB-gated: `search_reaches_secondary_fields_tolerates_typos_and_filters_by_kind`,
  `check_duplicates_blocks_on_identifier_alone_and_ignores_kind`.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration → 2.0,
  sea-query → 1.0. No raw-`Statement` or `ExprTrait` fallout in this
  crate — it has neither.
- **loco's `ColType::PkAuto` now generates a 64-bit primary key.** 16
  entities move from `i32` to `i64`: `plans`, `audit_logs`,
  `merge_records`, and the governance/visibility/strategy phase tables
  (`proposals`, `gate_reviews`, `risks`, `budget_lines`,
  `plan_dependencies`, `milestones`, `allocations`,
  `report_definitions`, `ideas`, `scenarios`, `objectives`,
  `objective_links`, `benefits`) — plus the audit code that carries a
  row id (`compliance/audit_integrity.rs`'s `mismatched: Vec<i64>`, the
  `record_integrity.rs` test fixture). The 11 tables whose migrations
  write raw SQL (`id SERIAL PRIMARY KEY`) instead of the loco schema DSL
  — `event_outbox`, `insight_snapshots`, `reviews`, `automations`,
  `automation_runs`, `scheduled_actions`, `notifications`, `sprints`,
  `tasks`, `sprint_notes`, `devops_events` — stay `i32`.
- Also fixed three pre-existing `needless_borrows_for_generic_args`
  clippy findings unrelated to the width change but surfaced by the
  same `cargo clippy` run (`controllers/mod.rs`, `controllers/oversight.rs`
  passing `&reason` where `ErrorDetail::new` now takes it by value).
- No behavioural change; verified with the full DB-gated suite (38
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — pagination on the plan collection reads (2026-08-01)

- **`GET /api/plans` and `GET /api/plans/search` take `?limit=` and
  `?offset=`**, reporting `X-Total-Count` / `X-Limit` / `X-Offset`
  (`agents/share/restful.md`). Bodies stay bare arrays. Defaults
  reproduce the old caps (100 / 50), `limit` clamps to 500, and an
  `offset` past 10 000 is a `400`.
- **The `?parent=` scope reaches the count as well as the page**, so a
  child listing's total describes that parent's children rather than
  every plan. `list_paged` and `count_for` build the same predicate for
  exactly this reason — a total counting a different set from the page
  would be worse than no total.

**Still capped, not yet paged:** the operational sub-resource lists
(`automations`, automation runs, the deadline queue, delegations,
approvals) keep their `LIST_CAP` of 200. They are per-plan working
lists rather than the entity collection, and pagination there wants the
front-end screens that consume them; recorded in tasks.md PG-1 rather
than half-done here.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-2, the loco-style half of the rollout (case was the reference; the
five axum-style services landed the same day as AU-1).

- **The verifier and the ABAC policy are now reloadable holders**
  (`ReloadableVerifier` / `ReloadablePolicy`) that the blanket guard
  **and** the bearer extractors read per request. They were boot-only
  `OnceLock` snapshots, so a rotated key set or an edited policy could
  not have reached a running process at all.
- **`spawn_key_refresh`** re-fetches `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` every
  `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`spawn_policy_watcher`** polls `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE`'s mtime every
  15 s and calls `reload_policy()`; a malformed edit falls back to the
  built-in default rather than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, new here and in its
  own binary: `401` without a token, `403` for a valid token the default
  policy denies a write to, `200` for `access=write`.
- New environment variable: `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_REFRESH_SECS`.

### Fixed — the DB-gated suite ran for the first time (2026-08-01)

Both failures were in the tests; the service behaved correctly in each
case.

- **The workflow-automation test indexed an object as an array.**
  `GET /api/plans/{pid}/tasks` answers `{ "tasks": [...], "counts": {...} }`,
  so `moved[0]["assignee_ref"]` was `Null` and the assertion read as "the
  automation never fired". It had: the rule logged an `applied` run and
  the row carried the assignee. Now reads `moved["tasks"][0]`.
- **The burndown test hard-coded a sprint window that drifted into the
  past.** Burndown counts `done_at` stamps falling on or before each day
  in the window; the test completes a task *now*, and once "now" passed
  the fixed `ends_on` (2026-07-26) the completion stopped counting — the
  final point read 2 remaining instead of 1. The window is now relative
  to today (−6 / +7 days), keeping today inside a 14-day sprint.

  Suite: 36/36 green vs Postgres 18; crate enrolled in
  [`ci/db-suites.txt`](../../ci/db-suites.txt).


### Added

- 2026-07-22 — **Collaboration, automation, and prioritisation
  capabilities** (spec §9.4a). Migration
  `m20260722_000001_capabilities` adds `reviews`, `automations`,
  `automation_runs`, `scheduled_actions`, and `notifications`.
  - **Collaborative review** — `POST`/`GET /api/reviews`,
    `/{pid}/respond`, `/{pid}/submit`, `DELETE /{pid}`, and
    `/api/reviews/consensus`. Reviewers are `EntityRef` URNs, never raw
    emails; `reviewer_scope` records the internal/external disclosure
    decision explicitly. Only a reviewer who **accepted** may submit, so
    an unanswered invitation never becomes evidence; consensus requires
    a **strict** majority (a tie or plurality reports none) and always
    reports what is still outstanding.
  - **Assignees** — `POST /api/plans/{pid}/tasks/{t_pid}/assign`
    (`null` unassigns, notifies the assignee) and
    `GET /api/assignees/workload`, which surfaces the `unassigned` pile
    rather than dropping it.
  - **Workflow automation** — `POST`/`GET /api/automations`,
    enable/disable, delete, and `GET /api/automations/runs`. Rules fire
    from a Kanban move and from a submitted plan review. Action shapes
    are validated at write time; a failing rule is logged as a `failed`
    run and never undoes the operator's move; actions are applied
    without re-entering the engine, so automations cannot cascade.
  - **Set and forget** — `POST`/`GET /api/scheduled-actions`,
    `/sweep`, and cancel. The sweep **claims** each due row with a
    conditional update, so a deadline fires exactly once even if the
    optional ticker (`PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES`,
    default off) races a manual sweep. Sweeps are capped and say so.
  - **Data-driven prioritisation** — `GET /api/plans/{pid}/smart-score`
    and `/api/prioritisation`. The Smart Score is a renormalised
    weighted average over seven components with a full per-component
    breakdown; absent evidence is **dropped and disclosed** (`missing`
    + `coverage`), not scored zero, and a plan with no evidence scores
    `null` / `unscored` and sorts last. ROI is computed only within a
    single currency (no FX conversion, ever). Weights are tunable via
    `PROJECT_PORTFOLIO_MANAGEMENT_SMART_SCORE_WEIGHTS` (a complete
    10 000-basis-point map; anything else is warned about and ignored).
  - **Bird's-eye visibility** — `GET /api/lifecycle` and
    `/api/plans/{pid}/lifecycle`. Every phase is reported even at zero,
    items in an unresolvable phase are counted separately, and
    readiness is a five-check checklist that names each blocker.
  - Every mutation writes an `audit_logs` row; all four rule modules
    (`collaboration`, `automation`, `prioritisation`, `lifecycle`) are
    pure and DB-free with 56 unit tests, plus six `#[ignore]`d request
    tests and OpenAPI coverage.
  - **Not built:** email / push transport (notifications are in-app
    only), a `votes` Smart Score component (a plan carries no link back
    to its originating idea), and record-level ABAC on the new
    endpoints — they sit behind the blanket guard only.

- 2026-07-22 — **Five new `kind` labels: `Practice`, `Process`,
  `Purpose`, `Pathway`, `Proposal`.** `parse_kind_label` accepts the
  singular and plural spellings of each (`practice`/`practices`, …), the
  OpenAPI `Plan.kind` enum lists them, and the `kind_target` /
  `collection` validation messages on `/api/proposals`,
  `/api/ideas/{pid}/convert`, and `/api/reports` name them. Matching is
  unchanged — still kind-agnostic. (The `Proposal` **label** on a plan is
  unrelated to the `proposals` intake pipeline resource.)

### Changed — BREAKING: work items unified into one recursive `plan` collection (2026-07-20)

- **`WorkItem` → `Plan` rename.** The registered entity is now a **plan**.
  The embedded matcher exposes `Plan`, `PlanKind`, `PlanIdentifier`,
  `PlanRelationship`, `PlanStatus`, and `MatchingEngine::match_plans`
  (was `match_work_items`); `Plan::new(name)` leaves `kind` `None`. The
  service DTO, controller (`controllers/plans.rs`), and model
  (`models/plans.rs`) follow.
- **Four collections → one `/api/plans` collection.** The four REST
  collections (`/api/portfolios`, `/api/projects`, `/api/products`,
  `/api/programs`) are replaced by a single `/api/plans` collection; the
  `{collection}` path segment is gone. Plan-scoped sub-resources moved to
  `/api/plans/{pid}/...` (tasks, sprints, milestones, allocations,
  gate-reviews, risks, budget-lines, objectives, benefits, schedule,
  governance, audit).
- **`kind` is now an optional label — the kind gate is removed.** `kind`
  is `Option<PlanKind>`, an optional Portfolio / Project / Product /
  Program descriptive/grouping label; it no longer fixes a collection and
  no longer gates matching. The matcher's hard kind gate ("R-GATE") is
  gone — **any two plans may match** regardless of kind
  (`MatchBreakdown.kind_gate_blocked` remains only as a vestigial,
  always-false field).
- **General containment via `parent_ref` + cycle check.** `portfolio_ref`
  is renamed `parent_ref`: any plan may contain any other plan (a
  recursive tree). A `parent_ref` that points a plan at itself or at one
  of its descendants is now rejected `422` (new containment-cycle check).
- **Schema.** One `plans` table (was `work_items`) with a **nullable**
  `kind` column and a `parent_pid` column (was `portfolio_pid`);
  migration `..._000001_plans`.
- **Merge is unscoped.** Merge is no longer collection/kind-scoped — any
  two plans may merge; it returns `422` only on a self-merge (equal pids)
  and `404` on an unknown pid (the former cross-kind rejection is gone).
- **Proposals.** The proposal `kind_target` is now an optional descriptive
  label (blank = none), validated via the `parse_kind_label` helper.

### Added — engineering moderate fits (2026-07-20)

- Story points on tasks + `GET .../velocity` (team-local; real
  completions only); env-configured WIP limits enforced on board moves
  (`PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS`); sprint retro/feedback
  notes with once-only action/feedback → task conversion; DevOps event
  ingest (`/api/devops/events`) + DORA-style metrics derived only from
  ingested events (MTTR over linked pairs; declared-cause change
  failure) + the deploy-event release register.

### Added — engineering-team features (2026-07-20)

- The spec-§13 **tasks** sub-resource (Kanban statuses, PATCH board
  move with true flow stamps — `status_changed_at` per move, first
  `done_at` kept; PUT refuses status changes), **sprints**, and the
  honest **burndown** (real completions only, derivation served).
- The last-24h **standup digest** (audit-derived) and the estate
  views: blocked-work aging, the `moscow:<band>` scope cut, the
  delivery-links panel (external tracker identifiers), and the
  milestone calendar (`milestones.kind`:
  milestone/demo/release/checkpoint).
- Migration `m20260720_000001_engineering`; tasks/sprints never feed
  the matcher (the partition rule).

### Added — oversight areas: board / auditor / compliance / CRO / CISO / regulator (2026-07-20)

- Thirteen derived-view endpoints (`controllers/oversight.rs`) + the
  `insight_snapshots` table: the period board pack + investments +
  stored trend snapshots (explicit POST or env-gated ticker), the
  audit-trail explorer + segregation-of-duties findings + evidence
  pack (JSON/CSV), compliance/security risk registers + conformance
  findings, the CRO heatmap (posture, concentration, hygiene, declared
  risk appetite or an honest absence), and the deliberately coarse
  regulator extract honouring the ABAC `mask` obligation.

### Added — executive moderate fits (2026-07-19)

- **Stage-gated funding tranches**: `budget_lines.gate` + `released_at`
  (migration `m20260719_000002`); a gated line is held (actuals `422`)
  until the work item's stage reaches the gate and the new
  `POST …/budget-lines/{line_pid}/release` succeeds (fail-closed
  `gate_reached`; audited). `financials/exposure` reports per-currency
  `held_minor`.
- **Technical-debt register**: `risks.category` (validated closed set)
  + `GET /api/technology/debt` — `tech_debt` risks, exposure-sorted.
- **Delivery-flow metrics**: `milestones.done_at` stamped on complete +
  `GET /api/technology/flow` — throughput/month + median lead days;
  pre-stamp completions counted but never timed.
- **Strategic-alignment coverage**: `GET /api/executive/alignment` —
  aligned/unaligned per collection, unaligned spend per currency,
  ranked unaligned items (largest single-currency planned; disclosed
  heuristic).
- **Scenario comparison**: `GET /api/scenarios/compare?a=&b=` — two
  live evaluations side-by-side with per-currency deltas (b−a).

### Added — executive insight areas: CEO / CFO / CTO (2026-07-19)

- Seven read-only derived views over existing tables (no new
  migrations), ETag-conditional with `as_of`:
  `/api/executive/health` (per-portfolio RAG briefing),
  `/api/executive/decisions` (gate reviews, scenario commits, decided
  proposals, merges), `/api/executive/benefits` (per-currency target vs
  realized; honest null ratios), `/api/financials/variance` (by
  collection / category / portfolio; minor units; currencies never
  merged), `/api/financials/exposure` (per-currency totals, no FX),
  `/api/technology/dependency-risk` (fan-out / cross-portfolio /
  red-predecessor edges), `/api/technology/radar`
  (`tech:<name>[:<ring>]` tag convention, majority ring vote).
- Pure derivations live in `src/insights.rs` with DB-free unit tests;
  the RAG derivation is shared with `/at-a-glance`.

### Added

- 2026-07-18 — **PPM Phase C: strategy** (T-PPM-C; PPM-2/4/5/11).
  The idea funnel (capture / vote / dismiss / convert into a draft
  proposal, `provenance=idea` — completing idea → proposal → work
  item); what-if scenarios evaluated over live budgets, open risk
  exposure, and OKR alignment (per-currency saturating sums, budget
  cap + must-include violations; **infeasible commits refused**, the
  committed evaluation audited); the OKR objective registry with
  weighted (1–5) per-pair-upserting item mappings and
  per-collection alignment rollups; benefits with minor-unit
  financial targets or non-financial notes, accumulate-realize, and
  per-currency **ROI in basis points** against recorded budget
  actuals. Pure rules in `src/strategy.rs`; 3 unit + 4 DB-gated
  request tests vs Postgres 18.

- 2026-07-18 — **PPM Phase B: visibility** (T-PPM-B; PPM-6/7/8/9).
  Cross-item finish-start dependencies (cycle-refusing) + the
  portfolio schedule view (violations, memoised critical path,
  undated members); milestones with overdue flags; resource
  allocations over `person:`/`worker:` URNs + the per-person
  capacity rollup (summed percent over a window, > 100 % flagged);
  saved report definitions run synchronously as JSON or CSV
  (RFC-4180 escaping, row cap 1000); the ETag-conditional
  `/api/at-a-glance` dashboard (per-collection RAG — documented
  heuristic over materialised risks / overdue targets / budget
  overrun / exposure / schedule violations — stage distributions,
  and site tiles). Pure rules in `src/visibility.rs`; 7 unit + 5
  DB-gated request tests vs Postgres 18.

- 2026-07-18 — **PPM Phase A: the governance core** (T-PPM-A;
  PPM-1/3/10/12 from the entity roadmap). Work-intake `proposals`
  pipeline with matcher-backed duplicate-demand detection and
  promote-to-work-item (`provenance=intake`); strictly ordered
  phase-gate reviews (g0_concept…g5_benefits) advancing an
  operational `work_items.stage`, gate-lockable via the new
  `resource.stage` record-level ABAC (`auth::authorize_record`);
  risks (1–5 × 1–5 exposure, escalation); budget lines in integer
  minor units + ISO-4217 with per-currency planned/actual/variance;
  the per-item `/governance` summary. Pure rules in
  `src/governance.rs`; every mutation audited; OpenAPI `governance`
  tag; 4 unit + 5 DB-gated request tests, verified against
  Postgres 18.


### Fixed

- 2026-07-18 — **Unknown-pid reads returned 500, not 404.** loco 0.16's
  `IntoResponse` catch-all maps an unmapped `ModelError::EntityNotFound`
  to a 500, so `GET /…/{pid}` with an unknown pid crashed instead of
  404ing (the organization service was immune — its `http_err` helper
  already mapped it; the copy-adaptors dropped it). Controller lookups
  now route through a `model_not_found` mapping. Family-wide fix with
  per-crate request-test pins.


### Changed

- 2026-07-18 — **Subproject renamed**: `portfolio` →
  `project-portfolio-management` (directory, crate/package name, lib
  ident, env-var prefix `PORTFOLIO_*` → `PROJECT_PORTFOLIO_MANAGEMENT_*`,
  database names). The **domain language is unchanged**: the work-item
  kinds (portfolio / project / product / program), the `work_items`
  table, the API routes, and the matcher's `WorkItem` type keep their
  names — the rename repositions the *subproject* as a project
  portfolio management (PPM) product; see the feature roadmap in
  `../spec/15-roadmap.md`.


### Fixed

- 2026-07-18 — **Fresh-database `db migrate` failure.** The
  `…_000004_event_outbox` migration used the loco `create_table`
  helper, which pluralizes table names (`event_outbox` →
  `event_outboxes`); its own index DDL then failed and rolled back
  the entire fresh migrate (zero tables). Rewritten as explicit SQL
  creating exactly `event_outbox`; verified against a fresh
  Postgres 18 (all migrations apply, correct table names). Family-wide
  fix (case, care-pathway, organization, portfolio; patient-flow
  shipped with the explicit-SQL form).


### Security

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `/merge` / `/deduplicate` / `/import` via
  `path.ends_with`, so a trailing slash (`POST …/merge/`) fell through to
  `Write` — a non-admin `access=write` caller could reach a destructive op.
  The path is now `trim_end_matches('/')`-normalised first. Test extended.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Security — SEC-M1: input-size caps on the validation entrypoint (2026-07-13)

- `src/validation.rs` now rejects oversized `WorkItem` payloads before the
  record is stored or matched, closing a CPU/memory denial-of-service
  vector: the matcher runs `O(n·m)` string similarity (Jaro-Winkler /
  Soundex) and Jaccard over the payload's text fields and arrays, so a
  single huge string or huge array is a DoS (amplified by the
  check-duplicates scan). New named caps enforced (all problems collected,
  never aborting early, surfaced as `422`): `MAX_TEXT_LEN = 1024`
  Unicode scalar values per scalar text field (`name`, `code`,
  `owner_org_id`, `owner_org_name`, `lead_ref`, `portfolio_ref`,
  `start_date`, `target_date`, `in_language`); `MAX_ARRAY_LEN = 256`
  entries per array (`alternate_names`, `goals`, `keywords`, `tags`,
  `identifiers`, `same_as`, `relationships`); `MAX_ITEM_LEN = 512` per
  string entry inside an array (each entry, plus `goals[i].title`,
  `identifiers[i].value`, `relationships[i].work_item_id`). The `kind`
  discriminator is an enum, not free text, so it is not capped. New unit
  tests cover oversized single field, oversized array, oversized array
  item, and a within-caps large-but-valid record.

### Changed — event bus: audit now joins the outbox transaction (2026-07-09)

- Under the `outbox` transport, the `audit_logs` write now rides the
  **same transaction** as the entity mutation and its `event_outbox` row
  (`agents/share/event-bus.md` §3 — the three "can never disagree"). It
  was previously a best-effort side channel written *after* the
  transaction committed, so a crash or audit failure could leave a
  committed change + event with no audit row. `AuditModel::record` is now
  generic over `ConnectionTrait`; the `create/update/delete/merge_and_emit`
  functions own the audit write (strict/in-txn under `outbox`, best-effort
  logged under `memory`), and the `work_items` controller no longer audits
  separately. New DB-gated `tests/outbox_audit.rs` drives `create_and_emit`
  under `outbox` and asserts entity + event + audit all commit together.
  (The `merge_records` history row stays a best-effort side channel — it
  is merge metadata, not the §3 audit trail.)

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (supersedes the earlier per-crate
  roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`; matched on path suffix across
  all four collections), and the policy is evaluated over the token's
  new `attrs` claim, first-match-wins, defaulting to allow-read /
  deny-mutation.
- New env vars `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY` (inline JSON) and
  `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and returns
  `403` (deciding-rule reason) for a valid token the policy denies;
  `401` remains missing/bad credential. `require_auth_mw` in `app.rs`
  passes the request method and `auth::policy()`. DB-free unit tests
  pin the family §7 matrix. Flag off ⇒ behaviour-neutral.

### Added

- **Boot-time paseto-keys-over-HTTP fetch** (the spec §13 follow-up, done
  2026-07-04). New optional env var `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`: when set
  (non-blank), `auth::init` — called from `App::after_routes`, before the
  app serves traffic — fetches the auth-service's published Ed25519 key
  set once over HTTP via `Verifier::from_paseto_keys_url` (the
  `authentication-verifier` crate's `fetch` feature, now enabled). On
  success the fetched key set **wins** over the `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS`
  env key set (`tracing::info!`); on failure the service logs a
  `tracing::warn!` and falls back to the env path, so it **always
  boots**. Unset/blank ⇒ prior behaviour unchanged (env key set, else
  empty reject-all). Fetch is once-at-boot only — no refresh loop
  (rotation-triggered refetch is tracked in spec §16). The seeding is
  idempotent (`OnceLock`), and the fetch-or-fallback helper
  (`auth::fetch_or`) is dependency-injected (URL / issuer / audience /
  fallback passed in) so tests cover it without the process global: a
  `#[tokio::test]` local ephemeral-port HTTP listener proves a token
  signed by the served key verifies via the fetch-built verifier, and a
  fast-failing URL (`http://127.0.0.1:1/`) proves fallback without
  panic. Existing env-key auth tests unchanged and green.

## [0.1.0] - 2026-06-18

### Added

- **Inaugural spec scaffold (spec-only — no code yet).** Documentation
  set for the loco.rs work-item registry **and** project-management tool:
  - `spec/index.md` — the §1–§18 single-source-of-truth service spec,
    mirroring the care-pathway service shape. Defines the **four matchable
    collections** (`portfolios`, `projects`, `products`, `programs`) — one
    JSONB row table per kind, sharing one parameterised controller core
    (the API DTO **is** `project_portfolio_management_matcher::WorkItem`, persisted verbatim,
    matched with no adapter); **within-kind matching only** (the matcher's
    R-GATE makes a project never match a product); the umbrella hierarchy
    (Projects / Products / Programs carry a `portfolio_ref` to their parent
    portfolio); the operational sub-resources (goals, tasks, issues) in
    their own tables keyed by the parent `(kind, pid)` and **excluded from
    the matcher payload** (goal titles bridge via `data.goals[]`); the
    derived timeline / burndown read views; CRUD + soft-delete + audit;
    embedded probabilistic + deterministic matching (`POST /match` /
    `/check-duplicates` / `/deduplicate`); real-time create duplicate
    detection (`409`) + review queue; record merge (`Replaces` link +
    transferred snapshot + `Merged` event, same-kind only); `ILIKE` name
    search; event streaming (durable-bus Phase 1 envelope); OpenAPI/Swagger;
    per-collection Prometheus metrics; offline PASETO v4.public verification
    + blanket `/api/*` enforcement (off by default, gated by
    `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`); cross-service entity links (write side); and
    bulk import/export (deferred).
  - `README.md` — user-facing intro, route table, quick start, status.
  - `CLAUDE.md` — one-line `@AGENTS.md` include.
  - `AGENTS.md` — agent guide (what this is, API surface, MVP scope,
    golden rules incl. four-kinds-one-core, within-kind matching, and the
    matcher-partition rule, intended layout).
  - `index.md` — documentation index + worked flow.
- **Auth model is PASETO v4.public + cookie sessions (spec-only).** The
  intended auth design is **server-side cookie sessions** for the human
  session plus **offline PASETO v4.public** verification for peers
  (verified against the auth-service's published **Ed25519 key**), and a
  **BFF** for the front-end so the browser holds no token. The
  `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` flag + enforcement semantics follow the family
  contract. Source of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (RS256/JWKS not used).
- **Adopts the cross-service-linking contract.** Portfolio is a
  participating service with an `entity_links` write-side table and
  `POST`/`GET`/`DELETE /api/{collection}/{pid}/links` emitting `linked`
  / `unlinked`; a work item / goal / task / issue can link to **any** index
  entity. Cross-service links are **not** a matcher signal (separate from
  within-payload `relationships`). Contract:
  [`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md).
- **Adopts the bulk-import/export contract** (deferred §13). Async
  `bg_pg` jobs, JSONL/CSV/Parquet, the five endpoints under
  `/api/{collection}/*`; stable upsert key = a deterministic external PM
  identifier (Jira / Asana / Trello / MS Project / GitHub Project / Linear /
  URI / UUID) or owner-scoped `code` or `pid`; keyless rows → dedupe →
  review queue (within-collection). Lead / person refs are personal data →
  export audited. Contract:
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md).

### Notes

- No Rust / Cargo crate has been generated; every `spec.md §13` task is
  unchecked. Next step is `loco new` (stripped of the auth starter) plus
  the four work-item tables + the shared CRUD MVP.
- The canonical `WorkItem` domain model is owned by the
  [portfolio entity spec §5](../spec/index.md); this crate spec references
  it.
- Copy-adapted from the (deleted) `plan` service template; the headline
  differences are the **four distinct matchable kinds** (vs plan's single
  `plan_type` field), the within-kind match **gate** (R-GATE), and the
  dropped `posts` / `comments` / `members` sub-resources (now deferred
  roadmap).

[Unreleased]: #unreleased
[0.3.0]: #030---2026-08-26
[0.2.0]: #020---2026-08-05
[0.1.0]: #010---2026-06-18
