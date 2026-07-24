# Pillar 3 — HR service delivery & admin

## The employee record

The single source of **employment** truth (identity stays upstream):
status lifecycle `onboarding → active ⇄ on_leave → offboarding →
terminated | retired`, employment facts (type, FTE, department, job
title, location), the manager edge, hire/termination dates, and the
salary (minor units — **sensitive**, masked by default). Terminating
an employee stamps date + reason, cancels future shift assignments,
and closes open enrollments (all audited).

## Org chart

Derived from `manager_pid`: the tree is materialized per request
(`GET /org-chart?root=`), cycles are refused at write (the DFS check
patient-flow uses for dependencies), and a manager's **team** view is
one query — the manager ABAC persona is scoped by exactly this edge.

## Self-service

The same API serves three personas via ABAC (no separate endpoints):

- **Employee** — reads their own record (salary visible to self),
  requests leave, sees their payslips, their shifts, their reviews.
- **Manager** — reads their team's records (salary masked unless
  granted), approves leave, sees team calendars and review states.
- **HR / payroll** — full remit per policy; payroll fields visible
  only to the payroll persona.

Ownership is expressed with the family's `$sub` policy template
(`resource.person = $sub`-style rules); see [auth.md](auth.md).

## Benefits administration

Plans (health / pension / perk, with employee+employer costs) and
per-employee enrollments with effective ranges. Enrollment changes
are audited; active enrollments feed payslip deduction lines
(pension) in payroll. A plan cannot be deleted while enrollments are
active (soft-close instead).

## Health & wellbeing entitlements

Configurable public-health entitlement rules (e.g. NHS vaccination
cohorts — flu for frontline roles, shingles at 65+) evaluated over
**non-clinical** facts only (age via the upstream person record,
role, department). Eligible employees see an informational prompt in
self-service and record one acknowledgement
(`booked | done | declined | dismissed`); one optional reminder for
multi-dose courses. Acknowledgements are employee-owned and
aggregate-only for HR — never manager-visible; WPM stores no
vaccination status or clinical data ([design.md](design.md)
WPM-D17; [requirements.md](requirements.md) WPM-R25).
