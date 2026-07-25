# Purpose

## Problem

Workforce administration is scattered: applicants tracked in inboxes,
onboarding on paper, leave in spreadsheets, schedules on whiteboards,
reviews in documents nobody reopens, training records unverifiable,
succession in someone's head, and payroll reconciled by hand against
all of the above. Every handoff between those silos loses data,
delays people, and creates compliance risk — employment records are
regulated personal data with statutory retention duties.

## What WPM does

One operational system for the **employment lifecycle**, hire to
retire, organized as five pillars:

### 1. Talent acquisition & onboarding

- **Applicant tracking (ATS)** — job requisitions with a headcount
  and a hiring pipeline; applications advance through screening,
  interviews, and offer stages; interview scheduling recorded per
  application.
- **Candidate pool** — profiles of past applicants retained (with
  consent) for future openings; a new requisition can search the
  pool before advertising.
- **Onboarding** — a per-hire digital checklist (contract signature,
  background check, right-to-work, tax forms) tracked to completion
  **before day one**; hiring converts an accepted offer into an
  employee record.

### 2. Workforce management

- **Time & attendance** — clock-in/out and remote-hours entries,
  overtime derivation against contracted hours.
- **Absence management** — leave requests (annual, sick, parental,
  unpaid) with entitlements, accrual balances, and approval flow.
- **Scheduling** — shift plans per team/location, assignment against
  availability, and conflict/limit checks.
- **Working-time guardrails** — advisory 48-hour-average and 11-hour
  rest-gap flags (UK WTR shape) derived from recorded time and the
  rota, planned shifts included; flags, never blocks.

### 3. HR service delivery & admin

- **Core HR database** — the employee record as the single source of
  employment truth (identity stays in the person/worker services);
  the manager chain yields the **org chart**.
- **Self-service** — employees view their own record, request leave,
  see payslips; managers see their team; HR sees their remit —
  enforced as ABAC policy, not code.
- **Benefits administration** — plans (health, pension, perks) and
  per-employee enrollments with effective dates.
- **Wellbeing & benefits awareness** — configurable, non-clinical
  entitlement prompts (NHS-style vaccination cohorts; plan-linked
  benefit signposting that goes quiet on enrolment), aggregate-only
  uptake and conversion, and the anonymous k-floored pulse.
- **Ergonomics & adjustments** — DSE workstation assessments
  (equipment facts, never symptoms) and barrier-based reasonable
  adjustments (no diagnosis required — or storable).
- **Notifications & subject rights** — in-app, reference-only
  notifications; subject-access export, erasure as anonymisation,
  and the retention sweep.

### 4. Talent management & development

- **Performance reviews** — cycles, per-employee goals, continuous
  feedback entries, and appraisals with calibrated ratings.
- **360° appraisals** — multi-rater feedback (manager, peers,
  reports, self) per declared competency, with group-floored
  reports, rater self-service, and in-app notifications;
  development-facing, never a pay input.
- **Learning (LMS)** — training enrollments referencing the family's
  [course-service](../../course/course-service-with-loco/) courses
  and instances (compliance courses, certifications with expiry).
- **Assessments** — aptitude, personality, psychometric, selection,
  and cognitive (IQ-style index) tests with per-scale results, score
  bands, validity, and a derived profile. Reported, never
  prescriptive: WPM does not rank or recommend, no composite IQ
  exists, and cognitive scales are refused on selection instruments.
- **Succession planning** — key positions, incumbents with a risk of
  loss, a readiness-rated bench, and the single points of failure that
  fall out of criticality × risk.
- **Upskilling and reskilling plans** — per-employee skill steps
  toward the current role (upskill) or a different one (reskill),
  reporting claimed *and* verified progress.
- **Talent pipelines** — pools grown toward succession, hiring, early
  careers, or internal mobility, where readiness can regress as well
  as improve.
- **Apprenticeships and internships** — early-career programmes and
  placements, with off-the-job training hours enforced as a completion
  precondition, and honest conversion rates.
- **Workforce intelligence** — headcount and shape, capability and
  gaps, bench strength, and pipeline funnel; every rate carrying its
  numerator and denominator.

### 5. Payroll & compensation

- **Payroll runs** — per-period calculation of gross pay, deductions,
  and net pay into payslips (integer minor units, ISO-4217 — the
  family money posture; no floats).
- **Salary benchmarking** — internal pay per role compared against
  recorded market benchmarks to flag under/over-market positions.

## Goals (made testable)

| Goal | Mechanism |
|---|---|
| One source of employment truth | the Employee record + referenced identities; everything else keys off it |
| Hire-to-retire lifecycle | requisition → application → offer → onboarding → active → leave/shift/review/training → offboarding → terminated/retired, as explicit state machines |
| Strategic (not just admin) HR | succession pipelines, benchmarking, review calibration, training compliance surfaced on dashboards |
| Self-service without over-exposure | ABAC personas (employee/manager/HR/payroll) + masking on salary and payslip fields |
| Payroll correctness | pure-core arithmetic in minor units, overflow-refusing, reconciled to time/leave records |
| Compliance posture | every mutation and sensitive read audited; retention rules documented |
| Privacy by unrepresentability | what must not be stored gets no column: no health cohort, symptom, diagnosis, or pulse author (WPM-D17/D20/D24/D25) |
| Subject rights honoured honestly | one-document export with named exclusions; erasure as anonymisation; floored retention sweep (WPM-D22) |

## Non-goals

- Not an identity registry (person/worker/organization services are).
- Not a tax engine — statutory tables are configuration/stubs, not
  jurisdiction-complete tax law.
- Not a course-content platform — the course-service owns courses;
  WPM owns *enrollments*.
- Not a general accounting ledger — payroll produces payslips and
  totals, not double-entry books.
