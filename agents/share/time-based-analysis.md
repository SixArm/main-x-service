# Time-based analysis (TBA) — design

How the Main X Index family measures **elapsed calendar time** through a
process, and what it refuses to do with the result. This is a design
document: it fixes the measurement model, the vocabulary, the response
conventions, the anti-gaming properties, and the per-entity adoption, so
each crate adopts it without re-litigating.

It exists because two subprojects — [care-pathway](../../care-pathway/spec/time-based-analysis.md)
(a patient journey) and [project-portfolio-management](../../project-portfolio-management/spec/time-based-analysis.md)
(a delivery board) — implemented the same measurement model
independently, and the second one found errors in the first one's
reasoning. A third adopter should inherit the corrections rather than
rediscover them.

## 1. The question, and why it is one question

TBA asks one thing of a process: **of the calendar time this took, how
much of it was the work?**

The answer is reliably small, and remarkably consistent across domains
that share nothing else:

| Domain | Value-adding share | Source |
|---|---|---|
| NHS patient journeys | **8–14%** | Dr. R. C. Barker's tracked journeys |
| Knowledge work / delivery boards | **5–15%** | the flow-metrics literature |
| Manufacturing and services generally | **under 15%** | *The Time Based Organisation* |

That convergence is the reason this is one contract and not two. A
patient waiting for a scan and a pull request waiting for review are the
same phenomenon measured in different units, and the arithmetic that
gets either one wrong gets both wrong.

**The consequence that reframes improvement work.** If a unit of work is
worked on 6% of its life, making the work 20% faster improves delivery
by about 1%, while removing half the waiting improves it by nearly half.
Every metric that measures the 6% — utilisation, velocity, department
throughput, touch time — is measuring the wrong thing, and measuring it
precisely.

## 2. The three literatures

Each supplies a distinct piece. Keeping them distinct matters, because
their vocabularies overlap and their metrics do not.

- **Barker's time-based analysis** supplies the *headline ratio and the
  method*: track one unit end to end, record every phase, divide
  value-adding time by elapsed calendar time. Its three commitments —
  the whole journey or nothing; the people doing the work record the
  times; untapped capacity rather than more capacity — are what make the
  ratio actionable rather than merely damning.
