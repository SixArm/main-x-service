# Matching — Thing Entity orientation

There are **two matching layers** in this entity. Know which one you
are touching before you edit weights or thresholds.

## Layer 1 — canonical engine (thing-matcher)

The authoritative algorithm. Embedded by the service, usable
standalone.

- **Deterministic** ([matcher spec §5.1](../thing-matcher-rust-crate/spec/05-matching-engine.md)):
  shared `(property_id, value)` identifier pair, OR shared normalised
  `sameAs` URL, OR equal normalised canonical `url`.
- **Probabilistic** (§5.2–§5.10): renormalised weighted sum over up
  to 10 components — name, description, disambiguating description,
  identifiers, url, same_as, image, main_entity_of_page,
  additional_types — plus an optional Soundex bonus. Missing fields
  skip (never penalise).
- **Presets**: `default` (threshold 0.80), `strict` (0.95 +
  `strict_mode` requiring a deterministic match), `lenient` (0.65 +
  phonetic on). Default weights: name 0.30, identifiers 0.25,
  same_as 0.15, description 0.10, the rest small — see
  [matcher spec §3.4](../thing-matcher-rust-crate/spec/03-data-model.md).
- **Confidence bands** (fixed): High ≥ 0.90, Medium ≥ 0.75, else Low.
- Tuning guidance: [matcher agents/matching-algorithm.md](../thing-matcher-rust-crate/agents/matching-algorithm.md).

## Layer 2 — in-service scorer (thing-service)

A 5-component summary scorer used by the service's match / dedup
handlers.

- Weights: name 0.40 (Jaro-Winkler), identifier 0.30 (exact pair),
  description 0.10, URL 0.10, same_as 0.10; +0.05 Soundex bonus
  below 0.95.
- Deterministic short-circuit on DOI / ISBN / ISSN / GTIN / MPN /
  SerialNumber / UUID.
- Quality buckets (configurable): Certain ≥ 0.95, Probable ≥ 0.80,
  Possible ≥ 0.60, Unlikely.
- Reference: [service agents/matching.md](../thing-service-with-loco/agents/matching.md).

## How the layers connect

The service re-exports the matcher as `matcher_lib` and bridges via
[`src/matching/adapter.rs`](../thing-service-with-loco/src/matching/adapter.rs)
(`to_matcher_thing`). The contract is entity spec
[§5.3](../spec/05-domain-model.md); enforcement is the 15 bridge
tests in
[`tests/duplicate_detection.rs`](../thing-service-with-loco/tests/duplicate_detection.rs).

Open questions an agent should not pre-empt: whether the in-service
scorer is retired in favour of the engine (entity spec §16 OQ-1), and
which confidence vocabulary is API-facing (OQ-2 / T-8).

The two confidence vocabularies (Certain/Probable/Possible/Unlikely
vs High/Medium/Low) classify the same score with interleaving cut
points, so they have **no 1:1 label mapping** — see the score-range
overlay table in
[service `agents/matching.md`](../thing-service-with-loco/agents/matching.md)
("Relationship to the embedded matcher's confidence bands").

## Rules of thumb

- Changing **what a component scores** → matcher crate (spec-first).
- Changing **which service fields feed the engine** → adapter +
  entity spec §5.3 + a bridge test, one PR.
- Changing **thresholds the API reports** → service spec §6.2; check
  entity §13 T-8 first.
- Never let a match response drop the per-field breakdown — it is
  the audit trail for the score (ISO/IEC 42001 posture, entity §12).
