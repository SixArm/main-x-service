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
- **Cognitive (IQ-style) testing** is a fifth category with standard
  index scales (verbal comprehension, working memory, processing
  speed, spatial reasoning, fluid reasoning) — per-scale readings,
  **never a composite "IQ number"**, and never permitted on a
  `selection` instrument (an IQ scale cannot ride into hiring
  unreviewed); `psychometric` batteries span it. Equality-law review
  before any selection use is a deployment duty
  ([regulatory.md](regulatory.md)) — the report-never-gate posture
  above applies with full force here.

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

## WPM-R27 — Working-time guardrails

*As a rota planner I can see advisory working-time signals so rotas
and recorded time don't quietly drift into unsafe or unlawful
patterns.*

- Derived **entirely from data WPM already holds** — time entries and
  the shift rota; no new stored state. Signals (UK Working Time
  Regulations shapes, advisory):
  - **48-hour average** — average weekly recorded minutes over the
    17-week reference window, with WPM-D16 terms; flagged when the
    average exceeds 48 h. All recorded (non-deleted) time counts, not
    just approved — a safety signal must not wait for approval.
  - **11-hour daily rest** — consecutive shift assignments (recent and
    planned) with less than 11 h between them.
- **Advisory, never blocking** ([design.md](design.md) WPM-D19): the
  view flags; it refuses nothing, and assignment/time endpoints are
  unchanged.
- Visibility equals the underlying workforce data (the rota and time
  entries): the flagged list names employees exactly as the rota does;
  no salary-grade masking applies and no new persona is introduced.

## WPM-R28 — Wellbeing pulse (anonymous)

*As an HR lead I can run a periodic anonymous pulse so the
organisation hears how people are doing without anyone's answer being
attributable.*

- A pulse survey has a name, one question, and an active window; an
  eligible employee submits one score on a closed 1–5 scale.
- **Anonymous by construction** ([design.md](design.md) WPM-D20): the
  stored response carries the survey, the employee's *department*, the
  score, and the date — **no author column exists**, and the
  submission's audit row records no actor. Because no author link is
  stored, duplicate submissions are technically possible; the results
  view therefore counts *responses*, never *respondents*, and says so.
- Results are aggregate-only with a **k-anonymity floor of 5**: a
  department cell with fewer than 5 responses is suppressed (marked,
  its statistics withheld), and the overall block is suppressed below
  the same floor. Means carry WPM-D16 terms (`null`, never `0`).
- Submitting requires the caller to be the employee (`$sub` ownership
  when enforcement is on); a survey outside its window refuses `422`.

## WPM-R29 — 360° appraisals

*As an HR lead I can run a multi-rater (360°) appraisal so an
employee's development picture comes from a full circle — manager,
peers, direct reports, and themself — not one boss.*

- An appraisal belongs to a subject employee and declares its
  **competencies** (a capped list of short labels) at creation; a
  **self** nomination for the subject is created automatically.
- **Nominations** name the raters (employees) and their **group** —
  `self | manager | peer | report` (external raters are deferred: WPM
  has no identity for them). One nomination per rater; at most 12
  raters; only the subject may be `self`.
- Lifecycle `draft → collecting → shared` (pure core): moving to
  `collecting` requires at least **3 non-self nominations**;
  nominations are frozen once collecting.
- A **response** is one rater's per-competency scores (1–5, every
  declared competency required) plus an optional comment — accepted
  only while `collecting`, only from a nominated rater, once per
  rater (`$sub` ownership when enforcement is on). Who has responded
  is visible (chasing non-responders is the point); **what** they said
  never is, per rater.
- The **report** (readable once `shared`; reads audited like review
  content) aggregates per group × competency (count + mean) and pools
  comments per group, with a **group floor of 3** for `peer` and
  `report` — below it the group's cell and comments are withheld,
  count included. `manager` and `self` disclose at n = 1 **by
  convention** (a manager's feedback is accountable; the self view is
  the subject's own) — see [design.md](design.md) WPM-D21.
- Development-facing, not pay-facing: the report is not an input to
  payroll or benchmarking, and nothing in it feeds WPM-R13/R14.
- **Rater self-service**: an employee can list their own pending
  requests (`collecting` appraisals where they are nominated and have
  not yet responded — subject, group, competencies; `$sub`-owned) and
  respond from there. The view discloses only what the rater already
  knows: that they were invited.

## WPM-R30 — Subject rights & retention

*As a data-protection officer I can answer a subject-access request,
honour an erasure request, and run the retention schedule — without
pretending WPM can do things it cannot.*

- **Subject access** — `GET /api/employees/{pid}/subject-access`
  returns everything WPM holds keyed to that employee, in one JSON
  document (`$sub`-owned / HR per policy; the export itself is
  audited). Named exclusions, stated in the payload: pulse responses
  (structurally impossible — no author link exists, WPM-D20) and
  other raters' 360° content about the subject (third-party data; the
  shared report aggregate stands in).
