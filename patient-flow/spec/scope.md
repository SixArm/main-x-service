# Scope

## In scope (v1)

- **Physical topology**: sites → wards → bays → beds, with bed
  attributes (side room, isolation-capable, oxygen, bariatric) and
  bay sex-designation.
- **Bed lifecycle**: the full state machine
  (available / reserved / occupied / awaiting-clean / cleaning /
  closed) with timestamps and closure reasons.
- **Stays**: admission (from ED, elective, or external transfer),
  ward/bed transfers, discharge (with destination pathway), soft
  delete, derived length of stay.
- **SAFER fields**: senior-review flag, expected discharge date
  (EDD), clinical criteria for discharge (CCD) and whether they are
  met.
- **Red2Green journal**: per-stay, per-day red/green classification
  with up to two coded delay reasons per red day.
- **Discharge readiness & DTOC**: `discharge_ready` state, discharge
  pathway (0–3), delay reasons, DTOC counting.
- **Bed requests**: create, prioritise, allocate (rule-checked),
  cancel.
- **Infection control**: per-stay precaution flags (contact /
  droplet / airborne, named organism, Covid status), per-bed
  deep-clean-required, bay/ward closure to admissions.
- **Virtual ward**: a ward of `kind = virtual` whose beds are
  virtual slots; stays in it carry a home location note instead of a
  physical bed reference.
- **Read views**: ward whiteboard, bed card, hospital at a glance,
  patient locate, capacity metrics.
- **Audit + events**: audit rows and event envelopes for every
  mutation, per family conventions.
- **Auth**: offline PASETO v4.public verification + ABAC via
  `authentication-verifier`, blanket `PATIENT_FLOW_REQUIRE_AUTH`
  (default-off), record-level masking posture for patient-identifying
  fields.

## Out of scope (v1, some deferred to [roadmap.md](roadmap.md))

- **ED attendance tracking** — ED is an admission `source`, nothing
  more.
- **HL7 v2 ADT / FHIR ingest** — the domain model is deliberately
  ADT-shaped (admit/transfer/discharge events) so a later adapter can
  map A01/A02/A03 traffic onto the API, but no HL7 listener ships in
  v1. See [integrations.md](integrations.md).
- **Theatres, outpatients, diagnostics scheduling.**
- **Rostering, e-obs, EWS scores, clinical noting.**
- **Predictive analytics / forecasting models** — v1 capacity views
  are arithmetic over live state (occupancy, EDD-today counts), not
  ML forecasts.
- **Patient/family-facing views** — operator (staff) UI only.
- **Multi-trust federation** — one deployment serves one trust; the
  trust is a single organization-service reference.
- **RFID / RTLS location feeds** — bed cards are updated by staff
  action (touchscreen, desktop, mobile), not sensor feeds.

## Boundary with the Main X Index family

| Concern | Owner | Patient Flow holds |
|---|---|---|
| Patient identity, demographics, NHS number | person-service | `person:<pid>` EntityRef |
| Staff identity, role, registration | worker-service | `worker:<pid>` EntityRef |
| Hospital sites, buildings | place-service | `place:<pid>` EntityRef on Site/Ward |
| Trust / provider organization | organization-service | `organization:<pid>` EntityRef |
| Sign-on, tokens, attributes | authentication-service | verified PASETO claims |
| Wards, bays, beds, stays, requests, flags, audit | **patient-flow** | its own PostgreSQL tables |

Patient Flow never duplicates upstream records; it caches display
names at most (denormalised, refreshable) and always keeps the URN.
