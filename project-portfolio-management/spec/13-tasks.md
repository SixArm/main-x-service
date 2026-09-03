## 13. Tasks

Live entity-level work queue. (Historical header — the trio has been
**implemented** since 2026-06-19; per-crate work now lives in each
subproject's own spec §13. The unchecked items below are the original
build-out backlog kept for trace; the PPM feature catalogue and its
delivery state live in [15-roadmap.md](15-roadmap.md).) Tasks that belong to one subproject's internals
migrate into that crate's spec §13 once the crate is scaffolded; they
are listed here while the trio is being stood up. Each task has an
acceptance criterion; tick the box when an automated test or clearly
described manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [x] **2026-07-22 — Collaboration, automation, and prioritisation
  capabilities (§6.4a / §9.2a / §10.5).** Service: migration
  `m20260722_000001_capabilities`, four pure rule modules
  (`collaboration` / `automation` / `prioritisation` / `lifecycle`,
  56 unit tests), three controllers, the automation engine firing from
  a board move and a submitted plan review, the claim-based
  set-and-forget sweep + optional ticker, OpenAPI, six `#[ignore]`d
  request tests. Front-end: `CapabilityClient` + the `/prioritisation`,
  `/lifecycle`, `/reviews`, and `/automations` pages (English-first).
  Not built: email / push transport, a `votes` Smart Score component,
  record-level ABAC on the new endpoints, and a notifications page.

- [x] **T-15 — Custom workflows (§5.9.1 / FR-26).** **Landed 2026-08-25.**
  - Pure `src/workflow.rs` (12 tests); migration
    `m20260825_000005_workflows` (three tables); three entities;
    `src/controllers/workflow.rs` (4 routes); the task **create** and
    **move** paths now validate against the workflow in force rather
    than a compile-time constant.
  - **Resolution order:** the plan's own workflow, else the deployment
    default, else the built-in vocabulary — so a plan with nothing
    configured behaves exactly as before. Nothing is seeded; the
    built-ins stay code defaults.
  - **An empty transition set means unconstrained**, preserving today's
    open board. Constraint is opt-in.
  - **Schema-enforced, verified against raw SQL:** `category` is
    `NOT NULL` + CHECK (an uncategorised state is impossible even by
    direct insert), an invented category is refused, and a partial
    unique index makes two initial states impossible.
  - **Verified end to end:** a custom vocabulary takes force
    (`source: plan`); the old vocabulary is then refused on that plan;
    creation defaults to the workflow's initial state; an undeclared
    transition is refused **and writes no transition row**; the
    workflow's own WIP cap bites ahead of the env map.

  **Two defects found by testing, both fixed:**

  - **`done_at` was stamped on the literal string `"done"`.** A custom
    vocabulary finishing in `shipped` never stamped it, leaving the
    burndown blind to every board that renamed its final column —
    precisely the failure the mandatory category exists to prevent. Now
    stamped from the state's **category** on both the create and move
    paths.
  - **The TBA flow classes are keyed on status names**, so a custom
    vocabulary arrived with **no** classification and would have
    reported no value-adding time at all. `workflow::default_flow_classes`
    now derives classes from the categories, resolved per plan by
    `controllers::tba::classes_for`.

  **A regression I introduced and then caught**, worth recording because
  the first test missed it: four categories cannot express *necessary*
  non-value-adding, so the derivation called the built-in `in_review`
  **value-adding** where the disclosed default map calls it
  *necessary*. Every untouched board's flow efficiency would have risen
  because of an unrelated feature. Fixed by overlaying
  `tba::default_classes()` on top of the derivation, and the
  disagreement is now pinned by a test whose doc comment says why —
  the earlier version checked only three keys, passed, and let the
  regression reach a running service.

  **Not built:** issue workflows are resolvable but unused (there is no
  issues sub-resource — §14.2); no workflow **edit** route (withdraw and
  re-register); the withdraw guard checks tasks only; front-end
  `/workflows`. Request tests committed
  (`tests/requests/workflow_phase.rs`).

- [x] **T-16 — OKR engine (§5.9.2 / FR-27).** **Landed 2026-08-25.**
  Completes the "create a full OKR engine" reversal (§17.3).
  - **The spec was corrected first.** It anchored key results to a
    plan's `goals[]` via a `goal_id`. `Goal` carries **no identifier** —
    a bare struct in the JSONB payload, addressable only by array
    position — and **no goals sub-resource exists** (FR-12 specified,
    built nowhere; now recorded in §14.2). A key result bound to an
    array index would be orphaned by any reordering. Key results now
    hang off **`objectives`**, which already has a `pid`, a `period`
    (the OKR cycle) and weighted plan alignment through
    `objective_links` — the O in OKR, and the correct anchor.
  - Pure `src/okr.rs` (9 tests); migration
    `m20260825_000006_key_results`; two entities;
    `src/controllers/okr.rs` (5 routes); OpenAPI + pinning test.
  - **The plan score is weighted by the existing `objective_links`
    weight**, not by a second notion of importance invented for OKRs.
  - Schema-enforced: `maintain` without a tolerance and a `currency`
    metric without a currency code are both CHECK-refused, as well as
    validated in the handler.
  - **Verified end to end** (`tests/requests/metrics_control.rs`, three
    tests): a `decrease` key result starts at its baseline and reads
    **0%**, not 100% (seeding `current_value` at zero would have
    reported it complete on the day it was created); a check-in moves
    `current_value` and **leaves `start_value` untouched**; an
    unmeasured objective weighted 5× does **not** drag the plan score
    down and is still reported; a key result whose start equals its
    target is refused at write rather than reading `unmeasured` for a
    quarter.
  - **A test assumption of mine was wrong, not the code:** alignment
    weight is capped 1–5 (`strategy::valid_weight`), so my first
    version's `weight: 99` was refused and the link never landed. Fixed
    in the test.
  - **Not built:** no key-result **edit** route (a check-in is the way
    to move a value, which is deliberate — but title and target are also
    currently immutable); no objective-level `period` rollup; front-end
    `/plans/[pid]/okr`.

- [x] **T-17 — Time tracking (§5.9.3 / FR-28).** **Landed 2026-08-26.**
  Pure `src/effort.rs` (9 tests), migration `m20260826_000001_effort`,
  three entities, `src/controllers/effort.rs`. Roll-ups per plan, task
  and assignee, every one labelled **asserted**; uncategorised effort
  reported separately rather than folded into `opex`, which would
  flatter the capitalisable share. An entry over 1440 minutes for one
  date is refused — a day cannot hold more. Verified in
  `tests/requests/effort.rs`.

