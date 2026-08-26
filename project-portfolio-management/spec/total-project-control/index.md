# Total Project Control

**Total Project Control** (TPC) is Stephen A. Devaux's method for
managing a project **as an investment**: every tracking decision is
referred back to one question — *is the value still to come worth the
money still to spend?* Earned value management asks whether we are
conforming to a baseline (SPI, CPI); TPC asks whether continuing is
rational at all, and its central index is built so that **sunk cost
appears nowhere in it**. That matters here because this entity already
carries budgets, benefits, forecasts and a critical path — the inputs
TPC needs are mostly the ones a portfolio office already records.

This file is a **map, not a transcription**: what Devaux's method
defines, what this entity implements, and what it deliberately does
not. The pattern follows [prince2](../prince2/index.md) and
[flow-framework-metrics](../flow-framework-metrics/index.md).
Implemented in `src/tpc.rs` (pure rules), `src/controllers/tpc.rs`
(routes, spec §9.2c), and migration
`m20260825_000001_total_project_control` (entity spec §5.9.7 / FR-37;
delivered under [§13 T-25](../13-tasks.md)).

## 1. The premise — a project is an investment

Devaux's argument (*Total Project Control*, 2nd ed. 2015; *Managing
Projects as Investments*, 2014): schedule and cost conformance are
proxies, and a project that is perfectly on its baseline can still be
one that should be cancelled. The tracking figure should therefore be a
**profitability index over the remaining work**, recomputed as
estimates change — and because it divides remaining value by remaining
cost, money already spent cannot influence it. That is not an
implementation detail; it is the method's answer to the sunk-cost
fallacy, and this service pins it with a test
(`sunk_cost_cannot_influence_the_figure`).

## 2. The vocabulary

What TPC defines, in Devaux's terms:

| Term | Definition |
|---|---|
| **EMV** — expected monetary value | The expected value of the project *if completed*: benefit estimate weighted by probability. May legitimately be **negative**. |
| **CEC** — cost estimate to complete | The money still to spend. Never negative; zero means the project is finished, not infinitely attractive. |
| **DIPP** — Devaux's Index of Project Performance | `(EMV ± acceleration premium − delay cost) / CEC`. Above 1.0 the value still to come exceeds the money still to spend; below 1.0, continuing destroys value *whatever has been spent*. From "When the DIPP Dips" (1992). |
| **DPI** — DIPP Progress Index | Actual DIPP ÷ planned (baseline) DIPP at the same point in the schedule. At or above 1.0 the project is tracking its own plan as an investment. |
| **Cost of time** | The project-level value lost (or gained) per unit of delay (or acceleration) — the source of the DIPP's delay-cost and acceleration-premium terms. |
| **Critical-path drag** | For an activity on the critical path: how much *it alone* is adding to project duration — the time the project would save if that activity's duration were zero. **Drag cost** = drag × cost of time, i.e. the money one activity's duration is costing the project. |
| **DRED** — doubled-resource estimated duration | An activity-level second estimate: how long with double the resources? A cheap way to see where money can buy time. |
| **CLUB** — cost of leveling with unresolved bottlenecks | The value lost to a resource bottleneck that leveling worked around instead of fixing — the case for funding the bottleneck. |
| **VBS** — value breakdown structure | The WBS's value mirror: each activity carries the value it *adds* to the project's EMV, so scope decisions are value decisions. |

## 3. What is implemented

The first two rows of the table above and their ratio machinery — the
`total_project_control` table, the pure derivation, and four routes.

### 3.1 Schema — the data dictionary

Migration `m20260825_000001_total_project_control`; one observation row
per `POST`, per plan, newest-first. Money columns are `NUMERIC`, read
by the service as **integer minor units** of `currency`; ratios are
**basis points** (10 000 = 1.0). No float touches a currency figure at
any layer.

| Column | Meaning |
|---|---|
| `currency` | ISO 4217. Comparisons and rankings never cross it — this service converts currency nowhere. |
| `total_project_control_dipp` | The **stored** DIPP, optional. May carry the TPC time-value terms (acceleration premium, delay cost) that EMV alone does not — see §3.3 on divergence. |
| `total_project_control_expected_monetary_value` | EMV, minor units. **Deliberately unchecked for sign**: a project can be worth less than nothing to finish, and that is the case the metric exists to expose. |
| `total_project_control_cost_estimate_to_complete` | CEC, minor units. `CHECK (>= 0)` — no cost estimate to complete is negative. |
| `total_project_control_dipp_progress_index_numerator` | Actual DIPP at this point in the schedule, basis points. |
| `total_project_control_dipp_progress_index_denominator` | Baseline DIPP at the same point, basis points. |
| `total_project_control_dipp_progress_index_ratio` | The DPI. **`GENERATED ALWAYS AS (numerator / NULLIF(denominator, 0)) STORED`** — a ratio written by a handler beside its own inputs can disagree with them after any later edit; a generated column cannot, and Postgres refuses a hand-written value. A zero baseline yields `NULL`, not a division error. |

### 3.2 Endpoints

