# Pillar 4 — Talent management & development

## Performance reviews

Review **cycles** (period + status open/calibrating/closed) contain
one review per employee: goals (title, due, status), continuous
**feedback entries** (attachable any time, not just in cycles), and
the appraisal (`draft → submitted → calibrated → shared`) with a 1–5
rating. Calibration happens at cycle level (HR flips submitted
reviews to calibrated after moderation); `shared` releases the
review to the employee's self-service view. Review content is
sensitive — reads are audited and ABAC-scoped (self + manager + HR).

## Learning (LMS via the course registry)

WPM deliberately does **not** host courses — the family's
[course-service](../../course/course-service-with-loco/) owns course
identity and offerings. WPM owns **TrainingEnrollments**: employee ×
`course:` / `courseinstance:` URN, status (`enrolled → completed |
failed | withdrawn`), completion date, and an optional certification
expiry. The compliance view lists employees with missing mandatory
trainings or **expiring certifications** (next 90 days) — the
strategic reason enrollments live here. Course names are resolved
best-effort (the display-name client pattern), never copied as truth.

## Succession planning

Succession plans name a key **position** (title + department +
criticality 1–5) and its incumbent, then rank a pipeline of
candidate employees by **readiness** (`ready_now` / `ready_1y` /
`ready_2y`) with development notes. The dashboard surfaces uncovered
critical positions (criticality ≥ 4 with no `ready_now` candidate) —
succession as a measurable gap list, not a slide deck. Succession
data is HR-persona-only under ABAC.

A plan also records the incumbent's **risk of loss** (`low` /
`medium` / `high`) and, when known, the date the role is expected to
fall vacant. Exposure is the product of the two, so the
single-point-of-failure list
(`GET /api/workforce-intelligence/succession`) flags an uncovered
role at criticality ≥ 4 — or ≥ 3 when the incumbent is a high
flight risk.

**Bench coverage** is deliberately conservative:

| Coverage | Meaning |
|---|---|
| `covered_now` | at least one `ready_now` successor |
| `covered_soon` | none now, at least one `ready_1y` |
| `developing` | only `ready_2y` successors |
| `uncovered` | no successors at all |

Both judgements go stale faster than anything else in WPM, so both
are updatable in place: `PUT /api/succession-plans/{pid}` restates
criticality / risk / vacancy date / incumbent, and
`PUT /api/succession-candidates/{pid}` moves a successor's readiness
or rank. Readiness may move **down** — a bench that can only improve
on paper would overstate the organisation's cover.

## Assessments — aptitude, personality, psychometric, selection

WPM records the tests people sit, for hiring and for development.
Three record kinds: an **instrument** catalog (the named test, its
category, the scales it reports, its duration and validity), an
**assessment** (one administration to one candidate or employee,
optionally tied to an application), and per-scale **results**.

| Category | Measures | Scales |
|---|---|---|
| `aptitude` | how a person performs at tasks and reacts to situations | numerical reasoning, verbal reasoning, problem solving, logical thinking |
| `personality` | behavioural style and working qualities | work style, team compatibility, introversion/extraversion |
| `psychometric` | **spans aptitude, personality, and cognitive** | behavioural style, emotional intelligence, cognitive ability — *plus* every aptitude, personality, and cognitive scale |
| `selection` | suitability for a role during hiring | job simulation, skills assessment, judgement test |
| `cognitive` | IQ-style index measurement (WPM-R20) | verbal comprehension, working memory, processing speed, spatial reasoning, fluid reasoning |

A result whose scale is outside its assessment's category is a `422`.
Psychometric is the one deliberate overlap, because a full battery
covers aptitude, personality, *and* cognition by definition. Two
cognitive-specific guardrails: **no composite "IQ number" exists**
(per-scale readings only), and `selection` instruments **refuse**
cognitive scales — an IQ scale cannot ride into hiring unreviewed;
equality-law review before any selection use is a deployment duty.

**Scores are integers**: percentiles are 0–100 and raw scores are
whole points out of a whole maximum. A percentile derives a **band**
(`low` < 10, `below_average` < 30, `average` < 70, `above_average` <
90, `high` ≥ 90) unless the instrument reports one directly.

**Lifecycle**: `scheduled → in_progress → completed → expired`, with
`cancelled` from any open state. Completing requires at least one
recorded result — "completed" must not assert a scoring that never
happened — and derives `expires_on` from the instrument's declared
validity, so currency is a recorded fact rather than a read-time
assumption.

**Sensitivity.** Results profile cognition and behaviour, so they are
treated like salary and payslips: every read path honours the ABAC
`mask` obligation (the scale and band survive; raw scores,
percentiles, and narratives do not), and unmasked reads of scored
results are audited. The aggregate analytics
(`GET /api/assessments/analytics`) report counts and band
distributions only — no individual's score appears.

## 360° multi-rater appraisals

