# Domain model

All identities are **EntityRef URNs**; HCM's own records use public
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
SuccessionPlan 1──* SuccessionCandidate (employee pids)
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
  `incumbent_pid?`, criticality (1–5); **SuccessionCandidate** —
  plan × employee, `readiness` (`ready_now` \| `ready_1y` \|
  `ready_2y`), `development_note?`.

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
record with salary, payslips, review content) are audited too.
Events follow the family envelope with kinds such as
`employee_hired`, `leave_approved`, `payroll_run_calculated`,
`review_shared` — see [audit.md](audit.md).