- **Erasure** — `POST /api/employees/{pid}/erase` (destructive-
  classified) **anonymises rather than deletes**
  ([design.md](design.md) WPM-D22): identity fields are scrubbed and
  the record soft-deleted, free text they authored is scrubbed, their
  appraisals-as-subject are closed, while payroll/financial rows
  remain (statutory retention) keyed to a pid that no longer
  identifies anyone. Refused (`422`) while the employee is not
  `terminated`/`retired` — an active employment relationship is the
  lawful basis for the data.
- **Retention** — `GET /api/retention` reports, per table, the
  soft-deleted rows older than the horizon and the expired-consent
  candidates; `POST /api/retention/sweep` (destructive-classified)
  hard-deletes those rows and scrubs expired candidates. The horizon
  is `WPM_RETENTION_DAYS` (default 365, floor 30 — a sweep that could
  run at 0 days would turn soft-delete into hard-delete); the sweep
  is audited with its counts.
- Out of code scope, still gate WPM-G2: lawful-basis mapping,
  jurisdiction-correct payroll tables, equality-law review of scoring
  ([regulatory.md](regulatory.md)).

## WPM-R31 — 360° notifications

*As a rater I find out that my feedback is wanted; as a subject I find
out my report is ready — without leaving the app.*

- **In-app notifications**, written by WPM's own lifecycle
  transitions ([design.md](design.md) WPM-D23): moving an appraisal to
  `collecting` notifies **every rater** (self included — the
  self-assessment is a task too); moving to `shared` notifies the
  subject.
- A notification carries a kind (`appraisal_request` |
  `appraisal_shared`), a neutral body, and reference data (the
  appraisal, the subject's name for a request) — **never** scores or
  comments (the WPM-D21 posture extends to notifications).
- `GET /api/employees/{pid}/notifications` (`$sub`-owned; unread
  first) and `POST /api/notifications/{pid}/read` (owner-only).
- Erasure deletes the employee's notifications; the subject-access
  export includes them.
- Out of scope, stated: outbound delivery (email/push). WPM holds no
  contact details — identities are URNs — so external channels are a
  deployment integration over the upstream person service, not a WPM
  feature.

## WPM-R32 — Ergonomic (DSE) workstation assessments

*As an HR/safety administrator I can run display-screen-equipment
assessments (UK DSE Regulations shape) so workstation problems are
recorded and fixed — without WPM becoming a health record.*

- An assessment belongs to an employee, names the **workstation**
  (e.g. "Desk 4.12", "home office"), and instantiates a checklist —
  the default DSE item set (screen, chair, keyboard/mouse, desk,
  lighting, posture/leg room, breaks, software legibility) or a
  custom list.
- Items are answered `ok` / `issue` (with an optional **equipment**
  note); completing the assessment requires every item answered
  (the WPM-D15 record-and-enforce posture) and stamps the date.
- **About the workstation, never the body**
  ([design.md](design.md) WPM-D24): no symptom, condition, or
  health field exists; notes describe equipment and environment.
- `GET /api/ergonomics/issues` reports open issues (`issue`-flagged
  items) by department with employee, workstation, item, and note —
  rota-tier visibility (WPM-R27 precedent); counts per department.
- Erasure soft-deletes the employee's assessments and scrubs item
  notes; the subject-access export includes them; both tables join
  the retention sweep.

## WPM-R17 — Family fixtures

- OpenAPI + Swagger, `Accepts-version` negotiation, `/metrics.prom`,
  OTLP tracing, health routes, Podman build, `#![forbid(unsafe_code)]`,
  clippy-pedantic, input caps → `422`, unknown-pid → `404`.
