# Requirements

Numbered requirements with user stories and acceptance criteria.
IDs are stable; design decisions ([design.md](design.md)) and tasks
([tasks.md](tasks.md)) trace to them. The five-pillar map:
WPM-R1–R3 talent acquisition & onboarding, WPM-R4–R6 workforce
management, WPM-R7–R9 HR service delivery, WPM-R10–R12 talent
management & development, WPM-R13–R14 payroll & compensation,
WPM-R15–R17 cross-cutting.

## WPM-R1 — Requisitions (ATS pipeline root)

*As a hiring manager I can open a funded requisition so a vacancy is
tracked from approval to fill.*

- CRUD (soft-delete) with organization/department, job title,
  headcount, salary band (minor units), pipeline per
  [talent-acquisition.md](talent-acquisition.md).
- Lifecycle `draft → open → interviewing → offer → filled | cancelled`
  enforced by the pure core; illegal transitions `422` naming the
  current state; `filled` requires hired applications = headcount.

## WPM-R2 — Candidates & applications

*As a recruiter I can track candidates through screening, interviews,
and offer so nothing falls through.*

- Candidate records with consent expiry (`consent_until`); expired
  candidates excluded from pool search and listed for purge.
- Applications per requisition with stage machine
  `received → screened → interviewing → offer → hired | rejected |
  withdrawn`; Interview rows (schedule, interviewer `worker:` URNs,
  outcome); stage changes audited + evented.
- Hiring an application creates the Employee in `onboarding` status
  (one transaction).

## WPM-R3 — Onboarding checklists

*As an HR administrator I can digitize onboarding so nothing statutory
is missed before day one.*

- Template-instantiated OnboardingItem checklists per new hire
  (contract, right-to-work, references, equipment, training, …).
- Employee activation (`onboarding → active`) requires all mandatory
  items complete or explicitly waived with a recorded reason.

## WPM-R4 — Time & attendance

*As an employee I can record my worked time; as a manager I can see
and approve my team's.*

- TimeEntry create/list per employee/day with kind (regular,
  overtime, on-call); >24h/day refused `422`; overtime derived
  against contracted hours (FTE-scaled) in the pure core.
- Manager approval flips entries to `approved`; only approved time
  feeds payroll (WPM-R13).

## WPM-R5 — Absence & leave

*As an employee I can request leave against my balance; as a manager
I can approve or reject it.*

- LeaveEntitlement per employee/kind/year; LeaveRequest lifecycle
  `requested → approved | rejected | cancelled`.
- Annual leave over remaining balance refused `422`; sick leave may
  go negative but is flagged; approval decrements the balance in the
  same transaction; the two-approver race is serialized (one wins).

## WPM-R6 — Shift scheduling

*As a rota planner I can build shift plans and assign employees
without conflicts.*

- Shift + ShiftAssignment CRUD; double-booking (overlapping
  assignment) refused `422`; assignment overlapping approved leave
  refused `422`; a day's rota view per department.

## WPM-R7 — Employee records (core HR database)

*As an HR administrator I maintain the single source of employment
truth.*

- Employee CRUD with `person:`/`worker:`/`organization:` URNs
  (shape-validated; display names resolved best-effort), employee
  number (unique per organization), status lifecycle
  `onboarding → active ⇄ on_leave → offboarding → terminated | retired`,
  employment type, FTE percent, department, job title, manager,
  salary (minor units + ISO-4217, sensitive).
- Org chart derived from `manager_pid`; a cycle is refused `422`.

## WPM-R8 — Self-service

*As an employee I can see my own record, payslips, leave, and reviews
without asking HR.*

- Ownership rules (`resource.person = $sub`) let an authenticated
  employee read their own record (salary visible to self), payslips,
  balances, and shared reviews, and submit leave/time — with the
  blanket guard on.

## WPM-R9 — Benefits administration

*As an HR administrator I manage benefit plans; as an employee I
enrol.*

- BenefitPlan CRUD (kind, provider, employee/employer cost in minor
  units); BenefitEnrollment per employee with eligibility window;
  double-enrolment in the same plan refused `422`.

## WPM-R10 — Performance reviews

*As a manager I run review cycles with goals and feedback so
performance is tracked fairly.*

- ReviewCycle → Review per employee with lifecycle
  `draft → submitted → calibrated → shared`; Goals with status +
  weighting; FeedbackEntry rows.
