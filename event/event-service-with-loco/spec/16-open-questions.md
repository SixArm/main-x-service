## 16. Open Questions

- ~~**OQ-1 — FHIR mapping.**~~ **RESOLVED** (§13 T-1/T-10, 2026-07-07):
  `Appointment` is the shipped default (best-effort, `low` fidelity —
  see §6.8 for the documented gaps). `Encounter` remains a roadmap
  alternative, not an open question blocking anything today.
- **OQ-2 — Capacity invariant strictness.** Should we reject events
  where `remaining > maximum_total` outright (422), or accept and
  warn? Today: reject.
- **OQ-3 — `previous_start_date` semantics.** Required when
  `event_status == Rescheduled`? Today: not required, but consumers
  expect it.

