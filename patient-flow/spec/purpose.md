# Purpose

## Problem

Hospital wards traditionally run on paper: whiteboards with marker
pens, printed handover sheets, and phone calls to find a free bed.
The consequences are well documented across the NHS:

- **Nobody has a live view of bed supply.** Site teams ring round
  wards to count free beds; the answer is stale before the call ends.
- **Discharges drift.** Without a visible expected discharge date
  (EDD) and clinical criteria for discharge (CCD), patients wait for
  reviews, transport, pharmacy, or social care — each wait a "red day"
  that adds no clinical value but extends length of stay.
- **Delayed transfers of care (DTOC).** Patients who are medically
  ready to leave occupy acute beds because the next step (home care
  package, community bed, assessment) is not arranged.
- **Handover risk.** Paper handover sheets go missing, are out of
  date, and carry no audit trail.
- **Infection control is invisible.** Which beds need a deep clean,
  which bays are closed to admissions, which patients need isolation —
  this state lives in people's heads and on sticky notes.

## What Patient Flow does

A single operational system of record for the **flow state** of a
hospital:

1. **Bed management** — every bed's live state (available, reserved,
   occupied, awaiting clean, cleaning, closed) with timestamps, so
   turnaround is measurable and allocation is instant.
2. **Digital ward whiteboard** — interactive bed cards on ward
   touchscreens showing who is in each bed, their named nurse and
   consultant, EDD, discharge status, infection flags, and alerts.
3. **Inpatient management** — the stay from admission through ward
   transfers to discharge, with the SAFER bundle fields (senior
   review, EDD, CCD) and a Red2Green day journal with delay reasons.
4. **Bed requests & allocation** — a demand queue (from ED, elective
   admissions, transfers) matched against supply, with allocation
   rules (sex segregation, isolation need, specialty).
5. **Infection control** — patient precaution flags and bed/bay
   closure workflow, including deep-clean-on-vacate (Covid and other
   organisms).
6. **Hospital at a glance** — per-ward and whole-site occupancy,
   availability, expected discharges today, DTOC count, escalation
   status.
7. **Virtual ward** — hospital-at-home as a first-class ward whose
   "beds" are virtual slots, on the same whiteboard machinery.
8. **Audit trail** — every state change recorded with actor and
   timestamp, supporting clinical handover and retrospective review.

## Goals (the benefits, made testable)

Derived from the G-Cloud category benefits and NHS improvement
literature; each maps to requirements in
[requirements.md](requirements.md):

| Goal | Mechanism |
|---|---|
| Reduce paper-based processes; promote safe wards | digital whiteboard + bed cards replace marker boards and printed sheets |
| Real-time information access | live bed/stay state over an API; whiteboards update without refresh ceremony |
| Reduce length of stay | visible EDD/CCD, Red2Green journal, delay-reason capture → delays surfaced same-day |
| Clear view of available beds | bed state machine + at-a-glance views |
| Audit trail supporting clinical handover | immutable audit log of every flow event |
| Avoid procuring extra beds | measured occupancy + turnaround shortens effective bed cycle |
| Manage, maintain, allocate bed requests | bed-request queue with rule-checked allocation |
| Facilitate multi-disciplinary team (MDT) discussion | whiteboard as the shared MDT artefact; per-patient journey view |
| Improve the patient journey | fewer moves, right bed first time, shorter waits |
| Prevent delayed transfer of care (DTOC) | discharge-ready state + destination pathway + delay reasons made visible and countable |

## Non-goals

Patient Flow is **not**:

- an electronic patient record (no clinical notes, observations,
  prescribing, results);
- a master patient index (person-service is; Patient Flow references
  `person:` pids and never merges identities);
- a rostering/staffing system (named nurse/consultant are references
  to worker-service records, not shift plans);
- an emergency-department tracker (ED attendances are out of scope;
  ED appears only as an admission source and a bed-request origin).

See [scope.md](scope.md) for the full boundary.
