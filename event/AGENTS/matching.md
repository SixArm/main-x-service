# Matching — Event Entity

The entity has **two matching surfaces plus a bridge**. Know which
one you are editing before you touch weights.

## 1. In-service matcher (powers the REST endpoints)

`ProbabilisticMatcher` / `DeterministicMatcher` in the service's
`src/matching/`. Components: name (Jaro-Winkler + Levenshtein +
Soundex floor), start/end-date exponential decay (1 h half-life),
**window overlap** (Jaccard of `[start, end)` intervals), location
(variant-dispatched), organizer / performer / attendee parties,
identifier. Short-circuits to 1.0 on a strong identifier
(`BookingNumber`, `ConfirmationCode`, `TicketNumber`, `EncounterId`,
`TransactionId`). Quality buckets: Definite ≥ 0.95 / Probable ≥ 0.85
/ Possible ≥ 0.50 / Unlikely.

→ Weights, rules, per-component algorithms:
[`event-service-rust-crate/AGENTS/matching.md`](../event-service-rust-crate/AGENTS/matching.md).

## 2. Canonical matcher crate (the reference algorithm)

`event-matcher`: weight-renormalised probabilistic score over name /
start / end / location / category / country / event_ids / organizer /
performers / url (missing fields skip — they neither raise nor
lower), Gaussian temporal decay (scale 3600 s), Haversine geo (scale
100 m), optional Soundex bonus. Deterministic rule: shared
`(scheme, value)` event ID **or** identical normalised name + same
`start_date` instant. Thresholds: default 0.80 / strict 0.95 /
lenient 0.65; confidence bands High ≥ 0.90 / Medium ≥ 0.75 / Low.

→ Algorithm guide:
[`event-matcher-rust-crate/AGENTS/matching-algorithm.md`](../event-matcher-rust-crate/AGENTS/matching-algorithm.md)
and the crate [README](../event-matcher-rust-crate/README.md).

## 3. The bridge

The service embeds the crate (re-export `matcher_lib`) and projects
records through
[`src/matching/adapter.rs`](../event-service-rust-crate/src/matching/adapter.rs)
(`to_matcher_event`). Contract: entity spec
[§5.3](../spec/05-domain-model.md). Pinned by
[`tests/duplicate_detection.rs`](../event-service-rust-crate/tests/duplicate_detection.rs)
(16 tests) — change adapter or scoring, change a bridge test.

## Which one wins?

The crate is the **canonical reference algorithm**; the in-service
matcher is what `/api/v1/events/match` actually runs today. Their
weights differ (see entity spec [§6.1](../spec/06-functional-requirements.md));
convergence is an open decision (EOQ-1 / task ET-4). Do not "fix"
one to match the other without that decision.

## Editing rules

- Changing crate weights/thresholds → crate spec §7 + CHANGELOG in
  the same PR (crate golden rule).
- Changing in-service weights → service spec §6.2 + its AGENTS
  matching doc.
- Changing the adapter → entity spec §5.3 + bridge test.
- Time-zone-aware matching is an open gap (service T-2); don't
  assume wall-clock equivalence across zones is handled.
