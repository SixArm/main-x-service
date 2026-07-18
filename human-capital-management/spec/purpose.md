# Purpose

## Problem

Workforce administration is scattered: applicants tracked in inboxes,
onboarding on paper, leave in spreadsheets, schedules on whiteboards,
reviews in documents nobody reopens, training records unverifiable,
succession in someone's head, and payroll reconciled by hand against
all of the above. Every handoff between those silos loses data,
delays people, and creates compliance risk — employment records are
regulated personal data with statutory retention duties.

## What HCM does

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

### 3. HR service delivery & admin

- **Core HR database** — the employee record as the single source of
  employment truth (identity stays in the person/worker services);
  the manager chain yields the **org chart**.
- **Self-service** — employees view their own record, request leave,
  see payslips; managers see their team; HR sees their remit —
  enforced as ABAC policy, not code.
- **Benefits administration** — plans (health, pension, perks) and
  per-employee enrollments with effective dates.

### 4. Talent management & development

- **Performance reviews** — cycles, per-employee goals, continuous
  feedback entries, and appraisals with calibrated ratings.
- **Learning (LMS)** — training enrollments referencing the family's
  [course-service](../../course/course-service-with-loco/) courses
  and instances (compliance courses, certifications with expiry).
- **Succession planning** — key positions, incumbents, and a
  readiness-rated pipeline of high-potential employees.

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

## Non-goals

- Not an identity registry (person/worker/organization services are).
- Not a tax engine — statutory tables are configuration/stubs, not
  jurisdiction-complete tax law.
- Not a course-content platform — the course-service owns courses;
  HCM owns *enrollments*.
- Not a general accounting ledger — payroll produces payslips and
  totals, not double-entry books.