- Review content is high-sensitivity: shared reviews readable by the
  subject; drafts only by the author + HR; reads audited.

## WPM-R11 — Learning (LMS over course-service)

*As an employee I enrol in training; as HR I see compliance-training
gaps.*

- TrainingEnrollment against `course:`/`courseinstance:` URNs with
  status (enrolled, in_progress, completed, failed) and certificate
  expiry; an expiring-certificates report (`?within_days=`).

## WPM-R12 — Succession planning

*As an HR director I keep succession plans for critical roles.*

- SuccessionPlan per role (criticality 1–5) with ranked
  SuccessionCandidates (readiness `ready_now | ready_1y | ready_2y`);
  the gap report lists criticality ≥ 4 roles with no `ready_now`
  candidate; plans are high-sensitivity (reads audited).

## WPM-R13 — Payroll runs & payslips

*As a payroll administrator I calculate and approve a pay period so
every active employee gets a correct payslip.*

- PayrollRun per organization/period, lifecycle
  `draft → calculated → approved → paid`; calculation generates one
  Payslip per in-scope employee from salary (FTE-pro-rated), approved
  overtime, and benefit deductions — pure-core arithmetic, stub tax
  tables, minor units, overflow refused.
- Payslip invariant `net = gross − Σ deductions` enforced before
  persist; approved runs immutable; re-calculation only in `draft`.

## WPM-R14 — Salary benchmarking

*As a compensation analyst I compare salaries to recorded market
data.*

- Benchmark rows per job title (min/median/max, currency, source,
  as-of); the comparison view flags employees `below_min` /
  `above_max`; benchmark-vs-salary output is payroll/HR-persona data.

## WPM-R15 — AuthN/Z & masking

*As a deployment operator I can activate authentication and express
persona policies without code changes.*

- Family stack: offline PASETO verify, blanket `WPM_REQUIRE_AUTH`
  guard (default off), shared ABAC engine; record-level attrs
  (`resource.person`, `resource.department`, `resource.status`);
  `mask` obligation redacts salary/payslip amounts/review content;
  the four personas of [auth.md](auth.md) expressible as policy.

## WPM-R16 — Audit & events

*As a compliance officer I can reconstruct who did and saw what.*

- Every mutation audited + evented (family envelope,
  `WPM_EVENT_TRANSPORT` memory/outbox); sensitive reads (salary,
  payslips, review content, succession) audited; approvals/waivers
  carry reasons.

## WPM-R20 — Assessments (aptitude / personality / psychometric / selection)

*As a recruiter or an HR lead I can record the tests a candidate or
employee has sat, and read them back as a profile, without the
results turning into an automated verdict.*

- An instrument catalog: name, category, the scales it reports, its
  duration and validity. A scale outside the category is refused —
  except `psychometric`, which spans aptitude **and** personality by
  definition.
- Sittings against a candidate or an employee, optionally tied to an
  application (a mismatched application/candidate pair is refused).
  Lifecycle `scheduled → in_progress → completed → expired` (+
  `cancelled`); completing requires ≥ 1 result and derives the expiry
  from the instrument's validity.
- Per-scale results: whole-number raw / max / percentile (0–100) and
  a band derived from the percentile.
- Derived per-subject profile: the current reading per scale (most
  recent completed, unexpired sitting), the scales **not** assessed,
  and the selection-suitability mean — real scores only, `null` when
  nothing was measured.
- **Sensitive**: `mask` obligation honoured on **every** read path
  (bands survive, scores do not); unmasked reads of scored results
  audited; aggregate analytics report distributions only.
- WPM reports; it does not rank, recommend, or gate a stage on a
  score.

## WPM-R21 — Upskilling & reskilling plans

*As a manager I can plan a person's development and see whether it
actually happened.*

- `upskill` (deepen the current role, no target role) vs `reskill`
  (build toward a named different role) — the distinction enforced,
  not conventional.
- Items pair a catalog skill with a `current_level → target_level`
  step (1–5, strictly increasing), a method, and a due date.
- **Two** progress figures — declared (items marked achieved) and
  verified (declared proficiency actually reaching the target) — so a
  claim cannot silently stand in for the outcome.
- Lifecycle `draft → active → completed`; a plan cannot complete
  while an item is open.