- [x] **T-18 — Sprint ceremonies (§5.9.3 / FR-29).** **Landed
  2026-08-26.** Migration `m20260826_000002_ceremonies`, two entities,
  `src/controllers/ceremony.rs`. The retrospective already existed as
  `sprint_notes`; this adds planning, daily and review, and the
  commitment snapshot.
  - **The commitment is written once**, and a second `commit` is
    refused: a rewritable commitment would let mid-sprint scope look
    like scope committed at the outset. The view **names** what was
    added and removed afterwards rather than only counting it.
  - A second planning or review is refused in the handler *and* by a
    partial unique index — that is a re-plan, which is a new sprint.
  - Every ceremony kind is reported even at zero, so a sprint that never
    retrospected is a finding rather than a missing row.

- [x] **T-19 — Project phases (§5.9.4 / FR-30).** **Landed 2026-08-25.**
  - Matcher crate: `PlanPhase` + `Plan.phase`, informational-only and
    **pinned never-scored** (`phase_is_not_scored`), following the
    `PlanStatus` precedent exactly. 58 unit + 6 integration tests green,
    clippy and fmt clean.
  - Service: pure `src/phase.rs` (8 tests) — one-step advancement,
    explicitly-reasoned regression, per-phase durations that partition
    the elapsed time exactly and survive unsorted input and clock skew;
    migration `m20260825_000003_phase_transitions` (denormalised
    `plans.phase` with a CHECK, append-only log, **no backfill**);
    entity; `src/controllers/phase.rs`; OpenAPI + pinning test.
  - **Verified end to end** against Postgres 18: a skip is `422` **and
    names the phase skipped**; a backward move is `422` without a reason
    and `200` with one; an unknown token is refused, not coerced; the
    history reports all five phases with the revisit counted separately
    (2 visits, not merged into one total); payload and column agree
    (§5.8); `DELETE` on the history is `405`, the append-only property
    expressed as an absent route.
  - **Known gap, recorded not hidden:** `plans.phase` is **not** a
    separate field in the integrity pre-image, unlike `kind` / `name` /
    `parent_pid` / `stage`. Its authoritative value rides in `data`, so
    it *is* covered — but a raw-SQL edit of the column alone would not
    break the digest (it would break the §5.8 payload-equals-column
    invariant instead). Closing it means bumping `RECORD_HASH_VERSION`,
    which invalidates **every** stored digest estate-wide and would read
    as a false tamper alarm on every record. The bump waits for a change
    that needs one anyway. Reasoning is in
    `src/compliance/record_integrity.rs`.
  - **Not built:** phase-transition automation triggers (FR-32).
    Request tests committed (`tests/requests/workflow_phase.rs`).

- [x] **T-20 — Flow Distribution (§5.9.5 / FR-31).** **Landed 2026-08-25.**
  - **The spec was corrected first.** §5.9.5 had said the work type
    could be *derived* — feature from a task's `goal_id`, defect from an
    issue's `kind`. Checking the tree found `tasks` has no `goal_id`
    (objectives link to **plans**) and **there is no `issues` table at
    all** (FR-14 specified in §6/§9/§10, built nowhere — now recorded in
    §14.2). The derivation would have classified everything
    `unclassified` while appearing to work. The type is now **declared**,
    which is also what the Flow Framework itself assumes.
  - Pure `src/distribution.rs` (8 tests); migration
    `m20260825_000004_flow_type` adding a nullable CHECK-constrained
    `tasks.flow_type`; `src/controllers/distribution.rs` with the
    subtree rollup; OpenAPI + pinning test.
  - **Verified end to end** against Postgres 18: `unclassified` is
    **not storable** (the CHECK refuses it — it is the *absence* of a
    declaration, and a spelling would let a row claim to have been
    classified as unclassified); a mix of 2 feature / 1 defect / 1
    undeclared / 1 closed `tech_debt` risk reported 40/20/0/20/20 with
    `unclassified` standing alone; a closed `delivery` risk was
    correctly excluded; a declared intent produced gaps only for the
    types it named; a malformed intent was warned about, ignored
    **wholesale**, and the service still booted.
  - **A bug the tests caught:** an out-of-range intended share did not
    overflow `checked_sub`, so `i64::MAX` produced a gap of ≈ -9.2e18 —
    a nonsense number that looks like a measurement. Fixed in
    `distribution()` by range-guarding the intent where the arithmetic
    is, rather than trusting every caller to have come through
    `parse_intent`.
  - **Not built:** the front-end `/plans/[pid]/distribution` route.
    Request tests committed (`tests/requests/metrics_control.rs`).

