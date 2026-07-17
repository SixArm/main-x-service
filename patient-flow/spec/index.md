# Patient Flow — Specification

This directory is the **single source of truth** for the cross-cutting
Patient Flow specification, shared by both editions. Each subproject's
own `spec/` adds stack-specific detail and links back here.

> ⚠️ **Demo software, not a regulated medical record.** This project
> models NHS ward-flow practice for demonstration and integration
> purposes. It is not a certified medical device, not an EPR, and not
> assured for clinical use. See [regulatory.md](regulatory.md).

## What this project is

A **hospital patient flow and bed management system** for a UK NHS
hospital setting: digital ward whiteboards, live bed states, inpatient
stays from admission to discharge, bed requests and allocation,
infection-control flags, virtual wards, and 'hospital at a glance'
capacity views.

The core questions it answers, in real time:

- *"Where is patient X right now?"*
- *"Which beds are free, closed, or being cleaned — on this ward and
  across the hospital?"*
- *"Who is ready to go home today, and who is delayed — and why?"*

It is a **consumer application**: it does not register identities
itself. Patients are [person-service](../../person/person-service-with-loco/)
records, staff are [worker-service](../../worker/worker-service-with-loco/)
records, physical sites are [place-service](../../place/place-service-with-loco/)
records, and the trust is an
[organization-service](../../organization/organization-service-with-loco/)
record. Patient Flow owns only the **operational state**: wards, bays,
beds, stays, transfers, bed requests, and their audit trail.

Modelled on real NHS practice and the
[Access Patient Flow Manager](https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud/services/803442955706912)
G-Cloud service category (features: patient flow, bed management,
whiteboard, inpatient management, infection control, hospital capacity,
interactive bed cards, current-location visibility, hospital at a
glance, virtual ward), plus the NHS **SAFER patient flow bundle** and
**Red2Green** day methodology. See [purpose.md](purpose.md).

## Two editions

| Subproject | Role | Stack |
|---|---|---|
| [patient-flow-service-with-rust](../patient-flow-service-with-rust/) | Back-end JSON API | Rust, Loco (Axum + SeaORM), PostgreSQL |
| [patient-flow-front-end-with-svelte](../patient-flow-front-end-with-svelte/) | Ward whiteboard / touchscreen UI | SvelteKit 2, Svelte 5 runes, TypeScript |

Both serve the same domain. The Loco edition exposes the API and owns
the operational data; the Svelte edition is the whiteboard and
operations client (large-format ward touchscreens plus desktop/mobile).

## Specification (topic files)

| File | Covers |
|---|---|
| [purpose.md](purpose.md) | Problem statement, goals, benefits; research grounding |
| [scope.md](scope.md) | In scope / out of scope / explicitly deferred |
| [domain-model.md](domain-model.md) | Ward, Bay, Bed, Stay, Transfer, BedRequest, flags — the aggregates and their fields |
| [bed-management.md](bed-management.md) | The bed state machine, cleaning turnaround, closures, allocation rules |
| [patient-journey.md](patient-journey.md) | Admission → transfer → discharge flows; EDD/CCD; SAFER; Red2Green; DTOC delay reasons |
| [whiteboard.md](whiteboard.md) | Ward whiteboard, interactive bed cards, hospital-at-a-glance, patient locate |
| [virtual-ward.md](virtual-ward.md) | Virtual wards (hospital-at-home) as first-class wards |
| [infection-control.md](infection-control.md) | Infection flags, isolation, precautions, deep-clean-on-vacate (including Covid) |
| [capacity.md](capacity.md) | Occupancy, availability, predicted discharges, escalation, capacity metrics |
| [integrations.md](integrations.md) | Upstream Main X Index services; EntityRef URNs; ADT-ingest posture |
| [auth.md](auth.md) | SSO, PASETO verification, ABAC posture, `PATIENT_FLOW_REQUIRE_AUTH` |
| [audit.md](audit.md) | Audit trail, event stream, clinical-handover support |
| [architecture.md](architecture.md) | Editions, layering, persistence, event emission |
| [testing.md](testing.md) | Test strategy per edition |
| [regulatory.md](regulatory.md) | Demo status, UK DPA/GDPR posture, what production would require |
| [roadmap.md](roadmap.md) | Later phases beyond the v1 task queue |
| [glossary.md](glossary.md) | DTOC, EDD, CCD, MDT, SAFER, Red2Green, side room, … |

## Specification-driven delivery (SDD)

Three lock-step files drive delivery:

- [requirements.md](requirements.md) — numbered requirements (`PF-R*`)
  with user stories and acceptance criteria.
- [design.md](design.md) — numbered design decisions (`PF-D*`).
- [tasks.md](tasks.md) — the live delivery checklist (`PF-T*`), phased;
  every task traces to design and requirement ids.

A change starts in `requirements.md`, is shaped in `design.md`, is
queued in `tasks.md`, and only then lands as code in a subproject.
**No code lands without the spec describing it.**

## References

- [Access Patient Flow Manager — G-Cloud 14 listing](https://www.applytosupply.digitalmarketplace.service.gov.uk/g-cloud/services/803442955706912)
- [The SAFER patient flow bundle (NHS ECIP quick guide)](https://fabnhsstuff.net/fab-stuff/the-safer-patient-flow-bundle)
- [Red and Green bed days (NHS England)](https://www.england.nhs.uk/south/wp-content/uploads/sites/6/2016/12/rig-red-green-bed-days.pdf)
- [SAFER + Red2Green length-of-stay study (BMJ Open Quality, PMC10806560)](https://pmc.ncbi.nlm.nih.gov/articles/PMC10806560/)
- [HL7 v2 bedStatus vocabulary](https://terminology.hl7.org/1.0.0/CodeSystem-v2-0116.html)
- [IHE Bed Management profile notes](https://wiki.ihe.net/index.php/Bed_Management)
- Sibling consumer app precedent: [case-folder](../../case-folder/spec/index.md)
