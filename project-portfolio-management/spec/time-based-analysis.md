# Time-based analysis (TBA) — living specification

> **Source of truth for the portfolio time dimension.** This document is
> the canonical artefact for time-based analysis across the
> project-portfolio-management trio: what is measured, how each figure
> is defined, what the API returns, how it is stored, and what it
> deliberately refuses to do. It is a *cross-cutting* section of the
> [portfolio entity spec](index.md) rather than a numbered chapter,
> because it spans the domain model (§5), the API surface (§9),
> persistence (§10) and compliance (§12) at once.
>
> **Family contract.** The measurement model is fixed in
> [`agents/share/time-based-analysis.md`](../../agents/share/time-based-analysis.md);
> this document is the portfolio *adoption* of it. Where the two
> disagree, the family contract wins on the model and this spec wins on
> anything portfolio-specific.
>
> **Sibling.** The [care-pathway trio carries the same
> section](../../care-pathway/spec/time-based-analysis.md), tuned to a
> patient journey. The measurement model is deliberately the same — a
> work item queueing between the hands that touch it is one problem —
> and the differences are stated in §2.5 rather than left implicit.
>
> **Three-part PRs.** A behavioural change here is one PR: spec edit +
> code edit + test edit. See
> [`agents/spec-driven-development.md`](../agents/spec-driven-development.md).

## Table of contents

