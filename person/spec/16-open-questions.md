## 16. Open Questions

Entity-level open questions (`EOQ-N`). Subproject-local questions stay
in the subproject's spec (service §16, matcher §22, front-end §16).
Questions resolve into §13 tasks or §5–§9 amendments.

- **EOQ-1 — Two matchers in one service.** The service carries both
  its in-service matching stack (`src/matching/{algorithms,scoring,
  phonetic}.rs`, weights in service §6.2) *and* the embedded canonical
  `person-matcher` via the adapter. Which one is authoritative for
  which endpoint, long-term? Options: (a) migrate all scoring onto the
  canonical matcher and retire the in-service algorithms; (b) keep the
  in-service stack for blocking/candidate-ranking and the canonical
  matcher for final pairwise decisions; (c) status quo, documented.
  Until resolved, changes to either MUST keep the bridge tests green.
- **EOQ-2 — Data residency and cross-border matching.** A worldwide
  governmental deployment will face per-jurisdiction residency rules.
  Does each region run its own registry with federated matching, or is
  there one global store with regional read replicas? Affects §8.4,
  §10, §12.
- **EOQ-3 — Identifier issuance.** Should the person entity mint a
  citizen-facing identifier (its UUID is internal), or always defer to
  national schemes? The matcher deliberately refuses to score
  `local_id` — the same caution may apply to exposing UUIDs to
  agencies as quasi-identifiers.
- **EOQ-4 — Worker-entity overlap.** A person who is also a worker
  exists in both registries. Is the linkage a `PersonLink`, a shared
  identifier, or an event-driven sync? Today it is convention only
  (cross-reference via `person_id`).
- **EOQ-5 — FHIR Person vs national exchange standards.** FHIR R5
  Person is implemented (partially); some jurisdictions mandate other
  exchange formats. Decide per-jurisdiction adapters vs FHIR-only.
