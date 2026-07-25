# Audit & events

## Audit trail

Family conventions: every mutation writes an audit row (entity kind,
pid, action, actor, snapshot). Because HR data is regulated personal
data, **sensitive reads are audited too**: employee-record reads that
include salary, payslip reads, review-content reads, succession-plan
reads, 360 report reads (`report_read`), unmasked adjustment-request
reads (`adjustments_read`), unmasked assessment-score reads, and
subject-access exports (`subject_access_exported`) each record who
read what, when. Approval and override actions (leave approvals,
checklist waivers, calibration changes, payroll approvals, adjustment
decisions, erasures with per-step counts, retention sweeps with
counts) carry their detail in the snapshot.

**One designed exception**: a pulse submission's audit row carries
**no actor** (WPM-D20) — the trail records that a submission
happened, never who made it, because an actor-stamped row would
silently defeat the structural anonymity.

## Event kinds

`requisition_opened` / `requisition_filled`, `application_staged`,
`employee_hired` / `employee_activated` / `employee_terminated`,
`onboarding_item_completed`, `time_recorded`, `leave_requested` /
`leave_approved` / `leave_rejected`, `shift_assigned`,
`benefit_enrolled`, `review_submitted` / `review_shared`,
`training_completed`, `payroll_run_calculated` /
`payroll_run_approved` — the family envelope, deduped by consumers
on `event_id`; transactional outbox under the `outbox` transport.
Later rounds added audit actions such as `acknowledged` (wellbeing),
`submitted` (pulse — actor-less; 360 responses — actor-carried),
`status_changed` (appraisals, adjustments), `answered` / `completed`
(ergonomics), `erased`, `retention_swept`, and in-app notification
kinds (`appraisal_request` / `appraisal_shared` /
`adjustment_update` — reference-only, WPM-D23).

## Integrity

State transitions + audit + outbox share one transaction; approval
races (two managers approving the same leave) are serialized with row
locks (`FOR UPDATE`, the patient-flow bed pattern); payslip
reconciliation (net = gross − deductions) is enforced before persist.
