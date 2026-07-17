# Audit & events

## Audit trail

Every mutation writes an audit row (family conventions,
[auditability.md](../../agents/share/auditability.md)): entity type +
pid, action, old/new values as JSON, actor (`worker:` ref or token
`sub`), IP/user-agent, timestamp. Additionally — because flow data is
personal data — **sensitive reads are audited**: patient locate, stay
detail, and unmasked whiteboard renders record who read what.

Audit is the **clinical-handover substrate** (a G-Cloud benefit made
concrete): `GET /api/audits?ward={pid}&since={ts}` answers "what
changed on my ward since my last shift" — admissions, moves,
discharges, new flags, red days — replacing the paper handover sheet
as the source of truth for *what happened* (clinical content stays in
the EPR, out of scope).

Rule-override events (sex-segregation override, outlier placement)
carry their override reason in the audit row and are queryable — they
are reportable governance events, not buried facts.

## Event stream

Family-standard envelopes via the shared streaming seam
(`PATIENT_FLOW_EVENT_TRANSPORT`, default `memory`; outbox → Fluvio
when the durable bus lands). Event kinds:

| Kind | Emitted on |
|---|---|
| `ward_created` / `ward_updated` / `ward_deleted` (likewise bay, bed) | topology CRUD |
| `bed_state_changed` | every bed transition, with from/to state + reason |
| `stay_admitted` / `stay_transferred` / `stay_discharge_ready` / `stay_discharged` | journey flows |
| `bed_request_created` / `bed_request_allocated` / `bed_request_cancelled` | demand queue |
| `infection_flagged` / `infection_cleared` | IPC flags |
| `red_green_recorded` | day journal |

Consumers dedupe on `event_id` (at-least-once). The whiteboard's
future SSE push is a consumer of this same stream.

## Integrity

- Audit + event rows commit **in the same transaction** as the change
  (family invariant; outbox pattern).
- Bed-state transitions and stay moves lock the affected bed rows
  (`SELECT … FOR UPDATE`) so two coordinators cannot double-place a
  bed — same posture as the family's merge/upsert concurrency rules.
- The audit log is append-only; no update/delete surface exists.
