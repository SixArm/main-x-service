# Domain model

All identities are **EntityRef URNs**; WPM's own records use public
UUID `pid`s; every table carries `created_at` / `updated_at` and (where
meaningful) soft-delete `deleted_at`. Money is integer **minor units**
+ ISO-4217 currency (family posture; no floats).

```
Requisition 1──* Application *──1 Candidate
Application (hired) ──▶ Employee 1──* OnboardingItem
Employee *──1 person-ref (+ worker-ref?, organization-ref)
Employee 1──* TimeEntry · LeaveRequest · ShiftAssignment
Employee 1──* BenefitEnrollment *──1 BenefitPlan
Employee 1──* Review · Goal · FeedbackEntry · TrainingEnrollment
Employee 1──* EmployeeSkill *──1 Skill · PathEnrollment *──1 LearningPath 1──* Step
Mentorship (mentor × mentee) 1──* MentorshipSession
Assessment (subject: candidate|employee) 1──* AssessmentResult; *──1 Instrument
DevelopmentPlan 1──* Item · TalentPipeline 1──* Member · Program 1──* Placement
SuccessionPlan 1──* SuccessionCandidate (employee pids)
WellbeingEntitlement 1──* EntitlementAcknowledgement (employee × rule)
PulseSurvey 1──* PulseResponse (NO author column — WPM-D20)
Appraisal 1──* AppraisalNomination 1──0..1 AppraisalResponse (WPM-D21)
Employee 1──* Notification · ErgonomicAssessment 1──* Item · AdjustmentRequest
PayrollRun 1──* Payslip (one per employee per run)
Benchmark (job_title × currency)
```

## Employee (the employment record)

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `person_ref` | EntityRef | `person:<pid>` — the human; never raw demographics |
| `worker_ref` | EntityRef? | `worker:<pid>` professional identity, when registered |
| `organization_ref` | EntityRef | the employer |
| `employee_number` | text | employer-scoped, unique |
| `display_name` | text | denormalised cache; refreshable; maskable |
| `status` | enum | `onboarding` \| `active` \| `on_leave` \| `offboarding` \| `terminated` \| `retired` |
| `employment_type` | enum | `full_time` \| `part_time` \| `contract` \| `intern` |
| `fte_percent` | int | 1–100 |
| `department` | text | ABAC scoping attribute (`resource.department`) |
| `job_title` | text | benchmarking key |
| `manager_pid` | UUID? | another Employee — the org-chart edge |
| `hire_date` | date | |
| `termination_date` / `termination_reason` | date? / enum? | `resignation` \| `dismissal` \| `redundancy` \| `retirement` \| `end_of_contract` |
| `salary_minor` / `currency` | i64 / text | **sensitive**: masked unless HR/payroll/self |
| `location` | text? | scheduling + benchmarking dimension |

## Talent acquisition

- **Requisition** — `title`, `department`, `location?`, `headcount`,
  `hiring_manager_pid?`, salary band (`band_min_minor`/`band_max_minor`
  + currency), `status`: `draft → open → interviewing → offer →
  filled | cancelled`.
- **Candidate** — pool entry: `name`, `email?`, `person_ref?` (when
  matched to a person record), `summary?`, `tags[]`, `source`
  (`applied` \| `referral` \| `sourced`), `consent_until` (date —
  pool retention is consent-bounded).
- **Application** — `requisition_pid`, `candidate_pid`, `stage`:
  `received → screened → interviewing → offer → hired | rejected |
  withdrawn`; `Interview` rows (`scheduled_at`, `kind`
  (phone/technical/panel), `interviewer_ref?`, `outcome?`).
  **Hiring** an offer-stage application mints the Employee
  (`status = onboarding`) and fills requisition headcount.
- **OnboardingItem** — per new employee: `kind` (`contract` \|
  `background_check` \| `right_to_work` \| `tax_form` \| `equipment`
  \| `training` \| `other`), `status` (`pending → completed |
  waived`), `due_date?`. All items closed ⇒ employee may activate.

## Workforce management

- **TimeEntry** — `employee_pid`, `date`, `minutes` (or clock
  in/out pair), `kind` (`regular` \| `overtime` \| `remote` \|
  `on_call`), `note?`. Overtime = minutes beyond the contracted
  day, derived in the pure core.
- **LeaveEntitlement** — `employee_pid`, `kind`, `year`,
  `minutes_total`; balance = total − approved-taken.
- **LeaveRequest** — `employee_pid`, `kind` (`annual` \| `sick` \|
  `parental` \| `unpaid` \| `compassionate`), `from`/`to`,
  `status`: `requested → approved | rejected → cancelled`;
  approving decrements the balance (sick may go negative —
  documented rule).
