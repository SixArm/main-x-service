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
- **OQ-4 — Persist the dedup review queue?** Investigated 2026-08-29
  (repo `tasks.md` PRO-P7): `POST /api/events/deduplicate` computes
  `ReviewQueueItem`s on the fly and returns them in the response; there
  is no `review_queue` table (§10.1), no listing endpoint, and no
  decision endpoint. This is a genuine gap against the family pattern
  in
  [match-search-merge.md](../../../agents/share/match-search-merge.md),
  which person / worker / place / thing / organization all implement
  (a persisted table, normalized-pair `UNIQUE` upsert, `GET
  .../review-queue`, `POST .../review-queue/{id}/decision`). Today's
  behaviour also means a returned `AutoMerged` item is a label only —
  no merge is actually performed — since there is no follow-up call
  that could reference the item by its throwaway `id`. Adding
  persistence + the two endpoints (mirroring person's schema and
  routes) would close this; not built here — a real migration + model
  + repository + two endpoints is bigger than a documentation task.
  Tracked as future work, not scheduled.

