## 16. Open Questions

- **OQ-1 — FHIR mapping.** Encounter or Appointment (or both)?
  Encounter fits domain visits / interactions; Appointment fits
  scheduling. A "best-fit by `event_type`" dispatch is a third option.
- **OQ-2 — Capacity invariant strictness.** Should we reject events
  where `remaining > maximum_total` outright (422), or accept and
  warn? Today: reject.
- **OQ-3 — `previous_start_date` semantics.** Required when
  `event_status == Rescheduled`? Today: not required, but consumers
  expect it.