- [~] **T-21 — Automation breadth (FR-32).** **Partially landed
  2026-08-26, extended 2026-09-02.**
  - [x] **`plan_phase_changed` trigger**, wired into the phase
    controller. Deliberately its **own** trigger rather than folded into
    `plan_stage_changed`: the gate stage and the project phase are
    separate ordered vocabularies (§1.5.1), and one rule firing on both
    would fire on the wrong kind of change half the time.
  - [x] Phase filters validate against **phases**, not task statuses —
    the two vocabularies are disjoint, so validating one against the
    other would reject every legitimate rule. Pinned by
    `a_phase_trigger_filters_on_phases_not_task_statuses`.
  - [x] The existing invariant holds unchanged: the phase change is
    committed before the rule fires, so a failing rule is logged as a
    `failed` run and **never undoes the operator's move**.
  - [x] **`milestone_due` trigger (date-arrival).** *(Landed
    2026-09-02.)* The one dated field with unambiguous "arrived"
    semantics is `milestones.due` — a task's own `due_at`/`start_at`/
    `finish_at` are each ambiguous about which one "the" date-arrival
    trigger means, so this ships narrowly on milestones rather than
    guessing a task-date convention. Unlike every other trigger, a
    milestone's due date does not arrive as a write anyone makes — there
    is no event to hang the rule on — so it needs its own **exactly-once
    claim**, not the existing fire-on-write path. New join table
    `automation_milestone_fires (automation_pid, milestone_pid)`
    (migration `m20260902_000002_automation_milestone_fires`) with a
    `UNIQUE (automation_pid, milestone_pid)` constraint claimed via
    `INSERT ... ON CONFLICT DO NOTHING`; a suppressed conflict surfaces
    from sea-orm's `exec_with_returning` as
    `DbErr::RecordNotFound("Failed to find inserted item")` — **verified
    live against a real Postgres**, not assumed from reading the crate
    source (which also has a `RecordNotInserted` variant, but that one
    belongs to a different insert code path and does not appear here).
    `POST /api/automations/milestones/sweep` (new
    `controllers::automation::sweep_milestone_due`) queries overdue,
    undone, non-deleted milestones (capped at `SWEEP_CAP`), and for each
    enabled `milestone_due` rule matching the milestone's plan attempts
    the claim before applying the rule's actions — so a rule/milestone
    pair fires **exactly once, ever**, not once per sweep. The optional
    scheduler ticker (`src/scheduler.rs`) now calls this sweep alongside
    the existing `sweep_due` on every tick; the endpoint still works
    standalone for a deployment driving both sweeps from external cron.
    Shares the new `apply_rule_actions` helper extracted from `fire()` so
    both the write-triggered path and the sweep log outcomes identically
    (multi-action, per-action logging, never undoing prior state).
    Verified live:
    `a_milestone_due_rule_fires_once_the_date_arrives_and_never_again`
    seeds one overdue and one far-future milestone, sweeps twice, and
    confirms `fired: 1` then `fired: 0, already_claimed: 1` — with
    exactly one `automation_runs` row throughout and the far-future
    milestone never appearing. Full DB-gated suite 76/76 (was 75, +1);
    `cargo test --lib` 360/360 (was 358, +2); `cargo fmt --check` /
    `cargo clippy --all-targets -D warnings` clean.
  - [ ] **Field-change and SLE-breach triggers — deliberately deferred.**
    Both need an owner decision this pass is not the place to guess:
    field-change needs a declared set of "which field(s) count" (every
    field on a `Plan`/`Task` payload firing a rule is a very different
    product from a curated allow-list, and the wrong default is hard to
    walk back once rules depend on it); SLE-breach needs a chosen SLE
    source (the workflow's own service-level expectation, §-derived from
    history per `tba.rs`, vs an operator-declared target) and a
    once-only notification schema so a breach does not re-fire every
    sweep. Same reasoning basis as the PRO-P33 controls-registration
    deferral: a mechanical implementation would have to invent the
    business decision rather than express one already made. Left open
    pending a product decision on both.
  - [x] **Multi-action rules** applied in declared order with per-action
    outcomes logged. *(Landed 2026-09-02.)* `automations.action_kind`/
    `action_value` (one action per rule) replaced by an `actions JSONB
    NOT NULL DEFAULT '[]'` array (migration
    `m20260902_000001_automation_multi_action`; a
    `CHECK (jsonb_array_length(actions) > 0)` refuses an empty list even
    from a direct insert), backfilling every existing single-action rule
    into a one-element array so nothing silently emptied. Array order
    **is** declared order — no separate position column. Pure
    `automation::validate_actions` (5 new unit tests) validates every
    element with the existing `validate_action` and names the offending
    0-based index on failure. `automation_runs` gained `action_index`
    (`DEFAULT 0`, so every pre-existing run stays correctly addressable
    as "action 0" with no backfill needed); `fire()` now loops the
    parsed action list and calls `record_run` once per action, so an
    N-action rule writes N run rows — a partial failure (action 2 of 3)
    is visible rather than overwriting or being swallowed by the next
    action's outcome. `act_assign`/`act_set_task_status`/`act_notify`/
    `act_schedule` were refactored to take one action's
    `kind`/`value` rather than the whole rule row, so nothing changed
    about *what* an action does, only how many a rule may declare.
    OpenAPI schema + its mounted-routes pinning test updated (also
    fixed a pre-existing, unrelated staleness found in the same block:
    the `trigger_kind` enum was missing `plan_phase_changed`, landed
    earlier in this same task but never reflected in the doc).
    Verified live against a fresh Postgres:
    `a_multi_action_rule_applies_every_action_in_order_and_logs_each_outcome`
    seeds a two-action rule (`add_label` on an already-labelled plan,
    then `assign`) and confirms the first action logs `skipped` while
    the second still applies — proving one action's non-fatal outcome
    does not block the next. Full DB-gated suite 75/75 (was 74, +1);
    `cargo test --lib` 358/358 (was 353, +5); `cargo fmt --check` /
    `cargo clippy --all-targets -D warnings` clean. **No back-compat
    shim**: `actions` is the only accepted shape on `POST
    /api/automations` — there is no front-end consumer yet (PRO-P20)
    and this service is pre-1.0 with synthetic data only, so a clean
    cut was chosen over carrying two request shapes.

- [x] **T-22 — Realized gains (§5.9.6 / FR-33).** **Landed 2026-08-26.**
  Pure `src/value.rs` (11 tests), migration `m20260826_000003_value`,
  four entities, `src/controllers/value.rs`.
  - **A plan with no value points is `unrealized`, never a total loss.**
  - `approved_at` has no update path, and a **second first-measurable
    value point is refused by a partial unique index** — the
    Time-to-Value clock stops once.
  - Time to Value is a **distribution** (nearest-rank p50/p85), never a
    mean.
  - Adoption refuses a zero denominator **at write**, and stores its own
    `definition` and `window_days` — "active user" is the term most
    easily redefined between two readings.
  - **Investment is actual cost, and mixed currencies withhold the ROI**
    rather than adding pounds to euros. This came from checking the
    `budget_lines` shape: it carries a currency *per line*, so a
    plan-level sum needed the single-currency rule enforced.
  - The evidence mix (`measured_share_basis_points`) is disclosed, so a
    realized-value figure says how much of itself was measured.

- [x] **T-23 — Strategic performance (FR-34 / FR-36), partial.**
  **Landed 2026-08-26.** `satisfaction_responses` + the six-dimension
  view skeleton.
  - **NPS always carries its response count** — 100 from two
    respondents is not a finding — and reports `null` with
    `no_responses` rather than a score of zero.
  - Responses store a **role, never an identity**.
  - **SPI and CPI report `null` with `no_baseline`, never `1.0`.** A
    plan without a phased budget baseline is *unmeasured*, not on plan.
  - **Not built:** the phased budget baseline itself, so SPI/CPI are
    permanently unmeasured today; NPV; Strategic Alignment Index;
    defect density. Each needs an input the service does not hold, and
    reporting them from absent inputs is what this row refuses to do.

- [x] **T-24 — Per-person utilisation (FR-35).** **Landed 2026-08-26.**
  `working_time_configs` + `non_working_periods`;
  `GET /api/capacity/utilization?by=plan|team|person`.
  - **The obligation-2 test is the load-bearing one:** somebody on leave
    for the whole window reports `null` with `all_non_working`, **never
    0%** — leave leaves the denominator rather than sitting in it, and
    0% would read as measured idleness. A person entirely on leave is
    still listed, so the answer is "on leave" rather than a silent
    absence.
  - No declared capacity is its own reason (`no_declared_capacity`),
    distinct from leave: the denominator is unknown, not zero.
  - Below the suppression floor the figure is withheld **with its inputs
    still returned**, so suppression is visible rather than looking like
    missing data.
  - Team utilisation **sums** the numerator and denominator; it is not a
    mean of individual ratios, which over unequal denominators is a
    different and wrong number.
  - At or over 100% is flagged as a **warning** — what a queueing system
    looks like just before it stops coping — and is not clamped.
  - Per-person cycle time, throughput and flow efficiency remain
    **absent from every endpoint**, and the capacity arithmetic is
    integer throughout (a float denominator would not reconcile against
    a payroll system).

