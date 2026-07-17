# Domain model

The aggregates Patient Flow owns, with their fields. All upstream
identities are **EntityRef URNs** (`person:<uuid>`, `worker:<uuid>`,
`place:<uuid>`, `organization:<uuid>`) per
[cross-service-linking](../../agents/share/cross-service-linking.md);
Patient Flow's own records use public UUID `pid`s like every family
service. All tables carry `created_at`, `updated_at`, and soft-delete
`deleted_at`.

```
Site 1──* Ward 1──* Bay 1──* Bed
                │
                └──(virtual wards have virtual-slot Beds)

Stay *──1 person-ref          BedRequest *──1 person-ref
Stay 1──* Transfer            BedRequest 0..1──1 Bed (allocation)
Stay 1──* RedGreenDay
Stay 1──* InfectionFlag       Bed.deep_clean_required ← set on vacate
Stay 0..1──1 Bed (current)
```

## Site

A hospital site within the trust.

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | public id |
| `name` | text | e.g. "St Elsewhere General" |
| `place_ref` | EntityRef? | `place:<pid>` in place-service |
| `organization_ref` | EntityRef? | the trust, `organization:<pid>` |

## Ward

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `site_pid` | UUID | FK → Site |
| `name` | text | e.g. "Ward 7 — Respiratory" |
| `code` | text | short display code, unique per site |
| `kind` | enum | `inpatient` \| `assessment` \| `virtual` |
| `specialty` | text? | e.g. `respiratory`, `orthopaedics` |
| `open` | bool | closed wards accept no admissions |
| `escalation` | bool | temporary surge-capacity ward |
| `closed_to_admissions` | bool | infection outbreak / deep clean ([infection-control.md](infection-control.md)) |
| `place_ref` | EntityRef? | optional finer place record |

## Bay

A room or bay within a ward. Side rooms are single-bed bays.

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `ward_pid` | UUID | FK → Ward |
| `name` | text | e.g. "Bay A", "Side Room 1" |
| `sex_designation` | enum | `male` \| `female` \| `mixed` \| `flexible` — allocation rule input |
| `side_room` | bool | single-occupancy isolation-suited room |
| `closed_to_admissions` | bool | bay-level closure |

## Bed

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `bay_pid` | UUID | FK → Bay |
| `number` | text | display label, unique per bay |
| `state` | enum | `available` \| `reserved` \| `occupied` \| `awaiting_clean` \| `cleaning` \| `closed` — see [bed-management.md](bed-management.md) |
| `state_since` | timestamptz | when the current state began (turnaround metrics) |
| `closure_reason` | enum? | `infection` \| `maintenance` \| `staffing` \| `other` — required when `closed` |
| `deep_clean_required` | bool | set on vacate by an infectious stay |
| `isolation_capable` | bool | |
| `oxygen` | bool | piped oxygen at the bed head |
| `bariatric` | bool | |
| `virtual` | bool | true only for virtual-ward slots |

## Stay (the inpatient episode)

The central aggregate: one hospital stay for one patient.

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `person_ref` | EntityRef | `person:<pid>` — the patient; **never** raw demographics |
| `display_name` | text | denormalised cache for whiteboards; refreshable; maskable |
| `status` | enum | `admitted` \| `discharge_ready` \| `discharged` |
| `admitted_at` | timestamptz | |
| `source` | enum | `ed` \| `elective` \| `transfer_in` \| `virtual_admission` |
| `ward_pid` / `bed_pid` | UUID? | current location; null bed on a virtual ward |
| `home_location_note` | text? | virtual-ward stays: where the patient is |
| `named_nurse_ref` | EntityRef? | `worker:<pid>` |
| `consultant_ref` | EntityRef? | `worker:<pid>` |
| `senior_review_at` | timestamptz? | SAFER "S": last senior review |
| `edd` | date? | SAFER "A": expected discharge date |
| `ccd` | text? | clinical criteria for discharge, free text |
| `ccd_met` | bool | criteria met ⇒ candidate for `discharge_ready` |
| `discharge_pathway` | enum? | `p0` (simple home) \| `p1` (home with support) \| `p2` (community bed) \| `p3` (24h-care assessment) |
| `discharge_ready_at` | timestamptz? | start of any DTOC clock |
| `discharged_at` | timestamptz? | |
| `discharge_destination` | enum? | `home` \| `home_with_support` \| `community_hospital` \| `care_home` \| `other_acute` \| `deceased` \| `self_discharge` |
| `alerts` | text[] | free-form whiteboard alert chips (falls risk, DNAR present, dementia, …) — capped list |

