# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec.md) — single source of truth (numbered §1–§25; live work queue in §23); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **`MatchingEngine::match_one_to_many`** (T-10). Returns
  `Vec<MatchResult>` in the same order as the candidate slice (no
  rank, no filter). Mirrors `person_matcher::MatchingEngine::match_one_to_many`
  so cross-family callers share one signature; existing `rank`
  remains for the sorted-by-score variant and `find_matches` for the
  filtered + sorted view.
- **`IdentifierScheme` doc polish** (T-9). Every variant now carries
  a one-line example and is tagged **deterministic** vs
  **provider-scoped** so consumers can read off which schemes trigger
  the R-0 short-circuit at a glance.
- **Cross-link**: T-7 (service-side adapter + bridge test) and T-8
  (criterion benches) are now ticked in `spec.md §23` with pointers
  into the embedding `course-service` crate where the work landed.
- **Soundex phonetic bonus** (T-6). `src/phonetic.rs` ships the
  classic American Soundex encoder (first letter + 3 digits; H/W/Y
  ignored except as initial; consonant-run collapse). `name_score`
  in `src/matcher.rs` applies a `+0.05` bonus when both course
  names produce the same Soundex code and the underlying
  Jaro-Winkler is `< 0.95`. The result is capped at `0.95` so a
  phonetic hit nudges Medium-band scores up but never single-
  handedly mints a High-confidence classification.
- Unit tests: 4 in `phonetic::tests` (Robert/Rupert, classic
  examples, empty input, pad short codes, homophone-pair helper)
  plus 3 in `matcher::tests` (bonus fires on homophones, no-fire on
  unrelated names, cap respected on near-clones).
- 19/19 lib tests pass (was 12).

## [0.1.0] — 2026-06-04

Initial release. Library-only crate for pairwise course matching
modelled on schema.org/Course.

### Added

- **Public surface.** `Course`, `CourseIdentifier`, `IdentifierScheme`,
  `EducationalLevel`, `LearningResourceType`, `MatchingEngine`,
  `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`.
- **Algorithm.** Deterministic short-circuits on (a) deterministic
  identifier scheme matches (DOI / Wikidata / LOM / OER / URI /
  UUID), (b) same-`provider_id` + normalised `course_code` match,
  (c) `same_as` URL overlap. Probabilistic weighted-average over
  name (Jaro-Winkler with alternate-name fallbacks, 0.35), provider-
  scoped course code (0.15), provider (0.15), educational level
  with adjacent-ladder partial credit (0.10), keywords Jaccard
  (0.10), teaches Jaccard (0.15). Renormalised over the present
  components, so identical-on-the-known-fields records score 1.0
  rather than 0.65.
- **Normalisation.** `fold` (trim + NFKC + lowercase),
  `course_code` (strip whitespace + uppercase), `fold_set` (sort +
  dedupe). Total functions, no panics.
- **Configuration.** `MatchConfig` with default weights summing to
  1.0 and `threshold = 0.85`. Two presets: `strict()` (0.95) and
  `lenient()` (0.70).
- **Confidence bands.** `Confidence::High` ≥ 0.95, `Medium` ≥ 0.70,
  `Low` otherwise.
- **Unit tests.** Cover identical-clone score, DOI short-circuit,
  same-provider course-code short-circuit, unrelated-records low
  score, typo-tolerant name match, and rank ordering. Plus
  normalise + scoring helper tests.

### Documentation

- `spec.md` §1–§25 — research basis, full per-component derivation,
  configuration table, normalisation rules, quality goals,
  consumption story, anti-patterns, change-control discipline.
- `AGENTS.md` + `AGENTS/{index, spec-driven-development,
  matching-algorithm, normalization, testing}.md`.
- `README.md` with quick-start, algorithm summary, public surface,
  build / test commands.

### Pending (next iterations)

Tracked in `spec.md §23`:

- T-6: Soundex phonetic bonus on the `name` component.
- T-7: Service-side adapter + bridge test in
  [`course-service-rust-crate/tests/duplicate_detection.rs`](../course-service-rust-crate/tests/duplicate_detection.rs).
- T-8: Criterion benchmarks (name match, full match, rank-of-100).
- T-10: Expose `MatchingEngine::match_one_to_many` for parity with
  sibling matcher crates.

### Cross-references

- Sibling matcher crates for the algorithmic family:
  [`person-matcher-rust-crate`](../person-matcher-rust-crate/),
  [`event-matcher-rust-crate`](../event-matcher-rust-crate/),
  [`place-matcher-rust-crate`](../place-matcher-rust-crate/),
  [`thing-matcher-rust-crate`](../thing-matcher-rust-crate/),
  [`worker-matcher-rust-crate`](../worker-matcher-rust-crate/).
- Embedding service:
  [`course-service-rust-crate`](../course-service-rust-crate/).