Alongside the manager-led review cycles, a **360° appraisal**
(WPM-R29) gathers a full circle: manager, peers, direct reports, and
a self-assessment, per declared competency. The lifecycle is one-way
(`draft → collecting → shared`), collection requires at least three
non-self raters (≤ 12 total), responses are once-per-rater and
`$sub`-owned, and every rater gets an in-app request notification
(WPM-R31). Rater anonymity is **procedural** (WPM-D21): who responded
is visible (chasing is half the process), what they said is only ever
a group × competency aggregate — `peer`/`report` cells below three
responses are withheld, count included; `manager` and `self` disclose
at one by convention. The shared-only report pools comments per group
(alphabetised; withheld from masked readers), its reads are audited,
and it is development-facing — never a payroll or benchmarking input.

## Skills, learning paths, and mentorship

The skills catalog + declared per-employee proficiencies (1–5, with
optional targets) feed the skills matrix and gap views; **learning
paths** are ordered course sequences whose per-member progress counts
only completed training enrolments; **mentorships** pair employees
(`proposed → active → completed`) with a session log, surfaced in the
mentorship overview (load, unmatched, stale pairings).

## Upskilling and reskilling plans

A **development plan** is one employee's route from where they are to
where the organisation needs them, in declared skill steps.

- **`upskill`** — deepen the skills of the **current** role. It must
  not name a target role.
- **`reskill`** — build the skills for a **different** role. It must
  name the target (`target_job_title` and/or `target_department`), and
  naming the employee's current role is refused as an upskill by
  another name.

Each item pairs a catalog skill with a `current_level` → `target_level`
step on the shared 1–5 proficiency scale (the target must be higher —
a step that does not raise the level is not development), a method
(`course`, `mentorship`, `on_the_job`, `apprenticeship`, `internship`,
`self_study`), an optional `course:` ref, and a due date.

Plans report **two** progress figures, and the difference between them
is the point:

| Figure | Meaning |
|---|---|
| declared progress | items marked `achieved` / all items (abandoned items stay in the denominator) |
| verified progress | items whose skill's **declared proficiency** has actually reached the target / all items |

Marking an item achieved is a claim; reaching the proficiency is
evidence. WPM shows both rather than letting the claim stand in for
the outcome. The lifecycle is `draft → active → completed` (cancel
from either open state), and a plan cannot complete while any item is
still open.

## Talent pipelines

A **pipeline** is a named pool of people being grown toward something:
`succession`, `hiring`, `early_careers`, or `internal_mobility`. Its
members are candidates or employees, each at a stage:

`identified → assessing → developing → ready → placed`, with `exited`
reachable from any open stage — **and a step back from `ready` to
`developing`**, because readiness genuinely regresses and a pipeline
that can only move forward would overstate the bench.

Health counts the **live** pool (`placed` and `exited` members have
left it) and the `ready` count — the only number that answers "could
we fill this today?".

## Early careers — apprenticeships and internships

An **early-career programme** is an `apprenticeship`, `internship`, or
`graduate` scheme: name, level, duration in months, an optional
training-provider `organization:` URN, and the **off-the-job training
hours** a placement must accrue. An apprenticeship *must* declare that
minimum — the hours are the substance of a regulated programme, not a
nicety.

A **placement** puts one person on a programme with an optional
supervisor: `offered → active → completed`, or `withdrawn` from
either open state. Only an `active` placement accrues off-the-job
hours, and **an apprenticeship cannot be completed below its
minimum**; the refusal names the hours recorded and the hours
required. Withdrawing forces the `withdrawn` outcome, so a withdrawn
placement can never be counted as a conversion.

**Conversion rate** divides `converted` outcomes by **completed**
placements only: a placement still running has not had the chance to
convert, and counting it would understate the rate. Before anything
completes the rate is `null`, never `0%`.

## Workforce intelligence

The read-only analytical layer over everything above
(`/api/workforce-intelligence/*`):

| View | Answers |
|---|---|
| `overview` | headcount by department / status / employment type, total FTE, tenure distribution, spans of control |
| `capability` | declared skill coverage and gaps, development plans in flight against those gaps, assessment coverage per category |
| `succession` | bench strength per plan, coverage distribution, and the single-points-of-failure list |
| `pipelines` | pipeline funnel health and early-career conversion rates |

Four rules hold across all of them, because an analytics surface is
where numbers stop being checkable:

1. **Every rate carries its terms** — `{numerator, denominator, value}`,
   and `null` (never `0`) when the denominator is zero.
2. **Nothing is imputed.** A skill nobody declared is reported as
   `undeclared`; a category nobody sat is reported as not assessed.
   Neither is a zero that reads like a measurement.
3. **Every payload names its derivation**, so a proxy cannot be
   mistaken for the thing itself (coverage counts *declarations*, not
   ability).
4. **No individual's sensitive data appears** — aggregate counts only:
   no salary, no assessment score, no review content.