- **Value stream mapping** supplies the *classification and metric
  names*: VA / NNVA / UNVA, and LT / PT / VT / %A / #HO / RFPY. See
  [value-stream-mapping](https://github.com/joelparkerhenderson/value-stream-mapping).
- **Queueing theory** supplies the *flow mathematics*: λ / μ / ρ / τ / ω
  / φ / κ, and Little's Law κ = λτ. See
  [queueing-theory](https://github.com/joelparkerhenderson/queueing-theory).

**The VA/NNVA/UNVA split is not pedantry.** A blanket "non-value-adding"
figure invites the true reply *"but the safety checklist is not waste"*,
or *"code review is not waste"*. It is not. Conceding necessary
non-value-adding work as its own category is what makes the remaining
number arguable, and therefore actionable.

## 3. The measurement model

### 3.1 The denominator is elapsed calendar time

**Never the sum of recorded activity.** This is the load-bearing rule
and the one most likely to be "simplified" away by a later edit.

If the denominator were the sum of what was recorded, a service that
records only its value-adding work would report 100% — recording *less*
would score *better*, which is the exact inversion the method exists to
expose. Unrecorded time counts as non-value-adding.

**Every adopter must pin this with a regression test**: the same journey
with only its value-adding segments recorded must report the same ratio
as the fully-mapped one.

### 3.2 Coverage travels with every ratio

An unmapped process reports a ratio near zero that looks identical to a
catastrophically wasteful one. The two must never be confused, so a
coverage figure and a confidence label ship with every ratio, and a UI
renders "we do not know" differently from "catastrophic".

Where the intervals are *derived* rather than recorded (§4.2), coverage
is structural and the disclosable equivalent is the share of history
that was **synthesised** rather than observed.

### 3.3 Every millisecond lands in exactly one bucket

The categories partition the measured span by construction and sum to it
exactly — a property test, not a promise. There is no residual category
to hide time in, and no status excluded from the accounting.

An **unclassified** state falls to the *pessimistic* bucket, so adding a
stage or a board column can never silently improve the ratio.

### 3.4 Overlap is unioned, not summed

Where concurrent work is possible, the value-adding total is the
**interval union** — so the ratio is provably in [0, 1] — and the raw
sum is reported separately as touch time (φ), which may exceed elapsed
time and is a resource-effort figure, not a duration.

Where intervals are derived from one ordered log they cannot overlap,
and the union collapses to a sum. That is a property of the source, not
a licence to drop the distinction from the model.

### 3.5 Two clocks, never conflated

Every adopter reports both, and never one labelled as the other:

| | Meaning |
|---|---|
| **Lead time** | Created → finished. What the requester experiences. |
| **Cycle time** | Started → finished. What the team controls. |

The gap between them is the queue before work began. Quoting cycle time
as "our delivery time" is the commonest misreport in the field, it is
always flattering, and it can be an order of magnitude.

## 4. Where the intervals come from

Two shapes, and the choice is forced by the domain rather than by taste.

### 4.1 Recorded intervals (care-pathway)

A person maps the journey by hand, recording each segment with its
start, end and classification. Nobody logs a clinic's minutes
automatically, so the data has to be entered — and coverage is therefore
genuinely partial and must be disclosed (§3.2).

### 4.2 Derived intervals (portfolio)

A status-transition log is written by the endpoint that already performs
the move, and the intervals fall out of consecutive transitions. **This
is strictly better where it is available**: collection is a by-product
of the work, so there is nothing extra to keep up to date, and the data
does not depend on anybody's diligence.

The rule for a new adopter: **if the process already has a state-change
API, derive; only record by hand when it does not.** A method that asks
people to log hours will get logged hours, not true ones.

Two properties the derived shape must hold:

- **The log is append-only.** No edit, no delete. An editable flow log
  measures whatever the editor wanted; correcting history means moving
  the item again, which is itself recorded.
- **The transition commits in the same transaction as the change.** A
  committed change without its transition silently shortens the item's
  recorded life, and nothing downstream can tell.

## 5. Statistics

- **Percentiles, never means.** These distributions are right-skewed;
  the mean describes no actual unit. Report min / p50 / p75 / p85 or p90
  / p95 / max, and label the mean as the skew-sensitive figure.
- **Nearest-rank**, stated in the payload, so every percentile is an
  observed unit and "which one is the p90?" has an answer.
- **Aggregate versus median is itself a finding.** `Σ VA / Σ elapsed`
  is the system's ratio and is dominated by the longest-running units;
  the median of per-unit ratios is the typical one. A large divergence
  means the waste is concentrated in a minority — a different
  intervention from uniformly slow flow.
- **A rollup unions the underlying units; it never averages ratios.**
  Averaging weights a five-item set equally with a five-hundred-item
  one.
- **Refuse below a minimum sample.** A forecast or an expectation
  derived from a handful of observations is arithmetic on noise, and a
  confident-looking figure derived from nothing is what discredits the
  method. Return a stated reason, not a number.

### 5.1 The threshold comes from outside or from history

| Source | Use when |
|---|---|
| **An external standard** (NHS RTT 18 weeks, the cancer standards, DM01, A&E) | The domain has a published, externally-set threshold. Ship it as reference data **with a citation date** — targets move, and a stale threshold silently mis-scoring a cohort is worse than none. |
| **The set's own history** (a service level expectation: "85% finish within N") | There is no external standard. Strictly better in one respect: it cannot be argued with on grounds of local difficulty. |

### 5.2 Forecasting: throughput, not cycle time

**This is the error the second adopter found in the first's spec, and it
is the standard error in the field.**

- The **cycle-time** distribution forecasts **one unit** ("this will
  finish within 11 days at 85%"). That is the service level expectation.
- The **throughput** distribution — how many units completed per period
  — forecasts a **batch** ("these 20 will be done in N weeks").

Building a batch forecast from cycle times implicitly assumes units are
worked one at a time. Sum twenty cycle times for a team running five in
parallel and the answer is roughly five times too pessimistic.
Throughput sampling makes no such assumption, because parallelism is
already baked into the counts.

Two further properties:

- **The conservative percentile reverses between the two questions.**
  For *how long*, higher is conservative. For *how many*, it is the
  **low** end — "at least this many, with 85% confidence" is the 15th
  percentile, not the 85th. Name the fields for what they mean, not for
  the percentile they came from.
- **A forecast must be deterministic.** Seed the simulation. One that
  changes every time you reload it is not one anybody will act on.

## 6. Anti-gaming is a design property

Waiting-time measurement is the most gamed metric class in both health
systems and delivery organisations, and the mechanisms are known. The
defences are structural rather than procedural, and every adopter
inherits all of them:

1. **No clock pause.** The clock runs from start to stop. Caller-caused
   or externally-caused delay is recorded as a *category*, visible and
   subtractable by the reader, never silently shrinking the denominator.
2. **No stop-the-clock milestone.** The clock stops on a recorded
   lifecycle transition with its own audit row, never on a metric-only
   event.
3. **No excluded category** (§3.3), and no "on hold" state.
4. **No business-hours discounting.** A weekend in review really was a
   weekend in review. Working-hours arithmetic is the standard way to
   make queues disappear from a report while the customer still waits.
5. **Unrecorded time counts against you** (§3.1), so under-recording is
   never a strategy.
6. **The flattering number never travels alone.** Cycle time is always
   reported beside lead time; throughput is always reported beside
   first-pass yield, so shipping work back to yourself cannot read as
   going faster.

## 7. It is never a person metric

**A stated refusal, not an unbuilt feature.** No per-clinician, per-
assignee or per-operator cycle time, throughput, or efficiency. Three
reasons, in ascending order of cost:

- **It measures the wrong 6%.** If a unit is worked on 6% of its life,
  per-person speed addresses 6% of the problem. The queue belongs to the
  system.
- **It is confounded.** Duration depends on what the unit was, who else
  was needed, and what it waited on. Attributing it to whoever held it
  last is not a measurement.
- **It destroys the data.** Where collection is a by-product of the work
  (§4.2), the measurement survives only as long as nobody has a reason
  to distort it. Turning it on individuals supplies that reason —
  people will split items, skip states, and sit on statuses to look
  good.

Handoff *counts* are a property of the unit's journey and are reported.
Actor identity is available for audit and for "who should be asked about
this", never as a ranked comparison.

## 8. Privacy, and the exporter side door

TBA data describes when a named subject was where, with whom. It is
personal data even when its subject is a task, and it inherits the
service's posture: the blanket guard, record-level ABAC, masking
obligations, and audit on read.

**Aggregates re-identify by arithmetic.** A p90 over three patients *is*
a patient's figure; a flow efficiency over two tasks describes two
people's week. Cohort responses report `n` and withhold percentile
detail below a small-number floor.

**The floor must hold on every surface, or it holds on none.** The
Prometheus endpoint is on the family's **public allow-list** — it stays
scrapeable with `<ENTITY>_REQUIRE_AUTH` on — so an exporter that
publishes what the API withholds is a side door, and a wide one. Any
metrics adoption applies the same suppression, plus a **series cap**
(per-record labels are unbounded cardinality, and a metric that takes
the monitoring down is worse than no metric). Neither bound may be
silent: export the suppressed and dropped counts alongside.

## 9. Response conventions

- Durations in **milliseconds** (`*_ms`) plus a rounded `*_days` for
  display. Milliseconds because A&E is measured in hours and RTT in
  weeks, and one unit must serve both.
- Ratios as floats in [0, 1], never pre-multiplied percentages, and
  **always with their numerator and denominator**, so a consumer can
  re-aggregate without trusting our rounding.
- A null figure carries a sibling **reason**. Never a sentinel zero: an
  undefined ratio is not a ratio of nothing, and a refused forecast
  rendered as `0` becomes a claim of instant delivery.
- Every response carries `as_of` and a `note` describing the derivation,
  and repeats any **classification map** in force — so a figure cannot
  be compared across two deployments without the difference being
  visible.
- Every cap that fires says so. A view that quietly covers half an
  estate reads as if it covered all of it.

## 10. Purity

The computation is a pure function of `(intervals, clock, as_of)` — no
I/O, and **no clock read**: `as_of` is a parameter. That is what makes
the whole model testable without a database, and it is why the hard
cases (degenerate clocks, reversed intervals, future transitions, clock
skew, overlapping concurrent work) are covered by unit tests rather than
hoped about.

Adopters put the computation in `src/tba.rs` and the HTTP surface in
`src/controllers/tba.rs`.

## 11. Per-entity adoption

The contract is identical; only these differ.

| Adopter | Interval source | Threshold | Notes |
|---|---|---|---|
| **care-pathway** | Recorded segments (§4.1) | NHS access standards | The reference for the recorded shape, and for external-standard scoring. |
| **portfolio** | Derived from a task-transition log (§4.2) | The plan's own SLE | The reference for the derived shape, for rework/RFPY, and for forecasting. |

Neither is a fork of the other: the measurement model is shared, and
each spec carries a **differences table** naming exactly what diverges,
so the two cannot drift apart without somebody editing that table.

A new adopter declares: its interval source, its stage/status
vocabulary and classification, its threshold source, and its
suppression floor.

## 12. Not yet family-wide, and why

Eight of the ten entity registries carry no TBA, and that is a
scope decision rather than a gap. TBA needs a process with **duration**
— a unit that enters, waits, is worked on, and leaves. A registry of
identities (person, place, thing, organization) has records, not
journeys; there is no elapsed time to measure because nothing is *in
progress*. `case` and `patient-flow` are the plausible next adopters,
because both track a unit through states over time.

## 13. Open questions

- ~~**Cross-service journeys.**~~ — **RESOLVED 2026-08-24**, link and
  traversal both ([cross-service-linking.md](cross-service-linking.md)
  §9, §10.2). The three answers worth carrying to another adopter:

  - **The owning service fetches, not the aggregator.** A link graph
    serves neighbours; giving it a timeline read-model would duplicate
    every owning service's data to answer a question those services can
    already answer.
  - **Under the caller's credential, forwarded** — never a service
    identity. With a peer token the fetching service becomes a
    **confused deputy**: a caller entitled to one leg but not the next
    receives both, because the far service sees only a trusted peer
    asking.
  - **A partial stitch publishes no total.** A stitched lead time
    missing a leg is not imprecise, it is *wrong* — understated by
    exactly the part nobody could see. Every leg carries a status, the
    resolved legs report their own figures, and the combined figures are
    null with a stated reason. The span is earliest-start to
    latest-stop, never the sum of the legs: the gap *between* episodes
    is real waiting and is usually the point.

  The peer contract is deliberately four numbers (clock bounds, lead
  time, value time), so participating does not couple a service to
  another's domain model. `patient-flow` adopted it on 2026-08-24 via
  `GET /api/stays/{pid}/time-analysis`.

  **What that adoption showed: a domain usually already has the
  answer.** Patient-flow did not have to invent a notion of
  value-adding time — **Red2Green** is exactly that measure in the
  NHS's own vocabulary (a green day moves the patient toward discharge;
  a red day does not). Its unclassified days count as non-value-adding
  for the same reason unrecorded segments do, so its figure is a floor
  and it reports its own coverage. A new adopter should look for the
  existing domain concept before defining one: the classification a
  service already records is more likely to be trusted, and more likely
  to be right, than a new field asking people to re-state it.
- **Waiting versus working inside one state.** A state like `in_review`
  conflates "in someone's queue" with "being read", and the constraint
  ranking keeps naming it. Splitting it makes the largest waste block
  legible at the cost of another state, which operators resist.
- **A shared crate.** The measurement model is now implemented twice.
  Whether `src/tba.rs` becomes a published crate or stays copy-adapted
  is the same call the family made for `mxi-events` and `EntityRef`.
  *Lean: copy-adapt while there are two adopters; extract at the third,
  and note that `EntityRef` reached eight consumers as a path dependency
  before anyone formalised it.*

## 14. References

- Barker, R. C. — *The Time Based Organisation*; [Time Based Analysis in the UK NHS](https://www.drbobbarker.co.uk/post/time-based-analysis-in-the-uk-nhs).
- [value-stream-mapping](https://github.com/joelparkerhenderson/value-stream-mapping) · [queueing-theory](https://github.com/joelparkerhenderson/queueing-theory).
- Little, J. D. C. — *A Proof for the Queuing Formula L = λW*.
- Reinertsen, D. — *The Principles of Product Development Flow*.
- [NHS England — RTT](https://www.england.nhs.uk/rtt/) · [cancer standards](https://www.england.nhs.uk/clinically-led-review-nhs-access-standards/cancer/).
- Per-subproject specs: [care-pathway](../../care-pathway/spec/time-based-analysis.md) · [portfolio](../../project-portfolio-management/spec/time-based-analysis.md).