1. [Purpose and vision](#1-purpose-and-vision)
2. [Research basis](#2-research-basis)
3. [Scope and non-goals](#3-scope-and-non-goals)
4. [Glossary](#4-glossary)
5. [Domain model](#5-domain-model)
6. [The measurement model](#6-the-measurement-model)
7. [Cohort statistics and the service level expectation](#7-cohort-statistics-and-the-service-level-expectation)
8. [Constraint analysis](#8-constraint-analysis)
9. [Flow analysis (queueing theory)](#9-flow-analysis-queueing-theory)
10. [API surface](#10-api-surface)
11. [Persistence](#11-persistence)
12. [Governance, honesty, and anti-gaming](#12-governance-honesty-and-anti-gaming)
13. [Non-functional requirements](#13-non-functional-requirements)
14. [Testing strategy](#14-testing-strategy)
15. [Tasks](#15-tasks)
16. [Implementation status](#16-implementation-status)
17. [Open questions](#17-open-questions)
18. [References](#18-references)

---

## 1. Purpose and vision

Time-based analysis evaluates delivery by measuring **elapsed calendar
time through the board**, rather than by activity, utilisation, or
points completed. It asks one question of every work item: *of the time
this took, how much was somebody actually working on it?*

The answer is reliably small. Flow-efficiency measurements across
knowledge work typically land between **5% and 15%** — the same order as
Dr. R. C. Barker's NHS finding that value-adding time is **8–14%** of a
patient journey. The remaining 85–95% is queueing: sitting in `todo`,
waiting for review, blocked on someone else, waiting for a decision.

This matters because it inverts the usual improvement instinct. If an
item is worked on for 6% of its life, then **making the work 20% faster
improves delivery by about 1%**, while removing half the waiting
improves it by nearly half. Teams that measure only velocity, utilisation
or story points cannot see this, because all three measure the 6%.

The portfolio service is unusually well placed to carry the measurement.
It already owns the plan, the task, the Kanban status, WIP limits, and —
critically — a status-change API. What it lacked was **history**: `tasks`
carried only `status_changed_at` (the *current* status's start) and
`done_at`. The moment a task moved twice, the first interval was gone.
TBA adds the missing primitive — a durable **transition** log — and
derives everything else from it.

**Vision.** A team lead opens the plan and sees: flow efficiency 7%, the
p85 cycle time (so "we usually finish within 11 days" becomes a number
with a confidence attached), the three items aging past it right now,
which column the time actually accumulates in, and how often work moves
*backwards*. No extra data entry: moving a card on the board is already
an API call, so the measurement is a by-product of the work.

### 1.1 Design goals

| Goal | Meaning |
|---|---|
| **Free at the point of collection** | Transitions are recorded by the existing status-change endpoint. A method that asks engineers to log hours will get logged hours, not true ones. |
| **Calendar time is the denominator** | Flow efficiency is work time over elapsed time, never over the sum of recorded activity (§6.3). |
| **Cycle and lead time are different numbers** | Conflating them is the most common measurement error in the field, and it always flatters (§6.1). |
| **Percentiles, not averages** | Cycle-time distributions are long-tailed. "Average 5 days" is a promise nobody can keep; "85% within 11 days" is one you can (§7.1). |
| **Names the constraint** | Output ranks *where the time goes*, ordered by time recoverable, not a score. |
| **Never a person metric** | Flow is a property of the system. §12.4 states why turning it on individuals destroys the data. |
| **Standard vocabulary** | VSM terms (VA / NNVA / UNVA, LT, PT, %A, #HO, RFPY) and queueing symbols (λ, μ, ρ, τ, ω, φ, κ) as defined upstream. |

## 2. Research basis

### 2.1 Barker's time-based analysis

Bob Barker's *The Time Based Organisation* method tracks a unit of work
end to end and records, for every phase, whether it was value-adding
"touch time". Measured across sectors, value-adding time is typically
**under 15%** of elapsed calendar time. The method commitments that
carry across to a delivery board:

- **The whole journey, or nothing.** Optimising a step that is 3% of the
  elapsed time cannot move the elapsed time. Barker calls the
  alternative *islands of efficiency* — and a Kanban board with a fast
  `in_progress` column and a two-week `in_review` queue is exactly one.
- **The people doing the work record the times.** Here that is literal:
  the transition log is written by the same call that moves the card.
- **Untapped capacity, not more capacity.** If 90% of elapsed time is
  queueing, the team already contains the headroom; the constraint is
  sequencing and WIP, not effort.

### 2.2 Value stream mapping (VSM)

VSM supplies the **classification and the metric names**. See
[value-stream-mapping](https://github.com/joelparkerhenderson/value-stream-mapping).

- **VA — value adding**: the customer would recognise it as the product
  being built.
- **NNVA — necessary non-value adding**: required but not building —
  review, sign-off, compliance evidence, release process.
- **UNVA — unnecessary non-value adding**: queueing, blocking, rework.
- **Metrics**: Value Time (VT), Process Time (PT), Lead Time (LT),
  Percentage Activity (%A = PT/LT), Number of Handoffs (#HO), **Rolled
  First Pass Yield (RFPY)**, Percent Complete & Accurate (%C&A).
- **The eight wastes**: waiting, transportation, motion, over-processing,
  defects, inventory, overproduction, underutilised people.

Two of these earn their keep here in a way they do not on a clinical
pathway. **RFPY** — the share of items that reached `done` without ever
moving backwards — is directly computable from the transition log, and
is the honest measure of whether "done" means done. And the VSM waste
**inventory** is exactly what a large `todo` column is: work bought and
not yet used, aging while it waits.

The VA/NNVA/UNVA split matters because a blanket "non-value-adding"
number invites the true reply *"code review is not waste"*. It is not.
Separating NNVA from UNVA concedes that and still names the queueing
that is genuinely recoverable.

### 2.3 Queueing theory

Queueing theory supplies the **flow mathematics**. See
[queueing-theory](https://github.com/joelparkerhenderson/queueing-theory).

- λ arrival rate, μ service rate, ρ = λ/μ utilisation
- τ lead time, ω wait time, φ work time, θ step time, κ item count
- **Little's Law**: κ = λτ — average items in the system equals arrival
  rate × time in system. On a board: **WIP = throughput × cycle time**.

Three consequences shape this spec:

1. **WIP is the lever you actually control.** Rearranged, cycle time =
   WIP / throughput. Throughput is hard to raise; WIP is a decision. This
   is why the service already enforces WIP limits, and why §9 reports
   WIP against those limits.
2. **Wait time explodes near saturation.** As ρ → 1, expected wait grows
   without bound. A team at 95% utilisation is not "5% from trouble"; it
   is already in it, which is why full utilisation and long queues are
   the same observation.
3. **Little's Law needs a stable system.** It holds over a period long
   enough for arrivals and departures to balance. §9.4 uses it as a
   *consistency check* and says so, rather than as a forecast.

### 2.4 Flow metrics as the delivery community states them

The Kanban/flow literature converges on four measures, all of which fall
out of the transition log:

| Measure | Definition here |
|---|---|
| **Cycle time** | Started → finished, per item. Reported as a distribution. |
| **Throughput** | Items finished per unit time. |
| **Work in progress** | Items started and not finished. |
| **Work item age** | For an item still open: how long since it started. The only one of the four that is *actionable today* — the other three are history. |

Plus the **service level expectation (SLE)**: a forecast of the form
"85% of items finish within N days", where N is the p85 of the team's own
cycle-time history. This is the portfolio analogue of a clinical access
standard, and it is strictly better in one respect: it is **derived from
the team's own data** rather than imposed, so it cannot be argued with on
grounds of local difficulty (§7.3).

Deliberately *not* adopted: velocity and utilisation as improvement
targets. Both measure the 6% (§1), both are trivially inflated, and
neither is a time.

### 2.5 What differs from the care-pathway sibling

The measurement model is shared; four things differ, and stating them
keeps the two from being wrongly unified later:

| | care-pathway | portfolio |
|---|---|---|
| Segment source | **Manually recorded** intervals — nobody logs a clinic's minutes automatically | **Derived from status transitions** — the board move is already an API call, so history is free |
| Classification | Recorded per segment by the person mapping the journey | Derived from the **status** by a disclosed, deployment-overridable map (§5.3) |
| Threshold | External **NHS access standards** (RTT 18 weeks, 62-day cancer, …) | The team's **own p85 cycle time** as an SLE, plus optional explicit targets |
| Rework | Not modelled | **RFPY** from backwards transitions — a first-class finding |

## 3. Scope and non-goals

### In scope

- A durable **`task_transitions`** log, written by the existing task
  create and move endpoints.
- A disclosed **status → VSM category** map, overridable per deployment.
- **Per-task analysis**: lead time, cycle time, work time, flow
  efficiency, time per status, blocked time, handoffs, rework.
- **Plan-level analysis**: cycle-time percentiles, throughput, WIP,
  flow efficiency, RFPY, and the SLE.
- **Aging WIP**: open items ranked by age against the SLE — the
  actionable view.
- **Constraint ranking**: statuses ordered by time recoverable.
- **Flow analysis**: λ, μ, ρ, κ, τ and a Little's-Law consistency check.

### Out of scope (and why)

| Not doing | Why |
|---|---|
| Time tracking / timesheets | Self-reported effort is the data quality problem this design exists to avoid. Transitions are observed, not attested. |
| Estimation, velocity forecasting, Monte Carlo | Worth having, a different feature, and dependent on this one landing first (§17). |
| Per-person throughput or cycle time | §12.4. Not an oversight. |
| Backfilling history before the transition log | There is nothing to backfill from: `tasks` kept only the *current* status's start. The migration seeds one synthetic transition per live task and labels it `backfilled` so nobody reads an invented history as observed (§5.4). |
| DORA metrics | Already served by `/api/devops/metrics` from `devops_events`. TBA cross-references it rather than recomputing it. |
| Calendar/working-hours arithmetic | An item that sits over a weekend really did sit over a weekend. Business-hours discounting is exactly the kind of adjustment that makes queues invisible (§12.3). |

## 4. Glossary

| Term | Symbol | Definition here |
|---|---|---|
| Transition | — | A recorded status change on one task: `from` → `to` at an instant. |
| Interval | — | The stretch between consecutive transitions: time spent in one status. |
| Lead time | LT / τ | Created → finished. What the requester experiences. |
| Cycle time | — | First **started** transition → finished. What the team controls. Always ≤ lead time. |
| Work time | VT / φ | Time in value-adding statuses. |
| Process time | PT | Time in value-adding **+** necessary statuses. |
| Wait time | ω | Elapsed − process time. |
| Flow efficiency | %A | Work time / cycle time. The headline 5–15% ratio. |
| Blocked time | — | Time in a status classified `blocked`. Reported separately, being the most directly actionable waste. |
| Work item age | — | For an open item: now − started. |
| Throughput | μ | Items finished per day over a window. |
| Work in progress | κ | Items started and not finished. |
| Arrival rate | λ | Items created per day over a window. |
| Utilisation | ρ | λ / μ; also WIP against the configured WIP limit. |
| SLE | — | Service level expectation: "p% of items finish within N days", N from the team's own history. |
| Rework | — | A backwards transition — toward the start of the board. |
| RFPY | — | Rolled first pass yield: the share of finished items with no backwards transition. |
| Handoff | #HO | A change of `assignee_ref` across a task's life. |

## 5. Domain model

### 5.1 The transition

```
task_transitions
  pid           UUID        public identifier
  task_pid      UUID        the task
  plan_pid      UUID        denormalised, so a plan-wide read is one query
  from_status   TEXT NULL   NULL = the task's creation
  to_status     TEXT        one of the Kanban statuses
  at            TIMESTAMPTZ when it happened
  actor_ref     TEXT NULL   who moved it (audit actor)
  assignee_ref  TEXT NULL   who it was assigned to at that moment
  backfilled    BOOLEAN     synthesised by the migration, not observed (§5.4)
```

Invariants:

1. `to_status` is one of the board's statuses; `from_status` is too when
   present. Unknown statuses are refused, never coerced.
2. A transition is **append-only**. Correcting history means moving the
   card, which writes another transition. There is no edit endpoint,
   because an editable flow log measures whatever the editor wanted.
3. Transitions are written **in the same transaction as the status
   change** that caused them. A committed move without its transition
   would silently shorten the item's recorded life.
4. `from_status = to_status` is never written (the move endpoint already
   short-circuits a no-op move).

### 5.2 Intervals

Intervals are **derived, not stored**: sort a task's transitions by `at`,
and each consecutive pair is `(status = previous.to_status, from =
previous.at, to = next.at)`. The final transition opens an interval
running to `done_at` for a finished task, or to `as_of` for an open one.

Because the intervals come from one ordered log, they **cannot overlap** —
which is the structural difference from the care-pathway sibling, where
concurrent care is real and the union algorithm exists to handle it. Here
the intervals partition the elapsed time exactly, by construction.

**The partitioned span is creation → finish (lead time), not the cycle
time.** The backlog dwell before an item starts is real time the
requester waited, and it has to land somewhere: time that belongs to no
status is time a report can quietly lose. Flow efficiency is still
measured against **cycle** time (§6.1), because the team cannot be held
to how long the backlog sat — but the dwell is never dropped, it is
reported as its own figure and as a `todo` share of the lead time.

Three edges are handled so the partition is exact in every case:

- **Before the first recorded transition.** A backfilled row (§5.4) is
  stamped at `status_changed_at`, later than the task's creation, so the
  span between them is unattributed. It is charged to the transition's
  `from_status` where one is recorded, and to `todo` otherwise — the
  **pessimistic** choice, matching the classification fallback, so
  unknown history can never flatter the figures.
- **`done` is terminal.** The clock stops there; a finished task
  analysed a year later must not have accrued a year.
- **Clock skew.** An `as_of` preceding the last transition yields
  zero-length intervals, never negative ones — and a transition in the
  future does not retroactively start the clock.

### 5.3 The status classification

Each board status maps to a VSM category. The **disclosed default**:

| Status | Category | Rationale |
|---|---|---|
| `todo` | UNVA (waste: `inventory`) | Work bought and not started; it ages while it waits. |
| `in_progress` | VA | Somebody is building the thing. |
| `in_review` | NNVA | Necessary and not building. Review *waiting* is waste; review *happening* is not, and the board cannot tell them apart — §17. |
| `blocked` | UNVA (waste: `waiting`) | The most actionable waste on any board. |
| `done` | terminal | Not an interval; the clock stops. |

**"Is `in_review` value-adding?" is a real argument**, and the answer is
local. So the map is overridable per deployment via
`PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES` (JSON, the same shape and
failure posture as the existing `..._WIP_LIMITS`: unparsable or unknown
keys ⇒ fall back to the disclosed default rather than half-applying an
override). Every response repeats the map in use, so a figure can never
be compared across two deployments without the difference being visible.

**Started** and **finished** statuses are declared alongside it:
started = anything other than `todo`; finished = `done`. This is what
separates cycle time from lead time (§6.1).

### 5.4 The backfill is labelled, not hidden

`tasks` carried only `status_changed_at`, so there is no history to
recover. The migration writes **one synthetic transition per live task**
(`NULL → current status` at `status_changed_at`), flagged
`backfilled = true`, so an existing board is analysable immediately.

Every analysis reports how many of its transitions were backfilled. A
figure resting on synthesised history is thereby visibly weaker than one
resting on observed moves — which is the difference between a useful
default and a lie.

### 5.5 Relationship to what already exists

| Existing | Relationship |
|---|---|
| `tasks.status_changed_at` / `done_at` | Kept, unchanged. TBA reads the transition log; these stay the fast path for the board. |
| `burndown` / `velocity` | Sprint-scoped and count-based; TBA is item-scoped and time-based. Complementary, and both remain "honest" — no ideal line, no interpolation. |
| WIP limits | The lever Little's Law identifies. §9 reports WIP *against* the configured limit. |
| `devops_events` / DORA | Deployment-level flow. Referenced, not recomputed. |
| The matcher payload | **Untouched.** Transitions are operational state, never part of the `Plan` DTO and never a matching signal. |

## 6. The measurement model

All of §6 is pure computation over `(transitions, as_of)` with no I/O,
implemented in `src/tba.rs` and unit-tested there.

### 6.1 Lead time and cycle time are different numbers

```
lead_time  = finished_at − created_at          (what the requester waits)
cycle_time = finished_at − first_started_at    (what the team controls)
```

Both are reported, always, and never one labelled as the other. The
error is worth spelling out because it only ever runs one way: an item
that sat in `todo` for three weeks and was built in two days has a cycle
time of 2 days and a lead time of 23. Quoting the cycle time as "our
delivery time" is a **10× flattering** misreport, and it is the single
most common mistake in flow reporting.

Flow efficiency is computed against **cycle time**, because the team
cannot be held to the backlog's dwell time. The `todo` dwell is instead
reported explicitly as its own figure, so it cannot simply vanish.

### 6.2 Intervals and clipping

Consecutive transitions give the intervals (§5.2). The last interval runs
to `done_at` (finished) or `as_of` (open). An `as_of` before the last
transition — a clock skew, or a client-supplied timestamp — yields a
zero-length final interval rather than a negative one.

### 6.3 The denominator rule

**The denominator is elapsed calendar time, never the sum of recorded
activity.** If it were the latter, a team recording only its
`in_progress` intervals would report 100% flow efficiency, and recording
*less* would score *better* — the exact inversion the method exists to
expose.

Here the rule is enforced structurally rather than by convention: the
intervals come from one ordered log and partition the elapsed time by
construction, so there is no unrecorded remainder to omit. An
unclassified status — a board column somebody added — falls back to
`unnecessary_non_value_adding` for the same reason: adding a column must
not be able to silently improve the flow efficiency. What remains
disclosable is the **backfill share** (§5.4), and it is reported.

### 6.4 The derived figures

| Figure | Definition |
|---|---|
| `lead_time_ms` | created → finished (or `as_of`) |
| `cycle_time_ms` | first started → finished (or `as_of`); null if never started |
| `work_time_ms` | Σ intervals in VA statuses (VT / φ) |
| `process_time_ms` | Σ intervals in VA + NNVA statuses (PT) |
| `wait_time_ms` | cycle time − process time (ω) |
| `blocked_time_ms` | Σ intervals in `blocked` |
| `queue_time_ms` | Σ intervals in `todo` — the backlog dwell (§6.1) |
| `flow_efficiency` | work time / cycle time (%A) |
| `by_status` | ms + share per status, with the category it was classified as; **partitions the lead time** |
| `by_category` | ms + share per VA / NNVA / UNVA; **partitions the lead time** |
| `transitions` | count, and how many were `backfilled` |
| `rework_count` | backwards transitions |
| `first_pass` | whether the item never moved backwards |
| `handoffs` | distinct assignees, and changes across the item's life |
| `age_ms` | for an open item: now − started. The actionable one. |

Every ratio ships with its numerator and denominator in milliseconds, and
a null figure carries a sibling reason rather than a sentinel zero.

### 6.5 Rework and first-pass yield

A transition is **backwards** when `to_status` sits earlier in the board
order than `from_status` (`in_review` → `in_progress`, `blocked`
excepted — `blocked` is orthogonal to progress, so moving into or out of
it is never counted as rework). An item is **first pass** if it finished
with no backwards transition; RFPY is the share of finished items that
did.

This is the honest counterweight to throughput. A team that raises
throughput while RFPY falls is not going faster; it is shipping work
back to itself, and the two figures are reported side by side so that
cannot be read as an improvement.

## 7. Cohort statistics and the service level expectation

### 7.1 Percentiles, not averages

Cycle times are long-tailed. Reported as **min / p50 / p75 / p85 / p95 /
max**, with the mean included and explicitly labelled skew-sensitive.

**p85 is called out** because it is the SLE convention: high enough to
be a commitment, low enough that the tail does not swallow it.

Percentiles use **nearest-rank** (`ceil(p × n)`, 1-indexed), stated in
the payload — so every percentile is an observed item, and "which one is
the p85?" has an answer.

### 7.2 Aggregate versus median flow efficiency

- **Aggregate** `Σ work / Σ cycle` — the system's ratio, dominated by
  the longest-running items, which is usually the right emphasis.
- **Median** of per-item efficiency — the typical item.

A large divergence *is the finding*: it means the waste is concentrated
in a minority of items — a different intervention from uniformly slow
flow.

### 7.3 The service level expectation

From the finished items' cycle times: **"p% of items finish within N
days"**, default p = 0.85, N the nearest-rank percentile.

The SLE is reported with its **sample size** and the **date range** it
was computed over, because a forecast from six items last quarter is not
a forecast. Below a minimum sample (default 10) it is returned as null
with a stated reason rather than as a confident number computed from
noise — the failure mode that discredits flow metrics faster than any
other.

A caller may also pass an explicit `target_days` to score against a
commitment already made, in which case the achieved percentage and
whether the target was met are reported alongside.

### 7.4 Breach attribution

For items that exceeded the SLE, the analysis reports which **status**
contributed the most non-value-adding time — turning "we missed 11 days"
into "we missed 11 days and 8 of them were in review".

### 7.5 Cross-plan rollup

A plan may contain other plans (`parent_ref`, recursive), so a portfolio
has flow figures of its own. Two rules fix what they mean:

- **The combined figures are the union of every task in the subtree**,
  not an average of the children's ratios. Averaging ratios weights a
  five-task plan equally with a five-hundred-task one, which is the same
  error §7.2 rejects at the item level.
- **The per-plan table is returned alongside, and for a portfolio it is
  usually the more useful half.** A rollup mixes boards whose teams mean
  different things by `in_progress` (§5.3 makes that classification
  deployment-local, and nothing forces two teams to agree). *Which child
  differs* is a firmer finding than the combined number, so the response
  never returns one without the other.

The walk is bounded three ways, and the reasons differ. A **visited
set**, because a cycle in `parent_ref` would revisit nodes and expand
exponentially rather than merely loop — the write path refuses a cycle,
but a rollup that *trusts* that is one bulk import or one direct
`UPDATE` away from hanging the service. A **depth cap** and a **node
cap**, so one enormous portfolio cannot become an unbounded response.
`truncated` reports a cap firing, and `revisits` reports containment
that is not a tree; neither is silent, because a rollup that quietly
covers half an estate reads as if it covered all of it.

## 8. Constraint analysis

Findings ordered by **recoverable time**, each naming the rule that
produced it. No composite score.

- **`status_dominates_wait`** — the status holding the largest share of
  non-value-adding time; fires above 40%.
- **`blocked_time`** — total time blocked. Separated because it is the
  waste with the shortest path to a fix: something specific is in the
  way, and it has a name.
- **`backlog_dwell`** — total `todo` time. Large dwell means work is
  being started too early, which is a WIP decision, not a capacity one.
- **`rework`** — backwards transitions and the RFPY.
- **`aging_wip`** — open items past the SLE, ranked by age. The only
  finding about work that can still be helped.
- **`handoff_heavy`** — items whose assignee changed three or more
  times.
- **`backfilled_history`** — the share of transitions synthesised by the
  migration rather than observed, so a thin-evidence analysis says so.

## 9. Flow analysis (queueing theory)

Over a window (default 90 days):

- **λ** = tasks created / window days
- **μ** = tasks finished / window days (throughput)
- **ρ** = λ / μ — demand against capacity
- **κ** = tasks started and not finished (WIP), now
- **τ̂** = κ / μ — the cycle time Little's Law implies for an item
  starting now

### 9.1 The consistency check

τ̂ is compared against the **observed** p50 cycle time of items finished
in the window:

- **τ̂ ≫ observed** — WIP is growing faster than it clears; recent
  completions flatter the system, and an item starting today will take
  far longer than the last one did.
- **τ̂ ≈ observed** — steady state; the observed cycle time is
  predictive.
- **τ̂ ≪ observed** — the queue is draining, or the window's completions
  were disproportionately old items being cleared.

Little's Law's stationarity assumption is named in the response, so a
short window on a volatile board does not get quoted as a forecast.

### 9.2 Delivery forecasting

Little's Law (§9) gives a consistency check; a **Monte-Carlo simulation
over the throughput history** gives a forecast. Two questions, both
served by `GET /api/plans/{pid}/forecast`:

- **"How long will these N items take?"** Each trial draws periods at
  random **with replacement** from the plan's throughput history and
  accumulates until the batch is covered; the distribution of trial
  lengths gives the answer.
- **"How many items will land in N periods?"** Each trial sums N drawn
  periods.

**The input is throughput, not cycle time** — see §17, where this spec
previously said the opposite. Cycle time answers a question about one
item; a batch forecast built from it assumes items are worked
sequentially and is pessimistic by roughly the team's parallelism.

Three properties the implementation fixes:

1. **The conservative percentile reverses between the two questions.**
   For *how long*, higher is more conservative (85% of runs finished by
   the p85). For *how many*, the conservative figure is the **15th**
   percentile — "at least this many, with 85% confidence". Quoting the
   p85 there would promise the best case while sounding careful, so the
   fields are named for what they mean (`at_least_items`) rather than
   for the percentile they came from, and each response repeats the
   direction.
2. **It is deterministic.** The seed is an input and is fixed unless
   supplied. A forecast that changes every time you reload it is not
   one anybody will act on.
3. **It refuses rather than guessing.** Below `MIN_THROUGHPUT_PERIODS`
   of history it returns a reason, not a number; a history of all
   zeroes returns *"the honest answer is `never`"*; and a per-trial
   ceiling turns what would otherwise be an unbounded accumulation loop
   into a reported `trials_hit_ceiling` count, so a percentile that is
   really a floor says so.

### 9.3 WIP against the limit

Where `PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS` is configured, per-status
occupancy is reported against its cap. This is where the theory becomes
a lever: cycle time = WIP / throughput, so lowering the cap is the one
change that shortens cycle time without anyone working faster.

Occupancy is reported next to the wait figures rather than in a separate
capacity view, because ρ near 1 and long queues are the same observation
(§2.3).

## 10. API surface

Read endpoints are `GET`, return `as_of`, and carry a `note` describing
the derivation — the convention the existing insight views follow. All
sit under `/api/*` and therefore behind the blanket guard
(`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`, default off).

### 10.1 Recording

There is **no recording endpoint**, and that is the design. Transitions
are written by the existing calls:

| Existing endpoint | Writes |
|---|---|
| `POST /api/plans/{pid}/tasks` | `NULL → todo` (or the created status) |
| `PATCH /api/plans/{pid}/tasks/{t}` | `from → to` |

`GET /api/plans/{pid}/tasks/{t}/transitions` exposes the log for
inspection. It is read-only: there is deliberately no edit or delete
(§5.1 invariant 2).

### 10.2 Analysis

| Method + path | Returns |
|---|---|
| `GET /api/plans/{pid}/tasks/{t}/time-analysis` | §6 for one task |
| `GET /api/plans/{pid}/time-analysis` | §7 plan cohort: cycle/lead distributions, flow efficiency, RFPY, SLE. Query: `?sle_percentile=`, `?target_days=`, `?sprint=` |
| `GET /api/plans/{pid}/constraints` | §8 ranked constraints |
| `GET /api/plans/{pid}/aging-wip` | §8 open items ranked by age against the SLE |
| `GET /api/plans/{pid}/flow` | §9 flow analysis. Query: `?window_days=` |
| `GET /api/plans/{pid}/cumulative-flow` | The board's composition sampled daily — the cumulative flow diagram. Query: `?days=` (default 60, max 365) |
| `GET /api/plans/{pid}/rollup` | §7.5 flow across a plan and everything it contains: the union plus the per-plan comparison. Query: `?depth=` |
| `GET /api/plans/{pid}/forecast` | §9.2 Monte-Carlo delivery forecast, both directions. Query: `?items=`, `?periods=`, `?history_periods=`, `?period_days=`, `?trials=`, `?seed=` |
| `GET /api/flow-classes` | The status → category map in force, plus the vocabularies |

### 10.3 Response conventions

- Durations in **milliseconds** (`*_ms`) plus rounded `*_days`.
- Ratios as floats in [0, 1], never pre-multiplied percentages, always
  with their numerator and denominator.
- Nulls carry a sibling `*_reason`. Never a sentinel zero.
- Every analysis response repeats the **classification map in force**
  and the **backfilled transition share**.

## 11. Persistence

- One migration: `task_transitions` plus the labelled backfill (§5.4).
- Indexes: `(task_pid, at)` for the per-task read, `(plan_pid, at)` for
  the plan-wide read, and `(to_status)` for status rollups.
- No derived storage. No stored `flow_efficiency` column: recomputation
  is cheap at this scale, and a stored ratio would drift the moment a
  transition was appended.
- Row caps on the analysis reads, matching the existing insight caps, so
  an unbounded plan cannot become an unbounded query.

## 12. Governance, honesty, and anti-gaming

### 12.1 The data is about people even when it is about work

A transition records who moved what, when. That is personal data under
GDPR even though its subject is a task. It inherits the service's
posture without exception: the blanket guard, ABAC where the plan
carries it, and audit. `actor_ref` and `assignee_ref` are the sensitive
fields; §12.4 governs what may be derived from them.

### 12.2 Aggregates must not re-identify

A plan with two tasks and two assignees discloses individual working
patterns by arithmetic. Cohort responses report `n`, and a cohort below
the minimum sample returns counts and constraint rankings without the
percentile detail that would isolate one item.

### 12.3 Anti-gaming is a design property

Board metrics are gamed the same way everywhere, and the defences are
structural:

1. **The log is append-only.** No edit, no delete. Correcting history
   means moving the card, which is itself recorded.
2. **No business-hours discounting.** A weekend in review really was a
   weekend in review. Working-hours arithmetic is the standard way to
   make queues disappear from a report while the customer still waits.
3. **No status is excluded.** Every millisecond from creation to finish
   lands in exactly one status and one category, and they sum to the
   lead time by construction — a property test, not a promise. There is
   no "on hold" state that stops the clock, and an unrecognised column
   counts against you rather than falling through a gap.
4. **Lead time is always reported next to cycle time** (§6.1), so the
   flattering number cannot travel alone.
5. **Throughput is always reported next to RFPY** (§6.5), so shipping
   work back to yourself cannot read as going faster.
6. **The backfill is labelled** (§5.4), so synthesised history is never
   mistaken for observed history.

### 12.4 Not a person metric — a stated refusal

The service will not compute per-assignee cycle time, throughput, or
flow efficiency, and this is a design decision rather than an unbuilt
feature. Three reasons, in order of how much they cost:

- **It is measuring the wrong thing.** If items are worked on 6% of
  their life, per-person speed addresses 6% of the problem. The queue
  belongs to the system.
- **It is confounded.** Cycle time depends on what the item was, who
  else was needed, and what it waited on. Attributing it to whoever held
  the card last is not a measurement.
- **It destroys the data.** A team measured on card movement will move
  cards to look good — split items, skip statuses, sit on `in_progress`.
  Because collection is a by-product of the work (§1.1), the measurement
  survives only as long as nobody has a reason to distort it. Turning it
  on individuals supplies that reason.

Handoff *counts* are reported as a property of the item's journey.
Assignee identity is available for audit and for the aging-WIP view
("who should be asked about this"), never as a ranked league table.

### 12.5 Audit

Transition writes ride the existing `task_created` / `task_moved` audit
records — the same transaction, so a change without its audit is not
possible. No new audit action is introduced, because no new user action
exists.

## 13. Non-functional requirements

| Requirement | Target |
|---|---|
| Per-task analysis | < 50 ms for ≤ 500 transitions |
| Plan analysis | < 500 ms for ≤ 1000 tasks; two bounded queries, no N+1 |
| Purity | §6–§9 computation has no I/O, no clock read (`as_of` is a parameter), deterministic |
| Never-panic | No `unwrap`, no overflow, no division by zero on any input including out-of-order transitions and clock skew |
| Bounded input | Per-task transition count capped; plan reads capped |
| Backward compatible | Existing boards analyse from the labelled backfill; no existing endpoint changes shape |
| Write overhead | One extra INSERT per status change, in the same transaction |

## 14. Testing strategy

### 14.1 Pure unit tests (`src/tba.rs`, DB-free)

- Interval derivation: none, one, many transitions; out-of-order input;
  open versus finished.
- Cycle versus lead time: an item that sat in `todo` for three weeks
  reports 2 days cycle and 23 lead — **the regression test against the
  flattering conflation** (§6.1).
- The denominator rule: statuses and categories each sum to the lead
  time exactly, including the pre-first-transition span and an empty
  log.
- Flow efficiency in [0, 1]; work ≤ process ≤ cycle ≤ lead.
- Classification: the disclosed default; a valid override; an
  unparsable override falls back whole rather than half-applying.
- Rework: forwards only, one backwards move, `blocked` round trips
  (which are **not** rework), RFPY over a mixed cohort.
- Percentiles: nearest-rank on n = 1, 2, even, odd; every value observed.
- SLE: below the minimum sample → null with a reason.
- Little's Law: μ = 0, λ = 0, and the three divergence labels.
- Degenerate: `as_of` before the last transition, duplicate timestamps,
  a task finished without ever starting — each a stated null, never a
  panic.

### 14.2 Boundary / validation tests

- An unknown status is refused by the move endpoint (existing pin) and
  therefore never reaches the log.
- The transitions endpoint is read-only — no edit or delete route
  exists.

### 14.3 Integration (DB-gated, `--ignored`)

- Create → move → move → done writes four transitions with the right
  `from`/`to` and no gaps.
- A no-op move writes no transition.
- A refused move (WIP limit) writes no transition — the log must not
  record a move that did not happen.
- Analysis round trip: known timestamps → expected cycle time, flow
  efficiency, and per-status split.
- Backfill: a task created before the migration analyses and reports
  `backfilled: 1`.
- Guard: with `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH=1`, every TBA
  path requires a token.

### 14.4 Property tests

Over arbitrary transition sequences: per-status and per-category totals
each sum to the lead time; work ≤ process ≤ cycle ≤ lead; ratios in
[0, 1]; no panic.

## 15. Tasks

| id | Task | Verified by |
|---|---|---|
| **TBA-1** | Migration: `task_transitions` + indexes + the labelled backfill (§5.4, §11) | §14.3 backfill test |
| **TBA-2** | `SeaORM` entity; the status classification with its env override (§5.3) | §14.1 |
| **TBA-3** | Pure `src/tba.rs`: intervals, cycle/lead, per-status and per-category splits, rework, handoffs (§6) | §14.1, §14.4 |
| **TBA-4** | Pure: percentiles, plan rollup, SLE, RFPY (§7) | §14.1 |
| **TBA-5** | Pure: constraints and flow/Little's Law (§8, §9) | §14.1 |
| **TBA-6** | Write transitions from `create_task` and `move_task`, in-transaction (§5.1, §10.1) | §14.3 |
| **TBA-7** | Controller: the six read endpoints (§10.2), routes wired, OpenAPI documented | §14.3 |
| **TBA-8** | Front-end: cumulative flow diagram, aging-WIP board, SLE badge — **done 2026-08-23** (`/plans/{pid}/flow`) | front-end vitest + Playwright |
| **TBA-9** | Cross-plan rollup: flow across a plan and everything it contains, via `parent_ref` (§7.5) — **done 2026-08-24** | §14.1 |
| **TBA-10** | Prometheus gauges: plan flow efficiency, p85 cycle time, WIP against limit — **done 2026-08-23** (`src/flow_metrics.rs`, default-off) | §14.1, §14.3 |
| **TBA-11** | Monte-Carlo delivery forecasting — from the **throughput** distribution, not the cycle-time one (§9.2; the §17 entry records the correction) — **done 2026-08-23** | §14.1 |

## 16. Implementation status

TBA-1 … TBA-8 are **implemented**. In
`project-portfolio-management-service-with-loco`:

| Piece | Location |
|---|---|
| Migration | `migration/src/m20260823_000001_time_based_analysis.rs` |
| Entity | `src/models/_entities/task_transitions.rs` |
| Pure analysis + classification + tests | `src/tba.rs` |
| Transition writes | `src/controllers/engineering.rs` (`create_task`, `move_task`) |
| HTTP surface | `src/controllers/tba.rs` |
| Routes | `src/app.rs` (`tba::routes()`) |
| OpenAPI | `src/openapi.rs` |

The cumulative-flow endpoint (`GET /api/plans/{pid}/cumulative-flow`,
§10.2) was added with TBA-8: it is the one view here that cannot be
assembled client-side, since it needs every task's whole history at
once, and an API that shipped the log to the browser to re-derive it
would be sending far more data to compute what the server already
indexes.

In `project-portfolio-management-front-end-with-svelte`:

| Piece | Location |
|---|---|
| API client + presentation helpers | `src/lib/api/tba.ts` |
| Cumulative flow diagram | `src/lib/components/CumulativeFlow.svelte` |
| The view | `src/routes/plans/[pid]/flow/+page.svelte` |
| Tests | `tests/unit/tba.test.ts`, `tests/e2e/flow.spec.ts` |

TBA-10 (flow gauges) landed 2026-08-23: `src/flow_metrics.rs` (a
default-off refresh loop) plus the `ppm_flow_*` family in
`src/metrics.rs`. Per-plan series are **capped** and small boards are
**suppressed**, because `/metrics.prom` is on the public allow-list and
a flow efficiency over two tasks describes two people's week — which
§12.4 refuses to measure, and reaching it by arithmetic is the same
thing. Per-column occupancy is deliberately **not** exported: it would
be five series per plan, the single biggest cardinality contributor
here, and the over-cap *count* carries the alertable fact in one
series. The p85 gauge inherits the service level expectation's own
minimum sample, so it stays absent rather than publishing a number from
noise.

TBA-11 (Monte-Carlo forecasting) landed 2026-08-23: the pure
`throughput_history` / `forecast_batch` / `forecast_items` in
`src/tba.rs`, and `GET /api/plans/{pid}/forecast`. It corrected this
spec's own §17 claim about which distribution a batch forecast needs.

TBA-9 (cross-plan rollup) landed 2026-08-24: the pure, bounded
`walk_descendants` in `src/tba.rs` and `GET /api/plans/{pid}/rollup`.

**Every TBA task is now closed.** What remains are the §17 open
questions, which are decisions rather than unbuilt work.

## 17. Open questions

- **Review waiting versus review happening.** `in_review` conflates "in
  someone's queue" with "being read". Splitting it into `review_queue`
  and `in_review` would make the single largest NNVA block legible — at
  the cost of another board column, which teams resist. *Lean: leave the
  board alone; revisit if the constraint ranking keeps naming
  `in_review` and nobody can act on it.*
- **Blocked-reason vocabulary.** `blocked` is the most actionable waste
  and currently carries no reason, so the finding stops at "8 days
  blocked" rather than "8 days blocked on external dependency". A
  disclosed reason vocabulary would fix that. *Lean: add it with TBA-8,
  when a UI exists to capture it.*
- ~~**Monte-Carlo forecasting.** The cycle-time distribution is exactly
  the input a Monte-Carlo "when will these 20 items be done" forecast
  needs~~ — **RESOLVED, and the sentence above was wrong.** It is the
  standard error in the field and worth leaving struck through rather
  than quietly deleting. The cycle-time distribution answers a question
  about **one item** ("this will finish within 11 days at 85%"), which
  is exactly what the service level expectation already reports (§7.3).
  A **batch** forecast needs the **throughput** distribution — how many
  items the team actually finished per period. Using cycle times for a
  batch implicitly assumes items are worked one at a time: sum twenty
  cycle times for a team running five in parallel and the answer is
  roughly five times too pessimistic. Throughput sampling makes no such
  assumption, because parallelism is already baked into the counts.
  Delivered as TBA-11 (§9.3) on the correct input.
- **Sprint versus continuous flow.** The plan-level view is
  sprint-agnostic; `?sprint=` filters. Whether sprint-scoped flow
  efficiency is meaningful, or just burndown with extra steps, is
  unresolved.
- ~~**Cross-plan rollup semantics.**~~ — **RESOLVED (TBA-9, §7.5):**
  it aggregates, as the union of the subtree's tasks rather than an
  average of the children's ratios, and the per-plan table always ships
  with it. The lean was right; what the implementation added is that
  the per-plan half is the more useful one, because a rollup mixes
  boards whose teams classify `in_progress` differently.
- **Minimum SLE sample.** 10 is a defensible floor and not a principled
  one. Deployment-configurable, or fixed?

## 18. References

### Time-based analysis

- Barker, R. C. — *The Time Based Organisation: Recreating and
  Transforming Existing Organisations*.
- [Time Based Analysis in the UK NHS](https://www.drbobbarker.co.uk/post/time-based-analysis-in-the-uk-nhs)
  — the 8–14% value-adding figures and the tracking method.

### Value stream mapping and queueing theory

- [value-stream-mapping](https://github.com/joelparkerhenderson/value-stream-mapping)
  — VA / NNVA / UNVA, VT / PT / LT / %A / #HO / RFPY / %C&A, the eight
  wastes.
- [queueing-theory](https://github.com/joelparkerhenderson/queueing-theory)
  — λ / μ / ρ / τ / ω / φ / θ / κ, Little's Law κ = λτ.
- Lean Enterprise Institute — value stream mapping lexicon.

### Flow metrics

- Little, J. D. C. — *A Proof for the Queuing Formula L = λW*.
- Reinertsen, D. — *The Principles of Product Development Flow* (queueing
  cost, WIP, and why utilisation is not the goal).
- The Kanban flow-metric quartet — cycle time, throughput, WIP, work
  item age — and the service level expectation.

### Family cross-references

- [care-pathway time-based analysis](../../care-pathway/spec/time-based-analysis.md) — the sibling section (§2.5 states the differences).
- [`agents/share/security.md`](../../agents/share/security.md) — the invariants cited in §12–§13.
- [`06-functional-requirements.md`](06-functional-requirements.md), [`09-api-surface.md`](09-api-surface.md), [`10-persistence.md`](10-persistence.md), [`12-compliance.md`](12-compliance.md).
