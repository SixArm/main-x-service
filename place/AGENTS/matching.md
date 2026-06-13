# Matching — Place entity

Two scorers exist; know which one you're touching.

| Scorer | Where | Role |
|---|---|---|
| **Canonical matcher** (`place-matcher`) | embedded by the service as `matcher_lib`, reached via `adapter::to_matcher_place` | The reference algorithm: deterministic + probabilistic, weight renormalisation, config presets. Spec is authoritative ([matcher spec §5–§7](../place-matcher-rust-crate/spec/05-matching-pipeline.md)) |
| **In-service scorer** (`matching::scoring::compute_match`) | `place-service-rust-crate/src/matching/` | The service's own weighted scorer over the service model ([service AGENTS/matching.md](../place-service-rust-crate/AGENTS/matching.md)) |

## Canonical matcher (place-matcher)

- **Deterministic rule:** shared `(scheme, value)` pair across
  `place_ids`, OR identical normalised `name` + identical normalised
  `address.postcode`.
- **Probabilistic pipeline:** weighted, weight-renormalised sum across
  name / coordinates / address / category / country_code / place_ids /
  phone / email; missing fields skip; optional Soundex bonus behind a
  phonetic gate.
- **Geo:** Haversine + Gaussian decay `exp(-(d/s)^2)`, default scale
  `50 m`.
- **Thresholds:** default `0.80`; `strict` `0.95`; `lenient` `0.65`
  (`MatchConfig` presets, [spec §7](../place-matcher-rust-crate/spec/07-configuration.md)).
- **Guarantees:** pure, deterministic, explainable (`MatchBreakdown`
  per field), no `unsafe`, no IO.
- Algorithm guide:
  [matcher AGENTS/matching-algorithm.md](../place-matcher-rust-crate/AGENTS/matching-algorithm.md);
  normalisation:
  [matcher AGENTS/normalization.md](../place-matcher-rust-crate/AGENTS/normalization.md).

## In-service scorer (place-service)

- **Weights (sum 1.0):** name 0.35 (Jaro-Winkler + Soundex bonus) ·
  geo 0.25 (Haversine sigmoid decay, reference 1 km) · address 0.20
  (weighted fields) · place type 0.10 · identifier 0.10.
- **Deterministic short-circuit:** exact GLN match → 1.0.
- **Confidence:** Certain ≥ 0.95 · Probable ≥ 0.80 · Possible ≥ 0.60 ·
  Unlikely below.
- Full reference:
  [service AGENTS/matching.md](../place-service-rust-crate/AGENTS/matching.md).

Note the two scorers differ deliberately (geo decay shape and
reference scale, component sets, presets); the matcher crate is the
canonical algorithm the in-service scorer does not duplicate (service
[spec §6.2](../place-service-rust-crate/spec/06-functional-requirements.md)).

## The seam

Changing match behaviour that crosses the service↔matcher boundary
(adapter routing, surfaced breakdown fields) is an **entity-contract
change**: update entity [spec §5.3](../spec/05-domain-model.md) and
add a bridge test in
[`tests/duplicate_detection.rs`](../place-service-rust-crate/tests/duplicate_detection.rs).
Changing weights, thresholds, or normalisation **inside** the matcher
is matcher-spec territory (its §5–§7 + `CHANGELOG.md`, same PR).

## Where matching is consumed

- Service: real-time duplicate detection on create (`409`), explicit
  check, batch deduplicate, merge candidate scoring.
- Front-end: `/places/match` (score a hypothetical) and `/places/new`
  (renders 409 candidates with their breakdowns).