- **Shift** — `date`, `start`/`end`, `department`, `location?`,
  `required_headcount`; **ShiftAssignment** — shift × employee;
  the pure core refuses double-booking and leave conflicts.

## HR core

- **BenefitPlan** — `name`, `kind` (`health` \| `pension` \| `perk`),
  `provider?`, `employee_cost_minor`/`employer_cost_minor` + currency.
- **BenefitEnrollment** — employee × plan, `effective_from`,
  `effective_to?`, `status` (`enrolled` \| `ended`).
- **Org chart** — derived: the `manager_pid` tree (cycle-refused at
  write, patient-flow dependency style).

## Talent development

- **ReviewCycle** — `name`, `period_start`/`period_end`, `status`
  (`open` \| `calibrating` \| `closed`).
- **Review** — cycle × employee, `status`
  (`draft → submitted → calibrated → shared`), `rating` (1–5),
  `summary`; **Goal** rows (`title`, `due?`, `status`); 
  **FeedbackEntry** rows (`author_ref?`, `text`, continuous —
  attachable outside cycles too).
- **TrainingEnrollment** — employee × `course_ref`
  (`course:`/`courseinstance:` URN), `status`
  (`enrolled → completed | failed | withdrawn`), `completed_on?`,
  `certificate_expires_on?` (expiring certifications drive the
  compliance dashboard).
- **SuccessionPlan** — `position_title`, `department`,
  `incumbent_pid?`, criticality (1–5), `risk_of_loss?` (`low` \|
  `medium` \| `high`), `vacancy_expected_on?`;
  **SuccessionCandidate** — plan × employee, `readiness`
  (`ready_now` \| `ready_1y` \| `ready_2y`), `development_note?`.
  Both the plan's judgements and a candidate's readiness are
  updatable in place; readiness may move **down**.

## Assessments