- [ ] **T-25 — Total Project Control (§5.9.7 / FR-37).**
  - [x] Pure `src/tpc.rs`: DIPP, the progress index, banding, the
    stored-vs-computed divergence, and currency-scoped triage. Money in
    minor units, ratios in basis points, **no float**; 11 unit tests
    including the never-panic and sunk-cost pins.
  - [x] Migration `m20260825_000001_total_project_control` with
    `dipp_progress_index_ratio` as a Postgres **`GENERATED ALWAYS`**
    column (`NULLIF` on the denominator, so a zero baseline is `NULL`
    rather than an insert-time division error) and a `CHECK` refusing a
    negative cost-estimate-to-complete.
  - [x] SeaORM entity `models/_entities/total_project_control.rs`.
  - [x] Controller `src/controllers/tpc.rs` + the four routes (§9.2c),
    **verified mounted** via `cargo run -- routes`, and OpenAPI with a
    test pinning that the sunk-cost and never-infinity properties are
    documented rather than implied.
  - [x] Verified end to end against Postgres 18: the generated ratio
    computes (1.2), a zero baseline yields `NULL` rather than an insert
    error, **the ratio cannot be written by hand** (Postgres refuses a
    non-DEFAULT value), a negative CEC is refused by CHECK **and** by
    the handler (`422`), a negative EMV is accepted and bands
    `value_destroying`, and triage reports its exclusions.
  - [x] `#[ignore]`d request tests committed
    (`tests/requests/metrics_control.rs`), so the checks above run in CI
    rather than only in a terminal session.
  - **Acceptance:** `CEC = 0` reports `null` **with a reason**, never
    infinity; a negative EMV is **not** clamped and bands as
    `value_destroying`; the generated ratio cannot be written by the
    handler and cannot disagree with its numerator and denominator;
    triage sets aside a foreign currency and an undefined DIPP rather
    than ranking them as zero; two plans with equal remaining value and
    cost rank identically however much has been sunk into either.

- [ ] **T-26 — Controls / the Controlling process (§5.9.8 / FR-38,
  FR-39).**
  - [x] Pure `src/controls.rs`: the three timings and the response each
    permits, standard validation against the metrics the service
    actually produces, the four comparators, and the coverage rollup;
    11 unit tests.
  - [x] Migration `m20260825_000002_controls` — three tables, with
    CHECKs on `timing`, `comparator`, `verdict` and `kind`, and a
    `controls_within_needs_tolerance` constraint. The `verdict` CHECK
    is the load-bearing one: it keeps `unmeasured` a real third value,
    so a typo cannot fall through as "not a fail".
  - [x] Three SeaORM entities; controller `src/controllers/controls.rs`
    + eight routes (§9.2c), **verified mounted**; OpenAPI + pinning
    test.
  - [x] Verified end to end: an unknown metric is `422` at registration;
    a feedforward control reports `block` and a feedback control
    `record`; a failing reading derives verdict and gap at write; a
    valueless reading is `unmeasured`, **not** a pass, and is excluded
    from the pass rate; coverage names never-read controls and
    unanswered failures, and shows every timing at zero; an `accept`
    action clears the unanswered count.
  - [x] **Action → task conversion**, *(done 2026-09-02)*: `POST
    /api/actions/{pid}/convert` creates a task on the action's own
    control's plan, in the plan's workflow-initial state, carrying the
    action's description as the task title, and stamps
    `converted_task_pid` — mirroring `engineering::create_task`'s own
    transactional task+transition commit (spec `time-based-analysis.md`
    §5.1 invariant 3) so a converted task is analysable identically to a
    hand-created one. Refuses a second conversion of the same action and
    a conversion of a closed one (`422`), and a `404` on an unknown
    action — pinned by
    `tests/requests/metrics_control.rs::converting_a_control_action_creates_a_task_on_the_plan`.
    **Issue conversion is deliberately not implemented**:
    `converted_issue_pid` stays reserved, always `NULL`, until this
    service has an `issues` store of its own (FR-14, still deferred
    below) — the migration's own doc comment says actions convert into
    work stores that *already exist*, and issues do not yet.
  - [x] `#[ignore]`d request tests committed
    (`tests/requests/metrics_control.rs`; now 76 cases, +1 for the
    conversion endpoint above).
  - [ ] Register the controls that already exist in all but name — gate
    readiness (feedforward), WIP limits and the SLE (concurrent),
    retrospectives and the variance views (feedback) — so coverage
    reports reality rather than only newly-authored controls.
    **Investigated 2026-09-02, deliberately left open rather than
    guessed**: every one of these four *can* already be registered
    today through the plain `POST /plans/{pid}/controls` API — their
    metrics (`gate_readiness`, `work_in_progress`, `cycle_time_p85`,
    `budget_variance`) are already in `KNOWN_METRICS`, and `validate()`
    requires a metric name for every control regardless of
    `source_kind`, so nothing in the *code* actually blocks this. What
    is missing is a **decision this task cannot make unsupervised**:
    what `target_value`/`comparator`/`tolerance` each gets, and whether
    registration is auto-created (risky: a feedforward control's
    verdict can **block a write** — silently registering one with an
    invented threshold would be an unrequested behavioural change to
    every plan) or an opt-in per-plan action an operator takes. There
    is also no computable metric for **retrospectives** at all today
    (no `KNOWN_METRICS` entry reflects "a retrospective happened"),
    which would need its own design, not a registration. Needs an
    owner decision before code, not a guessed default.
  - **Acceptance:** a feedforward control may block a write and a
    feedback control may not; a control naming an unknown metric is
    refused at **write**, not left permanently `Unmeasured`; an
    unmeasured reading is excluded from the pass rate rather than
    counted either way, and an all-unmeasured control reports `null`
    rather than 0%; a failing reading with no action and no explicit
    `Accept` appears as **unanswered**; every timing appears in coverage
    even at zero; a disabled control is never reported as overdue.

- [x] **T-27 — Request tests for T-15/T-19/T-20/T-25/T-26.**
  **Landed 2026-08-25.** Fourteen `#[ignore]`d request tests in
  `tests/requests/workflow_phase.rs` and
  `tests/requests/metrics_control.rs`, taking the DB-gated suite from
  **47 to 61**. `scripts/ci-check.sh test-db` green.

  Written because those five features had been verified only by hand
  against a running service — a session nobody can re-run, which by
  §14.3 rule 1 is not a status claim at all. Each test is one check that
  was performed manually and now runs in CI, and the assertions are
  chosen for the *refusals*: undefined is not zero, unmeasured is not a
  pass, unclassified is not a feature, a skip names what it skipped.

  **It immediately found a real gap.** `tasks.flow_type` existed, the
  migration constrained it, and Flow Distribution read it — but
  **nothing could set it**: the field was never added to the task-create
  payload. The manual pass missed it precisely because that verification
  seeded rows with `psql` and only exercised the read path. Fixed:
  `flow_type` is now accepted and validated on create (`unclassified`
  deliberately not accepted as an input, since it is the *absence* of a
  declaration).

