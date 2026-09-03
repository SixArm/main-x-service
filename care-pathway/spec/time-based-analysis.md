# Time-based analysis (TBA) — living specification

> **Source of truth for the care-pathway time dimension.** This document
> is the canonical artefact for time-based analysis across the
> care-pathway trio: what is measured, how each figure is defined, what
> the API returns, how it is stored, and what it deliberately refuses to
> do. It is a *cross-cutting* section of the [care-pathway entity
> spec](index.md) rather than a numbered chapter, because it spans the
> domain model (§5), the API surface (§9), persistence (§10) and
> compliance (§12) at once.
>
> **Family contract.** The measurement model is shared with the
> portfolio trio and is fixed in
> [`agents/share/time-based-analysis.md`](../../agents/share/time-based-analysis.md);
> this document is the care-pathway *adoption* of it. Where the two
> disagree, the family contract wins on the model and this spec wins on
> anything care-pathway-specific.
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
7. [Cohort statistics and access standards](#7-cohort-statistics-and-access-standards)
8. [Constraint analysis](#8-constraint-analysis)
9. [Flow analysis (queueing theory)](#9-flow-analysis-queueing-theory)
10. [API surface](#10-api-surface)
11. [Persistence](#11-persistence)
12. [Privacy, compliance, and anti-gaming](#12-privacy-compliance-and-anti-gaming)
13. [Non-functional requirements](#13-non-functional-requirements)
14. [Testing strategy](#14-testing-strategy)
15. [Tasks](#15-tasks)
16. [Implementation status](#16-implementation-status)
17. [Open questions](#17-open-questions)
18. [References](#18-references)

---

## 1. Purpose and vision

Time-based analysis (TBA) evaluates a patient journey by measuring
**elapsed time through the operational pathway** rather than by
departmental activity or budget. It asks one question of the whole
journey: *of the calendar time a patient spent on this pathway, how much
of it was care?*

The answer is reliably shocking. Tracking an outpatient end-to-end
through an NHS trust found **14% value-adding time**; other journeys
measure nearer **8%**. The remaining 86–92% is queueing, handoff,
re-referral, and time in which nothing at all happened. Championed by
Dr. R. C. (Bob) Barker, the method's central claim is that this waste is
visible without new infrastructure — you find it by *measuring the gaps*,
not by measuring the departments.

The care-pathway service is unusually well placed to carry this. It
already owns the pathway template (the registry) and the enrolled
instance (a subject's journey on that template, with steps, events, a
care team, and measures). What it lacked was **duration**: everything
recorded was either a point in time (`instance_events.occurred_at`) or a
date with no start (`instance_steps.done_on`). TBA adds the missing
primitive — a *segment* with a start and an end, classified by whether it
added value — and derives everything else from it.

**Vision.** A frontline clinician maps one real patient journey in an
afternoon, and the service returns: the value-adding ratio, the three
longest queues by name, how many times the patient changed hands, and how
this journey compares to the cohort on the same pathway. No consultants,
no new systems, no new data collection beyond the timestamps staff
already know.

### 1.1 Design goals

| Goal | Meaning |
|---|---|
| **Calendar time is the denominator** | Efficiency is measured against wall-clock elapsed time, never against the sum of recorded activity. Unrecorded time is waste until proven otherwise (§6.3). |
| **Honest about coverage** | Every response reports how much of the journey was actually mapped. An unmapped journey must never read as an efficient one (§6.6). |
| **Derived, never asserted** | No stored "efficiency" column. Every figure recomputes from segments + clock, so a corrected timestamp corrects the analysis. |
| **Ungameable by construction** | No clock pause, no stop-the-clock milestone, no excluded categories (§12.3). |
| **Names the constraint** | The output ranks *where the time went*, ordered by time recoverable, so the finding is actionable rather than a score. |
| **Standard vocabulary** | VSM terms (VA / NNVA / UNVA, LT, PT, VT, %A, #HO) and queueing symbols (λ, μ, ρ, τ, ω, φ, κ) are used as defined upstream, not reinvented. |

## 2. Research basis

TBA as implemented here is the intersection of three literatures. Each
contributes a distinct piece, and the spec is explicit about which piece
comes from where so a future edit does not blur them.

### 2.1 Barker's time-based analysis

Bob Barker's *The Time Based Organisation* method tracks a unit of work
(a manufactured part, a service request, **a patient**) end to end and
records, for every phase, whether it was value-adding "touch time" or
not. The headline ratio — value-adding touch time as a percentage of
elapsed calendar time — is typically **under 15%** across sectors, and
measured at **8–14%** in the NHS journeys published.

Three method commitments carry into this spec:

- **The whole journey, or nothing.** Optimising a department that is 3%
  of the journey cannot move the journey. Barker calls the alternative
  *islands of efficiency*: fast scans and quick surgeries separated by
  weeks of limbo.
- **Data comes from the value-adding staff.** The people who perform the
  work record the times, because they are the ones who know where the
  delay actually sits. Not a consultancy, not a returns process.
- **Untapped capacity, not more capacity.** If 86% of elapsed time is
  non-value-adding, existing capital and staff already contain the
  headroom; the constraint is sequencing.

### 2.2 Value stream mapping (VSM)

VSM supplies the **classification and the metric names**. See
[value-stream-mapping](https://github.com/joelparkerhenderson/value-stream-mapping).

- **VA — value adding**: the patient would recognise it as their care.
- **NNVA — necessary non-value adding**: required but not care —
  consent, safety checks, statutory recording, mandated waits.
- **UNVA — unnecessary non-value adding**: pure waste — re-keying,
  chasing, repeat appointments that changed nothing.
- **Metrics**: Value Time (VT), Process Time (PT), Lead Time (LT),
  Percentage Activity (%A = PT/LT), Number of Handoffs (#HO), Rolled
  First Pass Yield (RFPY), Percent Complete & Accurate (%C&A).
- **The eight wastes**: waiting, transportation, motion, over-processing,
  defects/errors, inventory, overproduction, underutilised people. This
  spec records the waste type on the segment, so "where is the time
  going" and "what *kind* of waste is it" are answerable separately.

The VA/NNVA/UNVA split matters clinically. A blanket "non-value-adding"
figure invites the reply *"but the safety checklist is not waste"* — and
it is not. Separating NNVA from UNVA lets the analysis concede the point
and still name the 60% that is genuinely recoverable.

### 2.3 Queueing theory

Queueing theory supplies the **flow mathematics** — why the waits exist
at all, and what would happen if demand or capacity moved. See
[queueing-theory](https://github.com/joelparkerhenderson/queueing-theory).

- λ (lambda) arrival rate, μ (mu) service rate, ρ (rho) = λ/μ utilisation
- τ (tau) lead time, ω (omega) wait time, φ (phi) work time, κ (kappa)
  item count
- **Little's Law**: κ = λτ — average items in the system equals arrival
  rate × time in system.

The operationally important consequence is **non-linearity near
saturation**: as ρ → 1, expected wait ω grows without bound. A pathway
running at 95% utilisation is not "5% from trouble"; it is already in
it. This is why a caseload that looks merely busy produces waits that
look inexplicable, and why §9 reports ρ next to the wait percentiles
rather than in a separate view.

### 2.4 NHS access standards

The published English standards give TBA its external reference points.
They are *thresholds on lead time*, which makes them directly comparable
to a TBA lead-time distribution:

| Standard | Threshold | Operational target |
|---|---|---|
| RTT incomplete pathway | 18 weeks (126 days) | 92% within |
| Cancer — faster diagnosis (FDS) | 28 days | 75%, rising to 80% (March 2026) |
| Cancer — decision-to-treat | 31 days | 96% |
| Cancer — referral to treatment | 62 days | 85% |
| Diagnostics (DM01) | 6 weeks (42 days) | ≤1% waiting over (2028/29) |
| A&E | 4 hours | 85% (2028/29) |

These are shipped as a **catalogue of named standards** (§7.3) so a
cohort can be scored against `rtt_18_weeks` without a caller hard-coding
126. They are reference data with a citation and a date, not an
assertion that any given pathway is subject to them.

### 2.5 Time-compression interventions

TBA is a diagnostic; these are the treatments it is usually pointed at,
and the analysis should make their effect visible:

- **PIFU** (patient-initiated follow-up) — removes routine low-value
  follow-ups. Visible as a fall in UNVA segments of stage `follow_up`.
- **Advice & Guidance** — resolves or redirects at referral. Visible as
  a fall in `referral`-stage lead time and in `#HO`.
- **Virtual wards / remote monitoring** — moves recovery out of a bed.
  Visible as a fall in `treatment`-stage elapsed time without a fall in
  VT.

## 3. Scope and non-goals

### In scope

- A recorded **segment** primitive: a bounded interval on one instance,
  classified VA / NNVA / UNVA with a stage and an optional waste type.
- An explicit **pathway clock** (start / stop) per instance, defaulting
  to the existing enrolment and closure dates.
- **Per-instance analysis**: LT, VT, PT, %A, coverage, gaps, handoffs,
  stage and category breakdowns.
- **Cohort analysis** per pathway template: lead-time percentiles,
  aggregate and median %A, compliance against a named standard.
- **Constraint ranking**: stages and named gaps ordered by recoverable
  time.
- **Flow analysis**: λ, μ, ρ, κ, τ and a Little's-Law consistency check
  over open instances.

### Out of scope (and why)

| Not doing | Why |
|---|---|
| Scheduling or booking | The service is a registry plus an instance journal. TBA reads the journal; it does not run the clinic. `patient-flow` owns operational bed/slot state. |
| Automatic segment inference from events | `instance_events` are point-in-time notes with no declared duration. Inferring intervals from them would manufacture data and make the coverage figure a lie. Explicit only. |
| Forecasting / simulation | Little's Law is used as a *consistency check* on observed figures (§9.4), not to project a future state. A discrete-event simulator is a different product. |
| Cross-service journey stitching | A journey crossing into `patient-flow` or `case` needs the link-graph aggregator. Deferred to TBA-9 (§15). |
| Ranking clinicians | The output is never per-clinician throughput. Handoff counts are per journey; care-team load already exists and is deliberately a workload view, not a performance view (§12.4). |
| A single "efficiency score" | %A next to a coverage figure is honest; a composite score hides which half you are looking at. |

## 4. Glossary

| Term | Symbol | Definition here |
|---|---|---|
| Segment | — | A recorded interval `[started_at, ended_at)` on one instance, with a stage, a VSM category, and optionally a waste type, actor, and location. |
| Clock | — | The instance-level `[clock_start_at, clock_stop_at)` window. The denominator for every ratio. |
| Lead time | τ / LT | Clock end (or `as_of` if running) − clock start. Elapsed calendar time. |
| Value time | VT | Union of the durations of VA segments (§6.4). |
| Process time | PT | Union of the durations of VA **+ NNVA** segments — time in which the system was doing something to or for the patient. |
| Touch time | φ | Raw **sum** of segment durations, not de-overlapped. Resource effort, which may exceed LT. |
| Wait time | ω | LT − PT. Includes both recorded waiting segments and unrecorded gaps. |
| Percentage activity | %A | PT / LT. The VSM ratio. |
| Value-adding ratio | %VA | VT / LT. **The Barker headline** — the 8–14% figure. |
| Coverage | — | (union of all segment durations) / LT. How much of the journey was mapped. |
| Gap | — | A maximal sub-interval of the clock covered by no segment. |
| Handoff | #HO | A change of `actor_ref` (or of `location_ref`) between consecutive segments in time order. |
| Arrival rate | λ | Enrolments per day over a window. |
| Service rate | μ | Closures per day over a window. |
| Utilisation | ρ | λ / μ. |
| Work in progress | κ | Count of open (non-terminal) instances. |
| Stage | — | Journey phase: `referral`, `triage`, `diagnostics`, `treatment`, `follow_up`, `discharge`, `other`. |
| Category | — | `value_adding` \| `necessary_non_value_adding` \| `unnecessary_non_value_adding`. |
| Waste | — | One of the eight VSM wastes, recorded only on non-VA segments. |

## 5. Domain model

### 5.1 The segment

A **segment** is the new primitive. It belongs to exactly one
`pathway_instance` and carries:

```
instance_segments
  pid            UUID        public identifier
  instance_pid   UUID        owning instance
  label          TEXT        human name — "MRI", "await triage outcome"
  stage          TEXT        referral|triage|diagnostics|treatment|follow_up|discharge|other
  category       TEXT        value_adding|necessary_non_value_adding|unnecessary_non_value_adding
  waste          TEXT NULL   waiting|transportation|motion|over_processing|
                             defects|inventory|overproduction|underutilised_people
  started_at     TIMESTAMPTZ
  ended_at       TIMESTAMPTZ NULL   -- NULL = still running
  actor_ref      TEXT NULL   worker:|organization: URN — who
  location_ref   TEXT NULL   place:|organization: URN — where
  note           TEXT NULL
  position       INTEGER     stable tiebreak for equal starts
```

Invariants, enforced at the boundary (§14 pins each):

1. `ended_at > started_at` when present. A zero-length or reversed
   segment is a `422`, not a silently-ignored row.
2. `category` and `stage` come from the closed vocabularies above.
3. `waste` is **refused on a `value_adding` segment**. Value-adding
   waste is a contradiction, and permitting it would corrupt the waste
   ranking in §8.
4. `waste` is **required on `unnecessary_non_value_adding`**. Declaring
   something pure waste without saying what kind is an assertion the
   analysis cannot act on.
5. At most one open (`ended_at IS NULL`) segment per instance. Two
   simultaneously-running open segments have no defensible end time, so
   the second is refused until the first is closed.

Segments **may overlap** once closed — concurrent care is real, and §6.4
handles it by union rather than by forbidding it.

### 5.2 The clock

`pathway_instances` gains two nullable columns:

```
clock_start_at  TIMESTAMPTZ NULL
clock_stop_at   TIMESTAMPTZ NULL
```

Resolution order, applied on read (§6.1) so existing rows keep working:

- `clock_start_at`, else `enrolled_on` at 00:00:00 UTC.
- `clock_stop_at`, else `closed_on` at 00:00:00 UTC when the instance is
  terminal, else the request's `as_of` (a running clock).

The existing dates are a coarse fallback, not a substitute: a journey
measured in hours needs the timestamps set explicitly. Responses declare
which source was used (`clock.start_source`, `clock.stop_source`) so a
day-resolution figure is never mistaken for a measured one.

### 5.3 Relationship to what already exists

| Existing | Relationship to TBA |
|---|---|
| `instance_steps` | Checklist completion, date-resolution, no start. Untouched; a step is *not* a segment and is not inferred into one. |
| `instance_events` | Point-in-time journal. Untouched; not inferred into segments (§3). |
| `instance_measures` | Clinical/PROM values. Orthogonal: outcome, not time. |
| `outcomes` view | Recorded closure outcome distribution. TBA is the time twin of it — the two answer "did it work" and "what did it cost in time". |
| The matcher payload | **Untouched.** TBA is operational state on the instance layer and is never part of the `CarePathway` DTO, never persisted into the JSONB payload, and never a matching signal. |

## 6. The measurement model

All of §6 is pure computation over `(clock, segments, as_of)` with no
I/O, implemented in `src/tba.rs` and unit-tested there.

### 6.1 Lead time (LT, τ)

```
LT = clock_stop − clock_start
```

with `clock_stop` resolved per §5.2. `LT ≤ 0` (a stop at or before the
start) yields a null analysis with a stated reason rather than a division
by zero or a negative ratio.

### 6.2 Clipping

Every segment is clipped to the clock window before it counts:

```
effective = [max(started_at, clock_start), min(ended_at ?? as_of, clock_stop))
```

A segment lying wholly outside the window contributes zero. This is what
makes the ratios provably bounded: no sequence of recorded segments can
push VT above LT.

### 6.3 The denominator rule

**The denominator is elapsed calendar time, never the sum of recorded
activity.** This is the single most important rule in the document and
the one most likely to be "simplified" away by a later edit.

If the denominator were the sum of recorded segments, a service that
records only its four value-adding segments would report %VA = 100%.
That is precisely inverted: recording *less* would score *better*, and
the 86–92% that TBA exists to expose is mostly time for which no record
exists at all. Unrecorded time is counted as non-value-adding, and §6.6
reports how much of the figure rests on that assumption.

### 6.4 Union versus sum

Segments may overlap. Two figures are therefore reported and must not be
conflated:

- **VT / PT use the interval union** — de-overlapped wall-clock time.
  Bounded by LT, so %A and %VA are always in [0, 1].
- **Touch time φ uses the raw sum** — resource effort. May exceed LT when
  care is concurrent, and that is meaningful (two clinicians for an hour
  is two clinician-hours).

Reporting only the sum would let a well-staffed hour of concurrent care
push %VA above 1. Reporting only the union would hide the resource cost.
The union algorithm (sort by start, merge overlapping) is the core pure
function and is property-tested (§14).

### 6.5 The derived figures

| Figure | Definition |
|---|---|
| `lead_time_ms` | LT (§6.1) |
| `value_time_ms` | union of VA segments |
| `process_time_ms` | union of VA + NNVA segments |
| `waste_time_ms` | union of UNVA segments |
| `touch_time_ms` | raw sum of all segments (φ) |
| `wait_time_ms` | LT − process_time (ω) |
| `unrecorded_ms` | LT − union of *all* segments |
| `value_adding_ratio` | value_time / LT (**%VA**) |
| `activity_ratio` | process_time / LT (**%A**) |
| `coverage_ratio` | (union of all segments) / LT |
| `by_category` | ms + share per VA / NNVA / UNVA, plus `unrecorded` |
| `by_stage` | ms + share per stage, plus segment count |
| `by_waste` | ms per waste type, non-VA segments only |
| `handoffs` | #HO — actor changes, location changes, and the total |
| `distinct_actors` / `distinct_locations` | touch-point breadth |

Every ratio is emitted alongside its numerator and denominator in
milliseconds, so a consumer can re-aggregate without trusting our
rounding.

### 6.6 Coverage is reported, always

`coverage_ratio` accompanies every %VA and %A. Interpretation guidance
travels with the payload:

- coverage < 0.20 — the journey is essentially unmapped; %VA is a floor,
  not a measurement.
- coverage ≥ 0.80 — the non-value-adding figure is substantially
  evidenced rather than inferred.

An unmapped journey reports %VA ≈ 0 with coverage ≈ 0, which reads
correctly as *"we do not know"* rather than *"catastrophically
inefficient"*. A response carries a `confidence` label (`unmapped`,
`partial`, `mapped`) derived from coverage so a UI cannot accidentally
render the first as the second.

## 7. Cohort statistics and access standards

### 7.1 Percentiles, not means

Lead-time distributions are right-skewed: a handful of journeys run ten
times the median. The mean of such a distribution describes no patient.
Cohort figures are therefore reported as **min / p50 / p75 / p90 / p95 /
max**, with the mean included but explicitly labelled as the
skew-sensitive figure.

Percentiles use the **nearest-rank** method on the sorted sample
(`ceil(p × n)`, 1-indexed), stated in the payload. Nearest-rank always
returns an observed value, which matters when someone asks *"which
patient is the p90?"* — with interpolation the answer is "nobody".

### 7.2 Aggregate versus median %A

Two cohort efficiency figures, both reported:

- **Aggregate** `Σ VT / Σ LT` — the system's overall ratio. Dominated by
  the longest journeys, which is usually the right emphasis.
- **Median of per-instance %VA** — the typical journey. Insensitive to
  one 400-day outlier.

A large divergence between them *is itself a finding*: it means the
waste is concentrated in a minority of journeys, which is a different
intervention from uniformly slow flow.

### 7.3 The standards catalogue

Named standards from §2.4, each with threshold, operational target,
authority, and citation date. A cohort query naming one gets back:
`within` / `breached` counts, the achieved percentage, the target, and
whether the target was met.

Callers may instead pass an explicit `target_days`, so a local pathway
with a local promise is measurable without pretending to be an RTT
pathway.

Standards are **reference data with a date**. Targets move (FDS 75% →
80%); a stale threshold silently mis-scoring a cohort is worse than no
threshold, so each entry carries `as_of` and the response repeats it.

### 7.4 Breach attribution

For breached instances, the cohort view reports which stage contributed
the most non-value-adding time — turning "we missed 62 days" into "we
missed 62 days and 41 of them were in diagnostics".

## 8. Constraint analysis

Barker's method is about naming the constraint, not scoring the system.
`constraints` returns findings ordered by **recoverable time**.

### 8.1 Gaps

A gap is a maximal sub-interval of the clock covered by no segment.
Gaps are ranked by duration and reported with the segments that bound
them (`after` / `before`), because a gap's *name* is the pair of things
it sits between: "8 days between referral received and triage" is
actionable; "8 days of unrecorded time" is not.

### 8.2 Stage ranking

Per stage across the cohort: total non-value-adding time, share of all
non-value-adding time, instance count, and p50/p90 of the stage's own
elapsed contribution. Ordered by total time — the stage where the most
time is recoverable, which is not necessarily the slowest stage per
visit.

### 8.3 Waste ranking

Per waste type: total time and instance count, over non-VA segments
only. Answers "is this a waiting problem or a rework problem", which
determines whether the fix is scheduling or process redesign.

### 8.4 Handoff cost

Consecutive segments whose `actor_ref` or `location_ref` differ are
handoff boundaries. The analysis reports the count and **the time in the
gaps at those boundaries** — the cost of changing hands, as distinct
from the cost of the work. Redundant handoffs are one of Barker's three
named constraints, and this is the figure that makes them arguable.

### 8.5 Disclosed rules, not a score

Every finding names the rule that produced it (`longest_gap`,
`stage_dominates_waste`, `handoff_heavy`, `low_coverage`), matching the
disclosed-rule convention the existing `insights/coverage` view already
uses. Thresholds are stated in the payload. No composite index.

## 9. Flow analysis (queueing theory)

`GET /api/instances/flow` computes, over a window (default 90 days):

### 9.1 Rates

- **λ** = enrolments in window / window days
- **μ** = closures in window / window days
- **ρ** = λ / μ (null when μ = 0, with a stated reason — not ∞, and not
  silently zero)

### 9.2 Work in progress

- **κ** = count of open instances (`active` + `on_hold`) now.

### 9.3 Little's Law

κ = λτ, so the **implied lead time** τ̂ = κ / λ. This is an *estimate of
how long a journey now entering will take*, derived from the queue
rather than from any individual journey.

### 9.4 The consistency check

τ̂ is compared to the **observed** p50 lead time of instances closed in
the window. Divergence is the finding:

- **τ̂ ≫ observed** — the backlog is growing; today's closures are
  finishing faster than the queue can be cleared. Recent completions
  flatter the system.
- **τ̂ ≈ observed** — the system is in steady state and the observed lead
  time is predictive.
- **τ̂ ≪ observed** — the queue is draining, or closures are
  disproportionately old cases being cleared.

Stated as a labelled interpretation, with both numbers, and with
Little's Law's stationarity assumption named — it holds over a period
long enough for arrivals and departures to balance, so a short window on
a volatile pathway gives a τ̂ that should not be quoted.

### 9.5 Utilisation is reported next to waits

Per §2.3, ρ near 1 is where wait time explodes. ρ is reported alongside
the wait percentiles rather than in a separate "capacity" view, so the
non-linear relationship is visible where the waits are read.

## 10. API surface

All read endpoints are `GET`, return `as_of`, and carry a `note`
describing the derivation — the convention the existing `insights` and
instance views already follow. All sit under `/api/*` and therefore
behind the blanket guard (`CARE_PATHWAY_REQUIRE_AUTH`, default off).

### 10.1 Recording

| Method + path | Purpose |
|---|---|
| `POST /api/instances/{pid}/segments` | Record a segment. `422` on the §5.1 invariants. An omitted `ended_at` opens a running segment. |
| `GET /api/instances/{pid}/segments` | List this instance's segments in time order. |
| `POST /api/instances/{pid}/segments/{seg}/close` | Close the open segment (`ended_at`, default now). `422` if already closed or if the end precedes the start. |
| `POST /api/instances/{pid}/clock` | Set `start` or `stop` explicitly (`{"event":"start"\|"stop","at":…}`). |

Writes audit as `instance_segment_recorded` / `instance_segment_closed`
/ `instance_clock_set` through the existing `Audit::record` path.

### 10.2 Analysis

| Method + path | Returns |
|---|---|
| `GET /api/instances/{pid}/time-analysis` | §6 for one instance: clock, totals, ratios, coverage, by-category, by-stage, by-waste, handoffs, ranked gaps. |
| `GET /api/instances/{pid}/timeline` | The mapped journey as an ordered wall — segments and gaps interleaved, each with duration and category. The visual artefact of §2.1. |
| `GET /api/care-pathways/{pathway}/time-analysis` | §7 cohort view. Query: `?standard=` or `?target_days=`, `?status=`. |
| `GET /api/care-pathways/{pathway}/constraints` | §8 ranked constraints for the cohort. |
| `GET /api/instances/flow` | §9 flow analysis. Query: `?window_days=` (default 90), `?pathway=`. |
| `GET /api/instances/time-standards` | §7.3 catalogue: thresholds, targets, authority, `as_of`. |

### 10.3 Response conventions

- Durations in **milliseconds** (`*_ms`) plus a rounded `*_days` for
  human display. Milliseconds because A&E is measured in hours and RTT
  in weeks, and one unit must serve both.
- Ratios as floats in [0, 1], never pre-multiplied percentages.
- Every ratio ships with its numerator and denominator.
- `confidence` (§6.6) on every analysis response.
- Nulls where a figure is undefined, each with a sibling `*_reason`.
  Never a sentinel zero.

## 11. Persistence

- One migration: `instance_segments` + the two `pathway_instances`
  clock columns, with a backfill from `enrolled_on` / `closed_on` so
  existing instances are analysable from the moment it runs.
- Indexes: `(instance_pid, started_at)` for the per-instance read,
  `(stage)` and `(category)` for cohort aggregation, and a partial index
  on open segments (`WHERE ended_at IS NULL`) for the one-open-segment
  invariant and the running-work query.
- No derived storage. No `efficiency` column, no materialised cohort
  rollup. Recomputation is cheap at this scale and a stored ratio would
  drift the moment a timestamp was corrected — precisely the correction
  TBA exists to invite.
- Row caps on the analysis reads, matching the existing `insights` cap
  (1000 pathways / bounded instances per cohort), so an unbounded cohort
  cannot become an unbounded query (security invariant 3).

## 12. Privacy, compliance, and anti-gaming

### 12.1 The data is personal data

A segment describes when a named subject was where, with whom. Combined
with `subject_ref` it is health data under GDPR Art. 9 and ePHI under
HIPAA. It therefore inherits the instance layer's posture without
exception: the blanket guard, record-level ABAC where the instance
carries it, masking obligations honoured on read, and audit on read for
the disclosure trail (§164.312(b) — reads are activity).

### 12.2 Aggregates must not re-identify

A cohort view over a small cohort discloses individual journeys by
arithmetic. Cohort responses therefore report `n`, and a cohort below a
small-number threshold returns the counts and the constraint ranking
without the percentile detail that would isolate one patient. A bulk or
aggregate read must never reveal more than the equivalent single read
(security invariant 5).

### 12.3 Anti-gaming is a design property

Waiting-time measurement is the most gamed metric class in health
systems. The mechanisms are well known — pause the clock, restart it on
re-referral, stop it on a milestone that is not treatment, exclude a
category. The defences here are structural, not procedural:

1. **No clock pause.** The clock runs from start to stop. Patient-caused
   delay is recorded as a *segment* (`unnecessary_non_value_adding`,
   waste `waiting`, labelled as patient deferral) and disclosed
   separately, so it is visible and subtractable by the reader — but it
   never silently shrinks the denominator.
2. **No stop-the-clock milestone.** The clock stops when the instance
   closes, which is a recorded lifecycle transition with its own audit
   row, not a metric-only event.
3. **No excluded category.** Every millisecond of the clock lands in
   exactly one of VA / NNVA / UNVA / unrecorded, and the four sum to LT
   by construction (a property test pins this).
4. **Unrecorded time counts against you** (§6.3), so under-recording is
   never a strategy.
5. **Coverage is disclosed** (§6.6), so a flattering ratio computed from
   a thinly-mapped journey is visibly thin.

### 12.4 Not a staff performance measure

Handoff and actor counts describe the *journey*, not the people. The
analysis deliberately provides no per-clinician throughput or duration
ranking. This follows §2.1's commitment: the frontline staff record the
data, and a method that turns their own records into their appraisal
destroys the data quality it depends on. Existing `care-team-load` stays
what it is — a workload view.

### 12.5 Audit and evidence

Segment writes and clock changes are audited like every other instance
mutation. Analysis reads on an identified subject are disclosure events
under §12.1. IEC 62304 traceability: each §15 task cites the §14 tests
that verify it.

## 13. Non-functional requirements

| Requirement | Target |
|---|---|
| Per-instance analysis | < 50 ms for ≤ 500 segments (single query + pure computation) |
| Cohort analysis | < 500 ms for ≤ 1000 instances; two bounded queries, no N+1 |
| Purity | §6–§9 computation has no I/O, no clock read (`as_of` is a parameter), and is deterministic |
| Never-panic | No `unwrap`, no arithmetic overflow, no division by zero on any input including reversed clocks and 10⁵-segment instances (security invariant 2) |
| Bounded input | Segment label/note within the family text caps; per-instance segment count capped; cohort reads capped (invariant 3) |
| Backward compatible | Existing instances analyse via the §5.2 date fallback; no existing endpoint changes shape |

## 14. Testing strategy

### 14.1 Pure unit tests (`src/tba.rs`, DB-free)

- Interval union: disjoint, touching, nested, identical, reversed input
  order, single point.
- Clipping: segment before / after / straddling each clock edge.
- Ratios: the four categories sum to LT; every ratio in [0, 1]; %VA ≤ %A.
- The Barker case: a journey with 14% VA reports 0.14, and the same
  journey with the VA segments alone recorded still reports 0.14 (the
  denominator rule, §6.3 — this is the regression test that stops the
  rule being "simplified" away).
- Coverage: unmapped journey → %VA 0 with `confidence: unmapped`.
- Overlap: concurrent VA segments give VT ≤ LT but φ > VT.
- Percentiles: nearest-rank on n = 1, 2, even, odd; p50 of a known
  sample; every percentile is an observed value.
- Gaps: leading, trailing, interior, none, whole-clock.
- Handoffs: no actors, one actor, alternating actors, location-only
  changes.
- Little's Law: λ = 0, μ = 0, and the three divergence labels.
- Degenerate: zero-length clock, reversed clock, stop before start,
  `as_of` before clock start — each a stated null, never a panic.

### 14.2 Boundary / validation tests

- Each §5.1 invariant returns `422` with a reason naming the field.
- `waste` on VA → refused; missing `waste` on UNVA → refused.
- Second open segment → refused while the first is open.
- Unknown stage / category / waste / standard → refused, listing the
  vocabulary.

### 14.3 Integration (DB-gated, `--ignored`)

- Record → analyse round trip: three segments, expected LT / VT / %A.
- Backfill: an instance created before the migration analyses from its
  `enrolled_on` fallback and says `start_source: "enrolled_on"`.
- Cohort: three instances, known percentiles, standard compliance.
- Audit rows written for segment and clock writes.
- Guard: with `CARE_PATHWAY_REQUIRE_AUTH=1`, every TBA path requires a
  token (the SEC-G8 default-off pin extended to the new surface).

### 14.4 Property tests

Over arbitrary segment sets: the four category totals sum to LT; VT ≤ PT
≤ LT; coverage ∈ [0, 1]; union total ≤ raw sum; no panic.

## 15. Tasks

The live queue for this cross-cutting section. Entity-wide tasks live in
[§13](13-tasks.md); these are cited from there rather than duplicated.

| id | Task | Verified by |
|---|---|---|
| **TBA-1** | Migration: `instance_segments` + clock columns + backfill + indexes (§11) | §14.3 backfill test |
| **TBA-2** | `SeaORM` entity + vocabularies (`STAGES`, `CATEGORIES`, `WASTES`) with the §5.1 invariant helpers | §14.2 |
| **TBA-3** | Pure `src/tba.rs`: union, clipping, per-instance metrics, gaps, handoffs (§6, §8.1, §8.4) | §14.1, §14.4 |
| **TBA-4** | Pure: percentiles, cohort aggregation, standards catalogue + compliance (§7) | §14.1 |
| **TBA-5** | Pure: flow analysis, Little's Law, divergence labels (§9) | §14.1 |
| **TBA-6** | Controller: recording endpoints (§10.1) with validation + audit | §14.2, §14.3 |
| **TBA-7** | Controller: analysis endpoints (§10.2), routes wired, OpenAPI documented | §14.3 |
| **TBA-8** | Front-end: the timeline wall + cohort view in `care-pathway-front-end-with-svelte` — **done 2026-08-23** (`/time`) | front-end vitest + Playwright |
| **TBA-9** | Cross-service journeys: link **and** traversal — **done 2026-08-24** (`GET /api/instances/{pid}/journey`) | §14.3, §16 |
| **TBA-10** | Small-number suppression on cohort percentiles (§12.2) — **done** with TBA-7 | §14.3 |
| **TBA-11** | Prometheus gauges for cohort %VA and p90 lead time per pathway — **done 2026-08-23** (`src/flow_metrics.rs`, default-off) | §14.1, §14.3 |

**Extensions queued in [§13 T-14](13-tasks.md)** (2026-09-03, triaged
from IPPA-py, the NHS BNSSG process-mining study, TreatmentPatterns, and
ehrapy): stage anchors + anchored standards (T-14d, which also settles
the anchor half of the "segment templates" question in §17),
censoring-aware cohort statistics (T-14e — today's `?status=all` mixes
running and completed lead times), rule-based cohort splits (T-14f), a
CONSORT attrition record on every cohort response (T-14g), a
data-quality report (T-14h), stalled journeys (T-14j), and the
generalised suppression rule with secondary suppression of marginals
(T-14k). The directly-follows map, journey variants, and template
conformance (T-14b, T-14c, T-14i) sit beside this section rather than
inside it: they are sequence analyses, not elapsed-time ones.

## 16. Implementation status

TBA-1 … TBA-7 are **implemented** in `care-pathway-service-with-loco`:

| Piece | Location |
|---|---|
| Migration | `migration/src/m20260823_000014_time_based_analysis.rs` |
| Entity | `src/models/_entities/instance_segments.rs` |
| Pure analysis + vocabularies + tests | `src/tba.rs` |
| HTTP surface | `src/controllers/tba.rs` |
| Routes | `src/app.rs` (`tba::routes()`, `tba::pathway_routes()`) |
| OpenAPI | `src/openapi.rs` (`tba_paths()`) |

TBA-10 (small-number suppression) landed **with** TBA-7 rather than
after it — `MIN_COHORT_FOR_PERCENTILES` in `src/controllers/tba.rs`,
pinned by the request test's `suppressed` assertion on a one-instance
cohort. It was listed here as open for a while after it was delivered,
which is the drift a §15 table invites; the entry now says so.

In `care-pathway-front-end-with-svelte` (TBA-8):

| Piece | Location |
|---|---|
| API client + presentation helpers | `src/lib/api/tba.ts` |
| The timeline wall | `src/lib/components/JourneyTimeline.svelte` |
| The view | `src/routes/time/+page.svelte` |
| Tests | `tests/unit/tba.test.ts`, `tests/e2e/time.spec.ts` |

TBA-11 (flow gauges) landed 2026-08-23: `src/flow_metrics.rs` (a
default-off refresh loop) plus the `care_pathway_flow_*` family in
`src/metrics.rs`. Two bounds are load-bearing rather than tidy, because
`/metrics.prom` is on the **public allow-list** and stays scrapeable
under enforcement: per-pathway series are **capped** (unbounded
cardinality takes the monitoring down), and cohorts below a floor are
**suppressed** on the same reasoning as §12.2 — a p90 lead time over
three patients is a patient's lead time, and the API's suppression
would be pointless if the same figure left through the exporter.
Neither bound is silent.

**TBA-9's three blockers were closed on 2026-08-24**, and it is worth
recording what each turned out to need, because "deferred" had been
hiding a contract change:

1. **The edge kind existed nowhere.** `entity-ref` gained `continues_as`
   plus two entity types — `care_pathway_instance` (a journey belongs to
   an enrolment, not to the template) and `patient_flow_stay` (the first
   type owned by a consumer application rather than a registry, because
   a journey does not stop at the registry boundary). All eight existing
   dependents still compile: the change is additive, and the exhaustive
   `match` arms in the registry are what forced each new variant to be
   given a sensitivity and an endpoint rule rather than defaulting
   quietly.
2. **This service originated no edges.** It now has an `entity_links`
   write-side — migration, model, `POST`/`GET`/`DELETE
   /api/instances/{pid}/links`, and the aggregator's reconciliation pull
   `GET /api/instances/links[?since=]` — following the case service's
   reference implementation. The `Envelope` gained an **additive**
   `data` field (`skip_serializing_if`, so existing CRUD envelopes stay
   byte-identical) and the `Linked`/`Unlinked` kinds, emitted through the
   transactional seam so an edge and its event share one commit.
3. **The governance was unwritten.** It is now
   [cross-service-linking.md §10.2](../../agents/share/cross-service-linking.md):
   `continues_as` is **high** sensitivity, authorised at the
   read-the-journey level (so a mental-health journey's edges inherit
   that pathway's protection), audited on every write, and its bulk pull
   gated as a privileged read — surfacing every journey edge at once maps
   which patients moved between which services, which is a different
   disclosure from reading one.

**The traversal landed the same day** as `GET
/api/instances/{pid}/journey` (`src/journey.rs`), answering its three
design questions rather than deferring them:

- **This service fetches, server-side.** The aggregator was the obvious
  alternative and is the wrong one: it is a link graph, it serves
  neighbours rather than segment detail, and giving it a timeline
  read-model would duplicate every owning service's data. Making the
  browser fetch each leg would need a credential per service, which is
  what the BFF pattern exists to avoid.
- **Under the caller's credential**, forwarded verbatim. A service
  identity would make this a **confused deputy** — a caller entitled to
  the pathway journey but not the inpatient stay would receive the
  stay's timeline anyway, because the far service would see only a
  trusted peer asking. No bearer presented ⇒ none forwarded, preserving
  the default-off posture instead of silently escalating.
- **A partial answer publishes no total.** Every leg carries a status
  and the resolved legs report their own figures, but the combined
  figures are `null` with a stated reason unless *every* leg resolved: a
  stitched lead time missing a leg is not imprecise, it is wrong,
  understated by exactly the part nobody could see. The span runs
  earliest-start to latest-stop, never the sum of the legs — the gap
  *between* episodes is real waiting and is usually the finding.

A **local** leg (a transfer to another pathway) needs no HTTP and no
configuration at all, so the commonest journey stitches out of the box.
A remote leg needs `CARE_PATHWAY_JOURNEY_URL_<TYPE>`; unset, the link is
still reported, its timeline simply not requested. The peer contract is
four numbers — clock bounds, lead time, value time — so participating
does not couple a service to another's domain model. This service's own
`/time-analysis` satisfies it unmodified.

**Remaining, and genuinely another service's work:** `patient-flow`
exposes no timeline endpoint yet, so a stay leg reports
`not_configured` until it does. The contract it would have to satisfy is
in `src/journey.rs`.

**What the front-end learned about §17's first open question.** The
view was expected to show whether a template should declare expected
stages with target durations. It shows something narrower: the cohort
view is perfectly usable against a *national* standard, and the case
for a per-template target only appears where no national standard
applies. That narrows the sidecar-table question rather than answering
it — the lean stands.

## 17. Open questions

- **Segment templates on the pathway.** Should a `CarePathway` template
  declare its expected stages with target durations, so an instance is
  measurable against *its own* plan rather than only against a national
  standard? Attractive, but it puts operational data into the matcher
  DTO — which §5.3 forbids. A sidecar table on the template pid is the
  likely shape. *Lean: sidecar, after TBA-8 shows what the UI needs.*
- **Automatic segmentation from steps.** `instance_steps.done_on` gives
  an ordered sequence of dates; consecutive completions imply intervals.
  Tempting, and rejected in §3 because a `done_on` date says when
  something finished, not when it started — the implied "segment" would
  be the *gap*, mislabelled as work. Revisit only if steps gain a start.
- **Small-number threshold.** Five is the common suppression floor; some
  regimes use ten. Deployment-configurable, or fixed at the stricter
  value? *Lean: fixed at 5, configurable upward only.*
- **Percentile method.** Nearest-rank (§7.1) versus linear interpolation
  as in most SQL engines. Nearest-rank chosen for explicability; a
  consumer comparing against `percentile_cont` will see small
  differences. Document, or offer both?
- **Window default.** 90 days for flow analysis is arbitrary. It should
  be a multiple of the cohort's own p90 lead time, which is circular.
  *Lean: keep 90, document the circularity.*
- **Instances/insights OpenAPI gap.** The pre-existing instance and
  insight endpoints are not in `openapi.json`; TBA now is. Close the gap
  by documenting the others rather than by undocumenting this.

## 18. References

### Time-based analysis

- Barker, R. C. — *The Time Based Organisation: Recreating and
  Transforming Existing Organisations*.
- [Time Based Analysis in the UK NHS](https://www.drbobbarker.co.uk/post/time-based-analysis-in-the-uk-nhs)
  — the 8% and 14% value-adding figures and the tracking method.
- UK Parliament Health and Social Care Committee publications on
  elective recovery and patient flow.

### Value stream mapping and queueing theory

- [value-stream-mapping](https://github.com/joelparkerhenderson/value-stream-mapping)
  — VA / NNVA / UNVA, VT / PT / LT / %A / #HO / RFPY / %C&A, the eight
  wastes.
- [queueing-theory](https://github.com/joelparkerhenderson/queueing-theory)
  — λ / μ / ρ / τ / ω / φ / κ, Little's Law κ = λτ.
- Lean Enterprise Institute — value stream mapping lexicon.

### NHS access standards

- [NHS England — Referral to treatment (RTT)](https://www.england.nhs.uk/rtt/)
  — clock start/stop rules; 92% within 18 weeks on incomplete pathways.
- [NHS England — Cancer waiting times standards](https://www.england.nhs.uk/clinically-led-review-nhs-access-standards/cancer/)
  — 28-day FDS, 31-day and 62-day standards.
- [NHS England — Medium Term Planning Framework 2026/27 to 2028/29](https://www.england.nhs.uk/long-read/medium-term-planning-framework-delivering-change-together-2026-27-to-2028-29/)
  — DM01 and A&E trajectories.

### Family cross-references

- [`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md) — HIPAA audit controls, GDPR/EHDS.
- [`agents/share/security.md`](../../agents/share/security.md) — the invariants cited in §12–§13.
- [`agents/share/privacy.md`](../../agents/share/privacy.md) — masking on every read path.
- [`06-functional-requirements.md`](06-functional-requirements.md), [`09-api-surface.md`](09-api-surface.md), [`10-persistence.md`](10-persistence.md), [`12-compliance.md`](12-compliance.md).
