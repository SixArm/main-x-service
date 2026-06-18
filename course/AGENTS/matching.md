# Matching — Course Entity

Orientation only. The **canonical algorithm lives in the matcher
crate**; the service embeds it; the front-end renders its breakdowns.

- Algorithm + derivations:
  [`course-matcher/AGENTS/matching-algorithm.md`](../course-matcher-rust-crate/AGENTS/matching-algorithm.md)
  and matcher [spec §5–§18](../course-matcher-rust-crate/spec/index.md)
- Service-side adapter view:
  [`course-service/AGENTS/matching.md`](../course-service-with-loco/AGENTS/matching.md)
- Normalisation rules:
  [`course-matcher/AGENTS/normalization.md`](../course-matcher-rust-crate/AGENTS/normalization.md)

## Shape of the algorithm

1. **Deterministic short-circuits** (any hit → score 1.0,
   `deterministic_match = true`):
   - R-0: shared deterministic identifier — DOI / Wikidata / LOM /
     OER / URI / UUID ([matcher spec §15](../course-matcher-rust-crate/spec/15-identifier-short-circuits.md)).
   - R-1: same `provider_id` + same normalised `course_code`.
   - R-2: `same_as` URL overlap.
2. **Probabilistic weighted average** over present components
   (renormalised; absent components don't penalise):

| Component | Weight | Algorithm |
|---|---|---|
| Name | 0.35 | Jaro-Winkler + Levenshtein (max), +0.05 Soundex bonus capped at 0.95 |
| Course code | 0.15 | Exact, **only within the same provider** — `None` across providers |
| Provider | 0.15 | Exact `provider_id` or provider-name Jaro-Winkler |
| Educational level | 0.10 | Exact 1.0 / one level off in family 0.5 / else 0.0 |
| Keywords | 0.10 | Jaccard on folded sets |
| Teaches | 0.15 | Jaccard on folded sets |

3. **Confidence**: Definite ≥ 0.95, Probable ≥ threshold (default
   0.85), Possible ≥ 0.50, Unlikely below. Presets:
   `strict()` 0.95, `lenient()` 0.70.

## How the service uses it

Blocking via Tantivy (`name` fuzzy and/or `(provider_id,
course_code)`), then `CourseMatcher` →
`course_matcher::MatchingEngine::match_courses` per candidate, then
ranked `MatchResult[]` back through `/api/courses/match`,
`check-duplicates`, create-time `409`, and batch `deduplicate`
(auto-merge above `auto_merge_threshold`, rest to the review queue).

## Rules an agent must not break

- Never score `course_code` across providers (CS101 ≠ CS101).
- Never add a deterministic scheme without a matcher spec edit + a
  service bridge test in the same change (entity FR-20).
- Weights / thresholds change only via matcher
  [spec §7](../course-matcher-rust-crate/spec/07-configuration.md)
  + CHANGELOG.
- The matcher stays pure: no IO, no async, no clocks, no RNG.