- [ ] **T-28 — PPM evaluation-criteria triage (2026-09-03).** A
  ten-criterion buyer's checklist for project portfolio management
  tools (strategic alignment, scenario modelling, resource management,
  reporting and analytics, financial management, integrations, task
  management, AI capabilities, usability, deployment effort — each with
  the question a buyer puts to a vendor) was triaged against what this
  trio carries **today**, by reading the mounted routes
  (`grep '\.add("' src/controllers/*.rs`), the entity columns, and the
  front-end route tree — not the roadmap. The table records, per
  criterion, what is already answered, what is not, and the
  disposition. Sub-tasks follow; the cross-subproject consequences
  (an enterprise IdP at the auth service, a webhook relay sink, a
  family go-live runbook) are in the repo root
  [`tasks.md`](../../tasks.md) Phase 10 and only pointed at from here.

  | Criterion | Already carried (route / module) | Not carried | Disposition |
  |---|---|---|---|
  | Strategic alignment | Smart Score with a disclosed per-component breakdown, `strategic_alignment` one of six weighted components (`src/prioritisation.rs`; `GET /prioritisation`, `GET /plans/{pid}/smart-score`); objectives registry + weighted `objective_links` + the OKR engine (T-16); gate reviews, lifecycle funnel, plan reviews with consensus | The Strategic Alignment Index (T-23: needs an input the service does not hold) | **Carried.** The buyer's question — "score and rank against custom objectives without a spreadsheet" — is answered by the objectives registry plus the env-tunable weights. Nothing new. |
  | Scenario modelling | `scenarios` are separate records; `GET /scenarios/{pid}/evaluate` reads live data without writing it; `GET /scenarios/compare` is the side-by-side; `POST …/commit` stamps funding | **Rollback** — a commit is one-way (`committed_at` only); an evaluation names no `as_of`, so two reads of one scenario a week apart differ silently | T-28a |
  | Resource management | `allocations` (person ref + role + percent + window); `GET /capacity` with `over_allocated`; `GET /capacity/utilization` (T-24); `GET /assignees/workload`; reassignment by largest slack | **Skill-based** allocation — an allocation carries a `role` string, no skills; no scale evidence for the "50+ concurrent projects" question | T-28c, T-28d |
  | Reporting and analytics | Persona surfaces already exist — `at-a-glance`, `executive/*`, `board/*`, `financials/*`, `technology/*`, `auditor/*`, `compliance/*`, `regulator/*`, `risk/heatmap`; saved `report_definitions` (filter + field projection) run on demand; Monte-Carlo delivery forecast | Nothing ties a persona surface to the **caller** — every user gets the whole nav; `report_definitions` has no `group_by` (PPM-9 promised one) and no scheduled run (PPM-9 says synchronous only, awaiting T-8) | T-28e, T-28f |
  | Financial management | `budget_lines` per plan; variance per currency (`insights::variance_by_currency`, `GET /financials/variance`); exposure; TPC with cost-estimate-to-complete; value realization with ROI (T-22) | **Cost forecasting** — no phased baseline, so SPI/CPI are permanently `no_baseline` (T-23) and there is no EAC/ETC or portfolio-level overrun forecast; actuals arrive by hand until T-8 lands | **T-28b** — the single highest-leverage gap in the table |
  | Integrations | Open API: hand-written OpenAPI 3 + Swagger UI, pinned two-way against mounted routes; inbound `POST /devops/events`; the durable outbox → relay (event bus); deterministic external ids for Jira / Asana / Trello / MS Project / GitHub / Linear | **Outbound** — the `notify` action is in-app only (no email, push, or webhook transport); no PM-tool import path (T-8 open, and no source-tool codec); two-way sync is roadmap only | T-28m (webhook sink, family-shaped), T-28n (import codec). **Refused:** a native-connector catalogue / no-code integration builder — the open API, bulk, and signed webhooks *are* the integration surface of a service like this one. |
  | Task management | Gantt (`/gantt`, `/plans/[pid]/schedule`); `plan_dependencies` with cycle refusal, critical path, and slipping-dependency violations (`src/visibility.rs`); the append-only transition log; multi-plan reviews + rollup | **Automatic reprioritisation when a deadline shifts** — exactly the field-change trigger T-21 deferred for want of a declared field set. This checklist supplies it: the plan timeframe and a milestone's due date are the two dates a shift is asked about | T-28g |
  | AI capabilities | Every derived figure here is deterministic and discloses its inputs: Smart Score components, forecast by seed, constraint ranking, aging WIP, the controls verdicts. The buyer's question — "which outputs are explainable and auditable vs black-box" — is answered *all of them, none* | No portfolio-**optimisation** recommendation (the evaluator scores a scenario a planner wrote; it proposes none); no **demand** forecast (the intake pipeline has arrival history nobody forecasts from); no assistant | T-28h, T-28i, T-28j. **Refused:** an LLM assistant *inside the service* — it would be the one output that could not disclose its inputs, in a service whose every other number does. If one is ever wanted it sits at the front-end BFF over the open API and cites the endpoint it read. |
  | Usability | Hamburger top-nav, 13 locales with parity tests, SVAR grid / Kanban / Gantt, Lily headless; `viewport` meta present | One `@media` rule in the whole app and two data-grid dependencies that are desktop-shaped — mobile is **unverified**, not absent; no per-user saved views (report definitions are shared); no role-specific UX (see reporting); no onboarding path beyond the README quick start | T-28k, T-28l, T-28p, and T-28f |
  | Deployment effort | Containerised (Podman, Debian slim, MUSL static), `compose.*.yaml`; SSO through the central auth service (magic link + PASETO v4.public, BFF so no token reaches the browser); every knob an env var, documented family-wide in `configuration.md` | **SAML / OIDC** federation to an enterprise IdP — mentioned nowhere in the family, and it belongs to the auth service, not here; data migration waits on T-8 + T-28n; no go-live runbook, and the one that matters most is the activation gate: `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` **defaults off** | T-28o; SAML/OIDC → root `tasks.md` EV-2 |

  Two things the triage did **not** change: the matchable / operational
  partition (nothing below is a matcher signal), and the measurement
  stance — absent evidence stays `null` with a reason, never zero, in
  every new figure.

  - [ ] **T-28a (M) — Scenario rollback and evaluation provenance.**
    `POST /scenarios/{pid}/rollback` restores each member's funding
    state to what the commit replaced (stored **at commit time** in a
    `scenario_commit_effects` row per member — the prior state is not
    reconstructable later), audited, and refused (`409`, naming the
    members) where a member's funding state has since been changed by
    any other path: report the divergence, never overwrite it.
    `evaluate` and `compare` gain an `as_of` and a list of the live
    inputs read (budget lines, allocations, scores) with their
    `updated_at`, so two evaluations that disagree say why.
    **Acceptance:** commit → rollback → commit is idempotent on funding
    state; a member changed between commit and rollback blocks the
    rollback and is named; every evaluation response carries `as_of`.
  - [ ] **T-28b (L) — Phased budget baseline → SPI / CPI / EAC and the
    portfolio overrun forecast.** A `budget_baselines` table: planned
    cost per period per plan, in one currency, **frozen at approval**;
    a re-baseline is a new version with a reason, append-only, and
    every derived figure names the baseline version it used. Unblocks
    the SPI/CPI T-23 left `no_baseline`; adds `GET
    /plans/{pid}/financials/forecast` (EAC = actuals + ETC, with ETC
    from the TPC cost-estimate-to-complete where recorded, else from
    the baseline's remaining periods — and the response says which) and
    `GET /financials/forecast` rolled over `parent_ref` **per
    currency**, mixed currencies withheld exactly as T-22's ROI is.
    Actuals still arrive by hand or by T-8 bulk; this task does not
    build a finance connector.
    **Acceptance:** a plan without a baseline reports `null` +
    `no_baseline` unchanged; a re-baseline preserves its predecessor and
    the old figure is reproducible from the old version; the rollup
    over a subtree with two currencies reports two rows, never one sum;
    integer minor units throughout, no float.
  - [ ] **T-28c (M) — Skill-aware allocation.** An allocation may
    declare `skills_required[]` (short tags); the capacity view
    resolves a person's skills through the worker service by
    `EntityRef` — lazy verify-on-read, cached with a TTL, **never copied
    into any stored row** (people stay references, family doctrine) —
    and reports a per-plan skill gap. A person whose worker record is
    unreachable reports `unknown`, never "lacks the skill".
    **Acceptance:** no skill text lands in `allocations` beyond the
    requirement tags; a stubbed worker service returning `404` yields
    `unknown` with a reason; the gap finding names the tag and the
    plan.
  - [ ] **T-28d (S) — Capacity at scale.** A DB-gated test seeds 60
    plans with allocations across 40 shared people and asserts
    `GET /capacity`, `GET /capacity/utilization`, and `GET /at-a-glance`
    each complete in a **bounded query count** (asserted through the
    connection's statement log, not timed), plus a Criterion bench over
    the pure rollups. **Acceptance:** query count does not grow with
    plan count; the bench compiles under the `bench` CI stage.
  - [ ] **T-28e (M) — Report grouping and scheduled runs.**
    `report_definitions.group_by` (one field, counts + the money
    columns summed per currency); scheduled runs as a loco `worker`
    job writing a bulk artifact with the family TTL posture. The
    scheduled half **depends on T-8** (the artifact store and job table
    do not exist here yet) and lands second. **Acceptance:** a grouped
    run over two currencies never sums across them; a scheduled run is
    audited like an export (even at zero rows).
  - [ ] **T-28f (M) — Role-tailored navigation and landing page.** The
    front-end reads the `attrs` the BFF already gets from `/whoami` and
    orders the nav / picks the landing view from a deployment-declared
    attribute (e.g. `view=executive|pmo|resource_manager`) — a
    vocabulary the deployment declares, not an enum this code owns.
    **Default is today's full nav**, so a token with no such attribute
    changes nothing. This is presentation only; authorisation stays
    with the service's ABAC. **Acceptance:** attrs absent ⇒ identical
    nav; `view=executive` lands on `/executive`; every route stays
    reachable by URL regardless of the attribute.
  - [ ] **T-28g (M) — Deadline-shift trigger and rescheduling.** Two
    narrow field-change triggers, the way `milestone_due` was narrowed
    rather than guessing a task-date convention: `plan_timeframe_changed`
    and `milestone_due_changed`. One new action, `propose_reschedule`:
    walks `plan_dependencies` from the shifted item, computes each
    successor's implied new dates through the edge lag, and writes a
    **notification carrying the proposed shifts** — it moves nothing.
    An opt-in `shift_dependents` action applies them, one logged
    `automation_runs` row per task moved. The FR-32 invariants hold:
    a failing action never undoes the deadline edit, applied shifts do
    not re-enter the engine (no cascade), every firing is logged.
    **Acceptance:** shifting a plan's end by 5 days proposes +5 on a
    finish-start successor and +5 − lag where a lag exists; a
    successor whose dependency is already violated is proposed with the
    violation named; the shifted successors fire no further rule.
  - [ ] **T-28h (M) — Deterministic scenario generator.**
    `POST /scenarios/generate` takes the same constraints a scenario
    holds (budget cap, currency, must-include) and returns a **draft
    scenario** whose members were chosen greedily by Smart Score per
    unit cost, with a **rationale row per candidate** — included with
    its score and cost, or excluded with its reason (`over_cap`,
    `no_score`, `foreign_currency`, `must_include_conflict`). It is
    saved as an ordinary scenario, so it goes through the same
    evaluate / compare / commit path as one a planner wrote. A
    candidate without a score is listed **unranked**, never scored
    zero. **Acceptance:** same inputs ⇒ byte-identical output; every
    candidate appears exactly once across included + excluded; a
    must-include that alone exceeds the cap is reported, not silently
    dropped.
  - [ ] **T-28i (S) — Demand forecast.** Reuse the throughput
    Monte-Carlo behind `GET /plans/{pid}/forecast` over the intake
    pipeline: proposal arrivals and approvals per period from
    `proposals` timestamps, answering "how many approved proposals in
    the next N periods" with the same seed determinism and the same
    refusal below the minimum history. **Acceptance:** fewer than the
    minimum periods ⇒ `null` + `insufficient_history`; the response
    names the history window it drew from.
  - [ ] **T-28j (S) — Explainability pin, and the non-goal recorded.**
    A test that walks every derived `GET` in the OpenAPI document and
    asserts the response carries either an inputs/reasons block or a
    `null` with a reason — the "no black-box output" property this
    entity already has, made unbreakable rather than habitual. And a
    §2.3 amendment recording the refusal above: no model-driven
    assistant inside the service. **Acceptance:** the test enumerates
    routes from `openapi.rs`, so a new derived route without disclosure
    fails CI.
  - [ ] **T-28k (M) — Responsive audit at a phone viewport.** Playwright
    (the existing API-stubbed e2e harness) at 390 × 844 across all 34
    routes: no horizontal body scroll, the primary action of each page
    reachable, and the two SVAR surfaces (grid, Gantt) degrading to a
    read-only list rather than an unusable grid. **Acceptance:** the
    e2e suite runs the mobile project in CI; each failing route is a
    named test, not a screenshot.
  - [ ] **T-28l (S) — Per-user saved views.** A `saved_views` table
    keyed by the token `sub` (route + filter + sort + columns; no other
    identity), served through the BFF so the browser holds nothing.
    **Acceptance:** two users on one route see their own views; a view
    is scoped to its route and never applied elsewhere.
  - [ ] **T-28m (M) — Outbound webhooks as a relay sink.** Not a new
    `notify` transport: a `WebhookSink` beside `LoggingSink` /
    `FluvioSink` in `src/relay.rs`, delivering the outbox envelope to
    configured URLs, **signed** with an HMAC over the body through the
    shared `integrity-mac` crate under its own HKDF domain (`webhook`),
    with a per-URL event-kind filter, retry with backoff, and a
    delivery log. Family-shaped so the other nine registries copy it
    (root `tasks.md` EV-3 carries the contract); portfolio is the first
    adopter. HTTPS-only outside loopback, no redirects followed
    (security invariant 7). **Acceptance:** a receiver verifies the
    signature with the published pre-image format; a 5xx is retried
    and a 4xx is not; a URL configured without the feature refuses to
    start rather than silently logging.
  - [ ] **T-28n (M) — Source-tool import codec.** Depends on **T-8**.
    A Jira project export (and Asana's, the two most asked about)
    mapped to plans + tasks: the project key → `JiraProjectKey` so the
    plan deduplicates against its registry twin (R-0), issues → tasks
    with the workflow's initial state unless the status maps, and every
    unmapped field reported in the per-row error report rather than
    dropped. **Acceptance:** re-importing the same export is idempotent
    (upsert by the deterministic id); an unmapped status lands the task
    in the initial state **and** names it in the report.
  - [ ] **T-28o (S) — Go-live runbook.** Depends on root `tasks.md`
    EV-4 for the family shape. Portfolio's own page leads with the
    activation gate — `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`
    defaults **off**, and a deployment reachable by untrusted callers
    must set it and mount an ABAC policy before it is reachable — then
    the PASETO keys URL, event transport, the optional scheduler ticker
    and flow-gauge loop, and who owns each knob (the deploying
    operator; there is no vendor-side configuration). **Acceptance:**
    the runbook is verified against a fresh container, not read.
  - [ ] **T-28p (S) — Operator onboarding guide.** A role-by-role
    "first hour" walkthrough (executive, PMO, resource manager) in the
    front-end docs, each step naming the route it lands on. It makes
    **no time-to-productivity claim** until one is measured.
    **Acceptance:** every route named in the guide exists (a test
    walks the guide's links against the route tree).

  **Suggested order:** T-28b first (it unblocks T-23's SPI/CPI and is
  the buyer question with the most weight); then T-28g and T-28a
  (both close a "deliberately deferred" note with a decision this
  checklist supplied); T-28j early because it is cheap and guards
  everything after it; T-28e / T-28n / T-28o wait on their
  dependencies.

- [ ] **T-1 — Scaffold the trio.**
  - [ ] Create `project-portfolio-management-matcher-rust-crate/`,
    `project-portfolio-management-service-with-loco/`, and
    `project-portfolio-management-front-end-with-svelte/` from the care-pathway / plan
    siblings (copy-adapt; drift accepted — repo decision 2026-06-02).
  - [ ] Each subproject ships its own `spec/` (matcher §1–§25; service
    + front-end §1–§18) referencing this entity spec's §5 as the
    canonical domain model rather than redefining it.
  - [ ] Add the entity `agents/` reference set (`index.md`,
    `models.md`, `matching.md`, `restful.md`, `testing.md`,
    `subprojects.md`, `spec-driven-development.md`).
  - [ ] Register the trio in the root `AGENTS.md`, `agents/share/overview.md`,
    and the front-end table.
  - **Acceptance:** every link in this entity spec resolves to a real
    file or section.
- [ ] **T-2 — Matcher crate: domain model + matching.**
  - [ ] The canonical `Plan` type + optional `PlanKind` label +
    `Goal` + all enums (§5.1–§5.4), serde round-trip, NFKC-folding
    diacritic-preserving normalisation.
  - [ ] **Kind-agnostic matching**: `kind` is an optional descriptive
    label that neither gates nor scores; any two plans may match
    regardless of their labels (`MatchBreakdown.kind_gate_blocked` is a
    vestigial always-`false` field).
  - [ ] Deterministic short-circuits: R-0 (each deterministic
    identifier scheme — `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
    `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`;
    owner-scoped `Code`/`LocalId`/`Custom` excluded), R-1 (same
    `owner_org_id` + equal normalised `code`), R-2 (`same_as`
    overlap) → 1.0.
  - [ ] Probabilistic components + weights per §6.8 (name JW + Soundex
    bonus, goal-title Jaccard, code, owner org, parent-plan exact,
    timeframe date-proximity, keywords, relationships typed-set
    Jaccard, tags), renormalised over present components; `status`
    and the optional `kind` label never scored; presets
    strict/default/lenient.
  - **Acceptance:** `cargo test` green; the FR-8 weight table sums to
    1.00 in a test; kind-agnostic matching, each rule, and each
    component have a unit test; public-API + doctest suites pass.
- [ ] **T-3 — Service crate: chassis + thin-record CRUD + matching.**
  - [ ] loco 0.16 / Axum 0.8 / SeaORM 1.1 chassis; the `plans`
    migration (nullable `kind`, `parent_pid`); `cargo loco start`;
    config yamls; port 5150.
  - [ ] The controller over the one plans collection: CRUD over the
    thin `Plan` (JSONB `data`), name search (`ILIKE`), `/match`,
    `/check-duplicates` (all kind-agnostic), validation (§FR-1a incl.
    `parent_ref` shape + containment-cycle rejection).
  - **Acceptance:** DB-free matcher-embedding + JSON round-trip tests
    green; blank-name / malformed-`EntityRef` / malformed-`parent_ref` /
    containment-cycle / malformed-deterministic-id → `422`.
- [ ] **T-4 — Service crate: operational sub-resources + derived views.**
  - [ ] Tables + CRUD for tasks, issues (keyed by `parent_pid`); goals
    via `data.goals[]` mutation (the §5.3 bridge).
  - [ ] Derived `timeline` + `burndown` read endpoints; `task_snapshots`
    feeding burndown.
  - [ ] Real-time `409` duplicate detection on create; record merge
    (any two plans) that **re-homes sub-resources** to the survivor;
    the child roll-up (`?parent=` filter + `parent_pid` column).
  - **Acceptance:** DB-gated request tests cover sub-resource CRUD,
    the goals bridge (a goal write changes a subsequent match score),
    `409` on create, merge re-homing, and the roll-up filter; the
    partition test (no sub-resource field in any `data`; a non-goal
    sub-resource write does not change the match score) passes.
- [ ] **T-5 — Auditability + security.**
  - [ ] `audit_logs` (plan + sub-resource actions) + read endpoints;
    in-memory `PlanEvent` stream + `…/events/recent`.
  - [ ] Offline PASETO v4 public verification (`authentication-verifier`);
    switch `src/auth.rs` per
    [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
    (supersedes the RS256-JWT model). `AuthUser`/`MaybeAuthUser`; `whoami`
    protected; audit / merge `actor` stamped from the token.
  - **Acceptance:** create + update + delete a plan and a task →
    audit rows + events read back; no token → `401`, valid token →
    `2xx`.
  - [x] *Follow-up (delivered):* blanket `/api/*` enforcement +
    paseto-keys-over-HTTP fetch + **ABAC** write authorisation over
    the token's `attrs` claim (per
    [`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md);
    supersedes the earlier role-based sketch). Default-off via
    `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`; activation awaits the coordinated family
    SSO rollout (the front-end must attach the bearer token first).
- [ ] **T-6 — Front-end: routes + sub-resource workspaces + tests.**
  - [ ] The `/plans`, `/plans/new`, `/plans/[pid]`,
    `/plans/[pid]/edit` routes over the thin record; sub-resource
    workspaces (`…/[pid]/{goals,tasks,issues}`) and derived views
    (`…/[pid]/{timeline,burndown}`); the child roll-up.
  - [ ] vitest units (`ApiClient` + `PlanRepository`) +
    Playwright smoke; `pnpm run check` strict 0/0 + production build.
  - **Acceptance:** both suites green; a contract-drift in any
    endpoint path fails a test.
- [ ] **T-7 — Cross-service links (write-side).**
  See §9.5 and
  [cross-service-linking.md](../../agents/share/cross-service-linking.md).
  - [ ] `entity_links` migration (`UNIQUE (from_pid, kind, to_ref,
    valid_from)`); `POST`/`GET`/`DELETE …/{pid}/links`; `linked` /
    `unlinked` events; the `EntityRef` value type (copied per project).
  - [ ] The partition rule (§7 there): links are never stored in
    `relationships` and never fed to the matcher.
  - **Acceptance:** a link create emits `linked`, a delete emits
    `unlinked`, and a test asserts no link ever reaches the matcher.
- [ ] **T-8 — Bulk import / export.**
  See §9.6, §10.4 and
  [bulk import/export](../../agents/share/bulk-import-export.md).
  - [ ] `bulk_jobs` migration (shared doc §3 schema, with
    `UNIQUE (entity, kind, idempotency_key)`; `entity` is the one
    `plans` collection).
  - [ ] The five endpoints on the plans collection (§9.6); `bg_pg`
    worker draining `queued → running → completed |
    completed_with_errors | failed`.
  - [ ] JSONL (lossless reference) + CSV (flattening per §9.6: every
    repeated / nested field a JSON-in-cell) codecs; Parquet
    **export-only**, feature-gated.
  - [ ] Per-row pipeline reusing the single-create validators +
    matcher + review queue: upsert by stable key (deterministic
    external-PM identifier, `(owner_org_id, code)`, or `pid`, §9.6);
    keyless / unmatched rows → duplicate detection → review queue with
    `provenance = import`; events + audit not bypassed.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`); one bad row
    never aborts the load; counts reconcile.
  - [ ] Export masking + audit: `masking_profile` (masked default —
    people references hidden; full gated), `include_soft_deleted`
    gated, every export audited (even zero-row).
  - **Acceptance:** integration tests cover idempotent re-import,
    the per-row error report, a keyless dedupe-to-review row
    (`provenance = import`), masked vs full export of a plan with
    people references, and that a zero-row export still writes an
    audit record.
- [ ] **T-9 — OpenAPI / Swagger + richer validation.**
  - [ ] OpenAPI 3 schema (hand-written, dependency-light, same
    approach as the organization / care-pathway services) + Swagger UI
    at `/api-docs/openapi.json` · `/swagger-ui`.
  - [ ] Validation of deterministic identifier shapes (UUID,
    external-PM-id patterns), `EntityRef` syntax, `parent_ref` (a valid
    plan `pid`, no containment cycle), `in_language` (BCP-47), and
    relationship integrity (no self-reference, inverse-consistency,
    acyclicity — §5.8); `422` on failure.
  - **Acceptance:** Swagger UI serves the documented endpoints; a
    malformed-identifier / self-referencing-relationship /
    containment-cycle test returns `422`.
- [x] **T-10 — Unify the four work-item kinds into one recursive
  `Plan`.** ✅ *Delivered 2026-07-20 (built + tested green across
  matcher, service, and front-end).* The former Portfolio / Project /
  Product / Program "work item" kinds were collapsed into **one
  recursive entity**, a **plan**:
  - The matcher type is renamed `WorkItem` → `Plan`
    (`PlanKind`/`PlanIdentifier`/`PlanRelationship`/`PlanStatus`);
    `kind` becomes `Option<PlanKind>` — an **optional descriptive
    label** that no longer gates or scores. `Plan::new(name)` defaults
    `kind` to `None`.
  - The hard **kind gate (R-GATE)** is **removed**: any two plans may
    match regardless of their labels. `MatchBreakdown.kind_gate_blocked`
    is kept as a vestigial, always-`false` field for wire compatibility.
  - The four `/api/{portfolios,projects,products,programs}` REST
    collections collapse into one **`/api/plans`** collection; the four
    per-kind tables collapse into one **`plans`** table (nullable
    `kind`, nullable `parent_pid`).
  - Containment becomes **recursive**: any plan may contain any other
    plan via `parent_ref` (renamed from `portfolio_ref`); a `parent_ref`
    forming a containment cycle is rejected `422`. Sub-resource tables
    are re-keyed by `parent_pid` (not `(parent_kind, parent_pid)`).
    Merge is no longer kind-scoped.
  - **Acceptance:** the matcher matches two plans with differing `kind`
    labels; the single `/api/plans` CRUD + match + merge suite is green;
    the containment-cycle `422` and the child roll-up (`?parent=`) pass;
    `cargo test` / `clippy` / `fmt` clean and the front-end `pnpm run
    check` + build clean.

- [x] **T-14 — Time-based analysis (TBA-1 … TBA-7).** The time dimension
  of delivery: a durable, append-only **task transition log**, and the
  derived per-task / plan / constraint / aging-WIP / flow views.
  Unifies Barker's time-based analysis (the value-adding ratio), value
  stream mapping (the VA / NNVA / UNVA classification and the LT / PT /
  %A / #HO / RFPY metric names) and queueing theory (λ / μ / ρ / κ / τ,
  Little's Law). Full contract, including the parts deliberately
  refused, in the cross-cutting
  [`time-based-analysis.md`](time-based-analysis.md).
  - **Done (2026-08-23):** implemented in the service crate —
    `migration/src/m20260823_000001_time_based_analysis.rs`,
    `src/models/_entities/task_transitions.rs`, the pure `src/tba.rs`,
    the transition writes inside `create_task` / `move_task`,
    `src/controllers/tba.rs`, routes, OpenAPI, and
    `tests/requests/tba.rs`.
  - **Acceptance (met):** creating a task opens its log and each
    accepted move appends exactly one transition, while a **no-op** move
    and a **refused** move append none — the log records moves that
    happened, not requests that were made; cycle time and lead time are
    both returned and never one labelled as the other; statuses and
    categories each sum to the lead time over a generated sweep;
    an item that started and finished inside one millisecond reports a
    zero cycle time rather than "never started"; a transition in the
    future does not retroactively start the clock; the service level
    expectation is refused below its minimum sample rather than computed
    from noise; the new paths are `401` under
    `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`. 44/44 `--ignored`
    request tests green vs Postgres 18; 230 unit tests; clippy pedantic
    clean.
  - **Open (TBA-8 … TBA-11):** the front-end cumulative-flow diagram and
    aging-WIP board, the cross-plan rollup over `parent_ref`, Prometheus
    gauges, and Monte-Carlo delivery forecasting from the cycle-time
    distribution.