- **AssessmentInstrument** — the catalog entry for a named test:
  `name`, `category` (`aptitude` \| `personality` \| `psychometric`
  \| `selection` \| `cognitive`), `provider?`, `scales[]` (each must
  suit the category — `cognitive` carries the IQ-style index scales
  verbal_comprehension / working_memory / processing_speed /
  spatial_reasoning / fluid_reasoning; **no composite score exists**,
  and `selection` instruments refuse cognitive scales — WPM-R20),
  `duration_minutes?`, `validity_months?` (drives a completed
  sitting's expiry).
- **Assessment** — one administration: instrument × subject
  (`candidate` \| `employee` + pid) × optional `application_pid`,
  `status` (`scheduled → in_progress → completed → expired`, or
  `cancelled`), `scheduled_on?` / `completed_on?` / `expires_on?`,
  `administered_by?`, `notes?`.
- **AssessmentResult** — assessment × `scale` (one row per scale):
  `raw_score?` / `max_score?` (whole points), `percentile?` (0–100),
  `band?` (`low` \| `below_average` \| `average` \| `above_average`
  \| `high`, derived from the percentile unless given),
  `narrative?`. Sensitive: masked under the ABAC `mask` obligation,
  unmasked reads audited.

## Talent strategy

- **DevelopmentPlan** — employee × `kind` (`upskill` \| `reskill`),
  `target_job_title?` / `target_department?` (**required** for a
  reskill, **refused** for an upskill), `rationale?`, `status`
  (`draft → active → completed`, or `cancelled`), `started_on?`,
  `target_on?`; **DevelopmentPlanItem** — plan × skill,
  `current_level` → `target_level` (1–5, target strictly higher),
  `method` (`course` \| `mentorship` \| `on_the_job` \|
  `apprenticeship` \| `internship` \| `self_study`), `course_ref?`,
  `due_on?`, `status` (`planned` \| `in_progress` \| `achieved` \|
  `abandoned`).
- **TalentPipeline** — `name`, `purpose` (`succession` \| `hiring`
  \| `early_careers` \| `internal_mobility`), target role/department;
  **PipelineMember** — pipeline × subject (`candidate` \|
  `employee`), `stage` (`identified → assessing → developing → ready
  → placed`, `exited` from any open stage, and `ready → developing`
  as a legitimate regression), `readiness?`, `added_on`.
- **EarlyCareerProgram** — `name`, `kind` (`apprenticeship` \|
  `internship` \| `graduate`), `level?`, `duration_months`,
  `min_off_the_job_hours?` (**required** for an apprenticeship),
  `provider_ref?` (an `organization:` URN); **ProgramPlacement** —
  programme × employee, `supervisor_pid?`, `started_on`, `ends_on?`,
  `status` (`offered → active → completed`, or `withdrawn`),
  `off_the_job_hours` (accrued; only an active placement accrues),
  `outcome` (`pending` \| `converted` \| `not_converted` \|
  `withdrawn`).

## Learning & mentorship

- **Skill** — catalog entry (`name`, `category`); **EmployeeSkill** —
  declared proficiency 1–5 + optional target, one row per
  employee × skill (upserted).
- **LearningPath** — ordered `course_ref` steps; **PathEnrollment** —
  employee × path; progress counts only **completed**
  `TrainingEnrollment` rows per step (honest derivation).
- **Mentorship** — mentor × mentee (`proposed → active → completed`,
  `ended` from open states); **MentorshipSession** — dated notes on
  an active pairing.

## Wellbeing (WPM-R25/R26 — WPM-D17/D18)

- **WellbeingEntitlement** — one configurable prompt rule: `kind`
  (`health` \| `benefit`), name/description/info URL, **non-clinical
  predicates only** (age band via the upstream person birth date —
  unknown age fails a banded rule — departments, job titles), `doses`,
  active window, optional `benefit_plan_pid` (benefit-kind only; a
  plan-linked prompt goes quiet once the employee is live-enrolled —
  derived, never stored).
- **EntitlementAcknowledgement** — employee × rule, `response`
  (`booked` \| `done` \| `declined` \| `dismissed`), one optional
  reminder for multi-dose courses (`reminded_on`). A workflow fact,
  never a vaccination status.
- **PulseSurvey** / **PulseResponse** — anonymous by construction
  (WPM-D20): the response row is survey + department + score (1–5) +
  date, **no author column**; results are k-floored (k = 5, counts
  withheld below it).

## 360° appraisals (WPM-R29 — WPM-D21)

- **Appraisal** — subject employee × declared `competencies[]`,
  lifecycle `draft → collecting → shared` (one-way; nominations
  freeze at collecting, responses close at shared).
- **AppraisalNomination** — rater × group (`self` \| `manager` \|
  `peer` \| `report`); self automatic; ≤ 12 raters, ≥ 3 non-self to
  collect.
- **AppraisalResponse** — links to its nomination (procedural
  anonymity: once-per-rater needs the link; **no endpoint serves
  rater-level content**); per-competency scores 1–5 + optional
  comment. The shared-only report aggregates group × competency with
  a floor of 3 on `peer`/`report` cells.

## Notifications, ergonomics, adjustments

- **Notification** (WPM-R31/D23) — employee × kind
  (`appraisal_request` \| `appraisal_shared` \| `adjustment_update`),
  neutral body + reference data (never scores/comments/words),
  `read_at`. In-app only; WPM holds no contact details.
- **ErgonomicAssessment** / **ErgonomicItem** (WPM-R32/D24) — a DSE
  workstation checklist (default 8 items; **no symptom field**);
  items answered `ok`/`issue` + equipment note; completion requires
  every answer; issue-flagged items feed the rota-tier department
  report.
- **AdjustmentRequest** (WPM-R33/D25) — `category` (practical closed
  set), **barrier / impact / adjustment** (all required, the
  requester's words), lifecycle `requested → agreed \| declined \|
  withdrawn`, `agreed → in_place \| withdrawn`; decision note + date.
  **No diagnosis / condition / medical-evidence column exists**;
  masked reads withhold the words; no aggregate reporting surface.

## Payroll & compensation

- **PayrollRun** — `organization_ref`, `period_start`/`period_end`,
  `status`: `draft → calculated → approved → paid` (each step
  audited; recalculation only from `draft`/`calculated`).
- **Payslip** — run × employee: `gross_minor`, deduction lines
  (`kind` (`tax` \| `social` \| `pension` \| `other`),
  `amount_minor`), `net_minor`, `currency`. Net = gross − Σ
  deductions, enforced in the pure core (overflow-refusing).
- **Benchmark** — `job_title`, `location?`, `currency`,
  `market_min_minor` / `market_median_minor` / `market_max_minor`,
  `as_of`; comparison flags employees paid below min / above max.

## Audit & events

Every mutation writes an audit row; **sensitive reads** (employee
record with salary, payslips, review content, 360 reports, adjustment
words, unmasked assessment scores, succession) are audited too — with
one designed exception: a **pulse submission's audit row carries no
actor** (WPM-D20). Events follow the family envelope with kinds such
as `employee_hired`, `leave_approved`, `payroll_run_calculated`,
`review_shared` — see [audit.md](audit.md).

## Subject rights & retention (WPM-R30 — WPM-D22)

Not record types but the lifecycle over all of them: the
**subject-access export** gathers every table keyed to one employee
(exclusions named in the payload); **erasure anonymises** (identity
fields scrubbed to a tombstone `person:` URN, authored free text
scrubbed, row soft-deleted; payroll rows remain under statutory
retention); the **retention sweep** hard-deletes soft-deleted rows
past the floored horizon (`WPM_RETENTION_DAYS`, default 365, floor
30) across the pinned 41-table list and scrubs expired-consent
candidates.
