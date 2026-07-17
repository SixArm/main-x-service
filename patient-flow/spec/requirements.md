# Requirements

Numbered requirements with user stories and acceptance criteria.
IDs are stable; design decisions ([design.md](design.md)) and tasks
([tasks.md](tasks.md)) trace to them. The G-Cloud category features
map: PF-R1–R3 bed management, PF-R4–R7 inpatient management +
patient flow, PF-R8 whiteboard, PF-R9 location visibility, PF-R10
at-a-glance/capacity, PF-R11 infection control, PF-R12 virtual ward,
PF-R13–R15 cross-cutting.

## PF-R1 — Physical topology

*As a site administrator I can model sites, wards, bays, and beds so
the system mirrors the physical hospital.*

- CRUD (soft-delete) for Site / Ward / Bay / Bed with the
  [domain-model.md](domain-model.md) fields; `422` on invalid input.
- Ward kinds `inpatient` / `assessment` / `virtual`; bay sex
  designation; bed attributes (side room via bay, isolation, oxygen,
  bariatric).

## PF-R2 — Bed state machine

*As ward staff I can see and change each bed's live state so free
beds are findable instantly.*

- States and transitions exactly per
  [bed-management.md](bed-management.md); illegal transitions `422`
  naming the current state.
- Every transition stamps `state_since` and is audited + evented.
- A bed is occupied iff exactly one active stay references it,
  enforced under concurrent requests.

## PF-R3 — Bed requests & allocation

*As a flow coordinator I can queue bed demand and allocate
rule-checked beds.*

- Create / list / cancel requests with origin, priority,
  requirements.
- Eligibility per the five allocation rules; eligible beds returned
  ranked; allocation flips the bed to `reserved`; rules 2 and 5
  overridable with a recorded reason.
- An open request reports its live eligible-bed count.

## PF-R4 — Admission

*As ward staff I can admit a patient into a bed.*

- Admit against a `person:` URN (shape-validated; display name
  resolved best-effort) with source; bed must be available/reserved;
  creates stay, occupies bed, writes placement transfer, fulfils
  request, audits, emits.

## PF-R5 — Transfer

*As ward staff I can move a patient to another bed/ward with a
reason.*

- Destination re-checked against allocation rules; old bed →
  `awaiting_clean`; transfer row records from/to/reason/actor.

## PF-R6 — Discharge readiness & discharge

*As an MDT I can mark a patient discharge-ready and then discharged,
and the system counts DTOC.*

- Discharge-ready requires EDD set + CCD met; stamps
  `discharge_ready_at` + pathway P0–P3.
- Discharge stamps destination + time, vacates bed, finalises LOS.
- DTOC = discharge-ready, still in bed, past the configured grace
  (default midnight of the ready day); count + bed-days exposed.

## PF-R7 — SAFER & Red2Green

*As a ward leader I can run SAFER and Red2Green on the board.*

- `senior_review_at`, `edd`, `ccd`, `ccd_met` on every stay; missing
  EDD and no-review-today surfaced as chips.
- One Red2Green row per stay-day; days default red; ≤2 coded delay
  reasons per red day; same-day editable, then frozen; reason
  aggregates queryable.

## PF-R8 — Ward whiteboard

*As ward staff I see one live bed card per bed and act from it.*

- `GET /api/whiteboard/{ward}` returns bay-ordered bed cards with
  the full [whiteboard.md](whiteboard.md) field set; `as_of`
  timestamp; ETag/`updated_since` polling.
- Every card action maps to an existing API mutation (no
  whiteboard-only writes).

## PF-R9 — Patient locate

*As authorised staff I can find where a patient is right now.*

- `GET /api/locate/{person_ref}` → active stay's site/ward/bay/bed
  or virtual location, else latest discharged stay; read audited;
  ABAC-gated.

## PF-R10 — Hospital at a glance & capacity

*As a site manager I see live per-ward and site capacity.*

- Per-ward rows + site tiles per [capacity.md](capacity.md):
  occupancy, availability, predicted discharges today, predicted
  available by midnight, DTOC, open requests, closures, escalation,
  virtual census; long-stay, early-discharge, turnaround, outlier
  metrics; mirrored as Prometheus gauges.

## PF-R11 — Infection control

*As an IPC nurse I can flag precautions and control admissions.*

- Stay-level flags (precaution/organism/status/side-room-needed);
  transfer restrictions while uncleared; vacate sets
  `deep_clean_required`; deep clean gates re-availability.
- Bay/ward `closed_to_admissions` refuses allocation; reopening
  requires terminal cleans done; capacity views expose closed-for-
  infection counts.

## PF-R12 — Virtual ward

*As a virtual-ward team I manage hospital-at-home on the same
board.*

- Virtual wards with virtual slots; step-down / step-up / direct
  admission flows per [virtual-ward.md](virtual-ward.md); no
  cleaning cycle; census in capacity views.

## PF-R13 — Audit & events

- Every mutation: audit row + event envelope in the same
  transaction; sensitive reads audited; overrides carry reasons;
  ward-scoped `since` audit query serves handover.

## PF-R14 — AuthN/AuthZ

- Family PASETO offline verification; `PATIENT_FLOW_REQUIRE_AUTH`
  default-off blanket guard; ABAC with ward-scoping and `mask`
  obligation redacting patient names; the [testing.md](testing.md)
  auth matrix passes.

## PF-R15 — Operability

- OpenAPI/Swagger, header API versioning, tracing + OTLP,
  `/metrics.prom`, health endpoints, Podman packaging, seed task
  with synthetic demo hospital, stub-mode boot with no upstreams.
