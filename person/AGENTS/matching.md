# Matching — Entity-Level Orientation

The person entity has **two matching stacks**; know which one you are
touching before you edit anything.

## The two stacks

| Stack | Lives in | What it is | Reference |
|---|---|---|---|
| In-service matcher | service `src/matching/{algorithms,scoring,phonetic}.rs` | Probabilistic (weighted: name 0.30, birth date 0.25, gender 0.10, address 0.10, identifier 0.10, tax ID 0.10, document 0.05) + deterministic short-circuits; serves the REST `match` / dedup endpoints today | [service AGENTS/matching.md](../person-service-rust-crate/AGENTS/matching.md) — full weights, rules, Soundex table |
| Canonical matcher (embedded) | `person-matcher` crate, re-exported as `matcher_lib`, reached via `adapter.rs` | Pure-library `MatchingEngine` + `MatchConfig` (`strict` / `default` / `lenient`); 42 national-identifier schemes, passport books, nickname tables, NFKD normalisation, E.164 phone (39 jurisdictions) | [matcher AGENTS/matching-algorithm.md](../person-matcher-rust-crate/AGENTS/matching-algorithm.md), [normalization.md](../person-matcher-rust-crate/AGENTS/normalization.md), [matcher spec §12](../person-matcher-rust-crate/spec/12-algorithm-specifications.md) |

Which is authoritative long-term is an open question — entity
[spec §16 EOQ-1](../spec/16-open-questions.md). Until resolved, keep
both consistent through the bridge tests.

## Shared semantics (entity contract)

- Score in `[0.00, 1.00]` with per-component breakdown — no black
  boxes, end to end (entity [spec NFR-10](../spec/07-non-functional-requirements.md)).
- Deterministic short-circuits beat probabilistic scores (tax ID /
  identifier / document exact → pinned high score).
- National identifiers are scheme-local; never cross-match schemes.
- Quality buckets are threshold-driven and configurable
  (service: Definite ≥ 0.95 / Probable ≥ 0.85 / Possible ≥ 0.50;
  matcher: `confidence` High / Medium / Low).

## The bridge

Service record → `to_matcher_person()` → `MatchingEngine::match_persons`
→ `MatchResult { score, is_match, confidence, breakdown }`. Routing
rules: entity [spec §5.3](../spec/05-domain-model.md). Pinned by the
14-test bridge suite (`cargo test --test duplicate_detection`).

## Editing rules

- Changing **weights / thresholds / rules** of the in-service stack →
  service spec §6.2 + service `AGENTS/matching.md` + unit tests.
- Changing **the canonical algorithm** → matcher spec (§12/§13) +
  matcher tests; never change default weights without its CHANGELOG.
- Changing **the adapter** → entity spec §5.3 + bridge test, same PR.
- Adding a **new identifier scheme** end-to-end: matcher parser
  (`src/identifiers.rs`) → adapter routing → bridge test → entity
  spec §13 E-8 table.