| Route | What it answers |
|---|---|
| `POST /api/plans/{pid}/tpc` | Record one observation. Negative EMV accepted; negative CEC refused (`422` **and** CHECK); currency validated as ISO 4217 shape. Audited (`tpc_recorded`). |
| `GET /api/plans/{pid}/tpc` | The observation history, newest first. |
| `GET /api/plans/{pid}/tpc/report` | The derived view over the newest observation: computed DIPP, band, DPI, stored-vs-computed divergence, echoed inputs. A plan with no observation reports `measured: false` with a reason — the same *unmeasured-is-not-zero* posture as the OKR and Smart Score views. |
| `GET /api/tpc?currency=` | Portfolio triage: plans ranked **highest DIPP first** — Devaux's intended use, scarce resources to the project returning most per remaining pound. |

### 3.3 Derivation properties worth knowing

All in `src/tpc.rs`, each pinned by a unit test; verified end to end
against Postgres 18 (§13 T-25).

- **Undefined is not zero and not infinity.** `CEC = 0` reports `null`
  with a reason (`zero_denominator`) — nothing left to spend is the end
  of a project, not an infinitely good one. Same for a zero DPI
  baseline.
- **A negative EMV is not clamped.** It computes, and bands
  `value_destroying` — the finding the metric exists for. The bands are
  deliberately three (`value_destroying` / `below_break_even` /
  `at_or_above_break_even`): 1.0 is the only threshold Devaux's
  arithmetic supplies, and inventing more would dress a judgement as a
  measurement.
- **Stored-vs-computed divergence is a finding, not an error.** The
  stored DIPP may legitimately carry time-value terms EMV/CEC alone do
  not (§2's acceleration premium and delay cost); the report states the
  gap rather than preferring either. This is how the time-value half of
  Devaux's formula is represented today — asserted inside the stored
  DIPP, not modelled as its own columns (see §4).
- **Triage sets aside rather than mis-ranks.** A plan recorded in
  another currency, or one whose DIPP is undefined, is excluded *and
  reported* — either would otherwise sort as if measured and bad.
- **Every figure is labelled `asserted`.** EMV and CEC are estimates a
  person supplied, never observations — the same labelling rule as the
  value-realization module (§5.9.6).
- **Checked arithmetic throughout.** Operator-supplied `i64` extremes
  return `overflow`, never panic (`agents/share/security.md`
  invariant 2).

## 4. Deliberately not implemented

Each of these is a real TPC concept whose *input* this service does not
hold; computing it from absent inputs is what the §3.3 refusals exist
to prevent.

- **Critical-path drag and drag cost.** The near-miss worth naming:
  the schedule view (`GET /api/plans/{pid}/schedule`) already computes
  a critical path over the finish-start `plan_dependencies` — but drag
  needs per-activity **durations** on that path, and dependencies here
  connect *plans*, while tasks carry no duration estimates at all.
  Without durations, drag is uncomputable, and a "drag" figure derived
  from anything else would be an invented number wearing Devaux's name.
- **Cost of time as a first-class curve.** Delay cost and acceleration
  premium exist only as whatever the operator folded into the stored
  DIPP (surfaced as divergence, §3.3). Modelling them properly means a
  per-plan value-over-time function — a genuine model addition, not a
  column.
- **DRED.** Needs a second duration estimate per task; no task carries
  even a first.
- **CLUB.** Needs resource-leveling over declared resource pools. The
  capacity/utilisation machinery (FR-35) reports load against declared
  working time, but nothing here levels a schedule against it.
- **VBS.** Needs per-activity value attribution. The nearest existing
  surfaces — `benefits`, `business_case_targets`, and the OKR alignment
  weights — attribute value to *plans and objectives*, not to
  activities.
- **Automated DIPP tracking.** Observations are explicit `POST`s, not
  derived from `budget_lines` actuals or the forecaster. Deriving CEC
  from recorded budgets is plausible future wiring; today the figures
  are asserted whole, and labelled so.

## 5. Relationship to the neighbours

- **[PRINCE2](../prince2/index.md) §1** maps *continued business
  justification* onto DIPP — the same question, arithmetically.
- **The controls register (FR-38)** can watch TPC: a control standard
  over a DIPP threshold turns "the DIPP dipped below 1.0" into a
  recorded, answerable exception rather than a chart nobody read.
- **The value-realization module (§5.9.6 / FR-33)** is TPC's
  after-the-fact complement: DIPP is forward-looking (value still to
  come ÷ cost still to spend); realized gains measure what actually
  arrived, on the same minor-units / basis-points / single-currency
  conventions.

## References

- Stephen A. Devaux — *Total Project Control: A Practitioner's Guide to
  Managing Projects as Investments*, 2nd ed., CRC Press, 2015.
- Stephen A. Devaux — *Managing Projects as Investments: Earned Value
  to Business Value*, CRC Press, 2014.
- Stephen A. Devaux — "When the DIPP Dips: A P&L Index for Project
  Decisions", *Project Management Journal*, September 1992.
- Wikipedia — <https://en.wikipedia.org/wiki/Critical_path_drag>
- Wikipedia — <https://en.wikipedia.org/wiki/Devaux%27s_Index_of_Project_Performance>