Derived: `length_of_stay` (now/discharged_at − admitted_at),
`dtoc` (`discharge_ready` and not discharged past a threshold).

## Transfer

Immutable record of each move (also the audit anchor for "visibility
of current location").

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `stay_pid` | UUID | FK → Stay |
| `from_bed_pid` | UUID? | null on admission placement |
| `to_bed_pid` | UUID? | null on discharge / to-virtual |
| `reason` | enum | `admission` \| `clinical` \| `capacity` \| `isolation` \| `patient_request` \| `discharge` \| `step_up` \| `step_down` |
| `moved_at` | timestamptz | |
| `moved_by_ref` | EntityRef? | `worker:<pid>` actor |

## BedRequest (demand queue)

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `person_ref` | EntityRef | patient needing a bed |
| `origin` | enum | `ed` \| `elective` \| `ward_transfer` \| `external` \| `virtual_step_up` |
| `target_ward_pid` | UUID? | preferred ward |
| `specialty` | text? | alternative to naming a ward |
| `priority` | enum | `emergency` \| `urgent` \| `routine` |
| `requirements` | flags | `isolation`, `side_room`, `oxygen`, `bariatric`, sex |
| `status` | enum | `open` \| `allocated` \| `fulfilled` \| `cancelled` |
| `allocated_bed_pid` | UUID? | set on allocation (bed → `reserved`) |
| `requested_at` / `resolved_at` | timestamptz | wait-time metrics |

## RedGreenDay

One row per stay per calendar day ([patient-journey.md](patient-journey.md)).

| Field | Type | Notes |
|---|---|---|
| `stay_pid` + `day` | UUID + date | unique pair |
| `classification` | enum | `red` \| `green` (days start red) |
| `delay_reasons` | enum[≤2] | coded, e.g. `awaiting_senior_review`, `awaiting_diagnostics`, `awaiting_pharmacy`, `awaiting_transport`, `awaiting_therapy_assessment`, `awaiting_social_care`, `awaiting_community_bed`, `awaiting_care_package`, `family_choice`, `internal_process`, `other` |
| `note` | text? | |

## InfectionFlag

Per-stay infection-control precaution ([infection-control.md](infection-control.md)).

| Field | Type | Notes |
|---|---|---|
| `pid` | UUID | |
| `stay_pid` | UUID | FK → Stay |
| `precaution` | enum | `contact` \| `droplet` \| `airborne` \| `protective` |
| `organism` | text? | e.g. `covid-19`, `c-diff`, `mrsa`, `norovirus` |
| `status` | enum | `suspected` \| `confirmed` \| `cleared` |
| `requires_side_room` | bool | allocation-rule input |
| `flagged_at` / `cleared_at` | timestamptz | |

## Audit & events

Every mutation writes an audit row (actor, old/new values, timestamp)
and emits a family-standard event envelope: `ward`/`bay`/`bed`
`created|updated|deleted`, `bed_state_changed`, `stay_admitted`,
`stay_transferred`, `stay_discharge_ready`, `stay_discharged`,
`bed_request_created|allocated|cancelled`, `infection_flagged|cleared`,
`red_green_recorded`. See [audit.md](audit.md).
