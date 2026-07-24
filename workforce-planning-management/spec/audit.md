# Audit & events

## Audit trail

Family conventions: every mutation writes an audit row (entity kind,
pid, action, actor, snapshot). Because HR data is regulated personal
data, **sensitive reads are audited too**: employee-record reads that
include salary, payslip reads, review-content reads, and
succession-plan reads each record who read what, when. Approval and
override actions (leave approvals, checklist waivers, calibration
changes, payroll approvals) carry their reason in the snapshot.

## Event kinds

`requisition_opened` / `requisition_filled`, `application_staged`,
`employee_hired` / `employee_activated` / `employee_terminated`,
`onboarding_item_completed`, `time_recorded`, `leave_requested` /
`leave_approved` / `leave_rejected`, `shift_assigned`,
`benefit_enrolled`, `review_submitted` / `review_shared`,
`training_completed`, `payroll_run_calculated` /
`payroll_run_approved` — the family envelope, deduped by consumers
on `event_id`; transactional outbox under the `outbox` transport.

## Integrity

State transitions + audit + outbox share one transaction; approval
races (two managers approving the same leave) are serialized with row
locks (`FOR UPDATE`, the patient-flow bed pattern); payslip
reconciliation (net = gross − deductions) is enforced before persist.
