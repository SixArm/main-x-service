# Roadmap

Beyond the v1 task queue ([tasks.md](tasks.md)), in rough order of
pull:

- **Server-push whiteboards** — SSE from the event stream so wall
  screens update without polling.
- **HL7 v2 ADT adapter** — map A01/A02/A03 (+A08/A11/A12/A13
  corrections) from a trust PAS onto the admit/transfer/discharge
  API ([integrations.md](integrations.md)); then a FHIR R4/R5
  `Encounter` + `Location` read surface for FHIR-native consumers
  (note: the family fhir.md contract targets the entity registries;
  Patient Flow would add `Encounter`, which no registry owns).
- **Discharge task checklists** — per-pathway to-do lists (TTOs,
  transport booked, care package confirmed) behind the
  discharge-ready gate.
- **Cohort suggestion** — allocator proposes outbreak cohorting
  ([infection-control.md](infection-control.md)).
- **Capacity forecasting** — admission-demand and LOS prediction;
  strictly additive over the v1 arithmetic views.
- **Porter/cleaning task dispatch** — turn `awaiting_clean` into a
  work queue with assignment and SLAs.
- **RTLS/RFID feeds** — sensor-driven bed occupancy reconciliation.
- **link-graph edge** — a governed `inpatient_at` read-model edge,
  if a cross-service consumer materialises (needs its own §10-style
  governance review first).
- **Multi-trust / ICS view** — system-level capacity across trusts
  (mutual-aid visibility).