## WPM-R22 — Talent pipelines

*As an HR lead I can maintain pools of people being grown toward
succession, hiring, early careers, or internal mobility.*

- Members are candidates or employees; stages
  `identified → assessing → developing → ready → placed` with `exited`
  from any open stage.
- Readiness may **regress** (`ready → developing`): a pipeline that
  only moves forward would overstate the bench.
- Health reports the live pool (excluding placed/exited) and the
  ready count.

## WPM-R23 — Apprenticeships & internships

*As an early-careers lead I can run apprenticeships, internships, and
graduate schemes with their obligations recorded rather than assumed.*

- Programmes declare kind, level, duration, provider, and the
  off-the-job training hours a placement must accrue — **required**
  for an apprenticeship.
- Placements: `offered → active → completed` (or `withdrawn`); only an
  active placement accrues hours; **an apprenticeship cannot be
  completed below its minimum**, and the refusal names both numbers.
- Conversion rate divides converted outcomes by **completed**
  placements only; `null` before anything completes.

## WPM-R24 — Workforce intelligence

*As an HR director I can see the shape and capability of the
workforce, and where it is exposed.*

- `overview` (headcount, FTE, tenure, spans of control), `capability`
  (declared skill coverage + gaps, plans in flight, assessment
  coverage), `succession` (bench strength + single points of failure),
  `pipelines` (funnel + early-career conversion).
- Every rate carries `{numerator, denominator, value}` and is `null`
  when the denominator is zero; nothing is imputed; every payload
  names its derivation; no individual's sensitive data appears.

## WPM-R25 — Health entitlement prompts (wellbeing)

*As an HR benefits administrator I can configure public-health
entitlements (e.g. NHS vaccination cohorts) so eligible employees
are prompted through self-service and don't miss free preventative
care.*

- Entitlement rules are configuration, not code (cohorts change year
  to year): each rule declares a name, a plain-language description,
  an information URL, an active window, and eligibility predicates
  over **non-clinical** facts only — age band (date of birth resolved
  via the upstream person client seam), role, department. Rules keyed
  on health status (e.g. an immunosuppressed cohort) are refused by
  design ([design.md](design.md) WPM-D17).
- An eligible employee sees an informational prompt in self-service
  and records one acknowledgement:
  `booked | done | declined | dismissed`. No re-prompt after an
  acknowledgement, except one optional reminder for multi-dose
  courses.
- Privacy: acknowledgements are readable by the employee (`$sub`
  ownership) and by HR as **aggregate uptake counts only** (WPM-D16
  terms); never visible to managers; declining has no recorded
  consequence and nothing feeds reviews or payroll.
- WPM prompts and records the acknowledgement; it does not book
  appointments and stores no vaccination status or clinical record.

## WPM-R26 — Benefits awareness

*As an HR benefits administrator I can signpost any entitlement or
benefit — not just health cohorts — so employees discover what they
already have.*

- The WPM-R25 entitlement rules generalise with a closed **`kind`**
  (`health | benefit`). The predicate vocabulary is unchanged — still
  non-clinical facts only (WPM-D17) — and the acknowledgement
  vocabulary is unchanged.
- A `benefit`-kind rule MAY reference an existing benefit plan
  (`benefit_plan_pid`, validated live); the prompt then carries the
  plan reference so self-service can link to enrolment. A `health`
  rule may not reference a plan (kept crisp).
- A plan-linked rule goes **quiet automatically** for an employee with
  a live enrolment in that plan — derived per request from
  `benefit_enrollments`, never stored ([design.md](design.md)
  WPM-D18). WPM signposts; enrolment remains the WPM-R9 act.
- The rule list filters by `?kind=`; the uptake view carries the kind.
- For a plan-linked rule the uptake view also reports **enrolment
  conversion** — of the distinct employees who acknowledged the
  prompt, how many now hold a live enrolment in the linked plan —
  aggregate counts only with WPM-D16 terms (`null`, never `0`, when
  nobody has acknowledged), derived per request, never stored.

## WPM-R17 — Family fixtures

- OpenAPI + Swagger, `Accepts-version` negotiation, `/metrics.prom`,
  OTLP tracing, health routes, Podman build, `#![forbid(unsafe_code)]`,
  clippy-pedantic, input caps → `422`, unknown-pid → `404`.
