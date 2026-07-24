# Requirements

Numbered requirements with user stories and acceptance criteria.
IDs are stable; design decisions ([design.md](design.md)) and tasks
([tasks.md](tasks.md)) trace to them. The five-pillar map:
HCM-R1–R3 talent acquisition & onboarding, HCM-R4–R6 workforce
management, HCM-R7–R9 HR service delivery, HCM-R10–R12 talent
management & development, HCM-R13–R14 payroll & compensation,
HCM-R15–R17 cross-cutting.

## HCM-R1 — Requisitions (ATS pipeline root)

*As a hiring manager I can open a funded requisition so a vacancy is
tracked from approval to fill.*

- CRUD (soft-delete) with organization/department, job title,
  headcount, salary band (minor units), pipeline per
  [talent-acquisition.md](talent-acquisition.md).
- Lifecycle `draft → open → interviewing → offer → filled | cancelled`
  enforced by the pure core; illegal transitions `422` naming the
  current state; `filled` requires hired applications = headcount.

## HCM-R2 — Candidates & applications

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

## HCM-R3 — Onboarding checklists

*As an HR administrator I can digitize onboarding so nothing statutory
is missed before day one.*

- Template-instantiated OnboardingItem checklists per new hire
  (contract, right-to-work, references, equipment, training, …).
- Employee activation (`onboarding → active`) requires all mandatory
  items complete or explicitly waived with a recorded reason.

## HCM-R4 — Time & attendance

*As an employee I can record my worked time; as a manager I can see
and approve my team's.*

- TimeEntry create/list per employee/day with kind (regular,
  overtime, on-call); >24h/day refused `422`; overtime derived
  against contracted hours (FTE-scaled) in the pure core.
- Manager approval flips entries to `approved`; only approved time
  feeds payroll (HCM-R13).

## HCM-R5 — Absence & leave

*As an employee I can request leave against my balance; as a manager
I can approve or reject it.*

- LeaveEntitlement per employee/kind/year; LeaveRequest lifecycle
  `requested → approved | rejected | cancelled`.
- Annual leave over remaining balance refused `422`; sick leave may
  go negative but is flagged; approval decrements the balance in the
  same transaction; the two-approver race is serialized (one wins).

## HCM-R6 — Shift scheduling

*As a rota planner I can build shift plans and assign employees
without conflicts.*

- Shift + ShiftAssignment CRUD; double-booking (overlapping
  assignment) refused `422`; assignment overlapping approved leave
  refused `422`; a day's rota view per department.

## HCM-R7 — Employee records (core HR database)

*As an HR administrator I maintain the single source of employment
truth.*

- Employee CRUD with `person:`/`worker:`/`organization:` URNs
  (shape-validated; display names resolved best-effort), employee
  number (unique per organization), status lifecycle
  `onboarding → active ⇄ on_leave → offboarding → terminated | retired`,
  employment type, FTE percent, department, job title, manager,
  salary (minor units + ISO-4217, sensitive).
- Org chart derived from `manager_pid`; a cycle is refused `422`.

## HCM-R8 — Self-service

*As an employee I can see my own record, payslips, leave, and reviews
without asking HR.*

- Ownership rules (`resource.person = $sub`) let an authenticated
  employee read their own record (salary visible to self), payslips,
  balances, and shared reviews, and submit leave/time — with the
  blanket guard on.

## HCM-R9 — Benefits administration

*As an HR administrator I manage benefit plans; as an employee I
enrol.*

- BenefitPlan CRUD (kind, provider, employee/employer cost in minor
  units); BenefitEnrollment per employee with eligibility window;
  double-enrolment in the same plan refused `422`.

## HCM-R10 — Performance reviews

*As a manager I run review cycles with goals and feedback so
performance is tracked fairly.*

- ReviewCycle → Review per employee with lifecycle
  `draft → submitted → calibrated → shared`; Goals with status +
  weighting; FeedbackEntry rows.
- Review content is high-sensitivity: shared reviews readable by the
  subject; drafts only by the author + HR; reads audited.

## HCM-R11 — Learning (LMS over course-service)

*As an employee I enrol in training; as HR I see compliance-training
gaps.*

- TrainingEnrollment against `course:`/`courseinstance:` URNs with
  status (enrolled, in_progress, completed, failed) and certificate
  expiry; an expiring-certificates report (`?within_days=`).

## HCM-R12 — Succession planning

*As an HR director I keep succession plans for critical roles.*

- SuccessionPlan per role (criticality 1–5) with ranked
  SuccessionCandidates (readiness `ready_now | ready_1y | ready_2y`);
  the gap report lists criticality ≥ 4 roles with no `ready_now`
  candidate; plans are high-sensitivity (reads audited).

## HCM-R13 — Payroll runs & payslips

*As a payroll administrator I calculate and approve a pay period so
every active employee gets a correct payslip.*

- PayrollRun per organization/period, lifecycle
  `draft → calculated → approved → paid`; calculation generates one
  Payslip per in-scope employee from salary (FTE-pro-rated), approved
  overtime, and benefit deductions — pure-core arithmetic, stub tax
  tables, minor units, overflow refused.
- Payslip invariant `net = gross − Σ deductions` enforced before
  persist; approved runs immutable; re-calculation only in `draft`.

## HCM-R14 — Salary benchmarking

*As a compensation analyst I compare salaries to recorded market
data.*

- Benchmark rows per job title (min/median/max, currency, source,
  as-of); the comparison view flags employees `below_min` /
  `above_max`; benchmark-vs-salary output is payroll/HR-persona data.

## HCM-R15 — AuthN/Z & masking

*As a deployment operator I can activate authentication and express
persona policies without code changes.*

- Family stack: offline PASETO verify, blanket `HCM_REQUIRE_AUTH`
  guard (default off), shared ABAC engine; record-level attrs
  (`resource.person`, `resource.department`, `resource.status`);
  `mask` obligation redacts salary/payslip amounts/review content;
  the four personas of [auth.md](auth.md) expressible as policy.

## HCM-R16 — Audit & events

*As a compliance officer I can reconstruct who did and saw what.*

- Every mutation audited + evented (family envelope,
  `HCM_EVENT_TRANSPORT` memory/outbox); sensitive reads (salary,
  payslips, review content, succession) audited; approvals/waivers
  carry reasons.

## HCM-R17 — Family fixtures

- OpenAPI + Swagger, `Accepts-version` negotiation, `/metrics.prom`,
  OTLP tracing, health routes, Podman build, `#![forbid(unsafe_code)]`,
  clippy-pedantic, input caps → `422`, unknown-pid → `404`.
