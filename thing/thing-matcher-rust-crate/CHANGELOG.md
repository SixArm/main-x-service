# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [index.md](./index.md) (documentation map), [spec.md](./spec/index.md) (authoritative behaviour — each entry below corresponds to a section / FR / task in the spec), [README.md](./README.md) (user-facing overview).

## [Unreleased]

### Security

- **SEC-M2 (High): empty-URL deterministic false positive.** The two
  URL/`sameAs` deterministic short-circuits in `src/matcher.rs`
  (`same_canonical_url`, `shares_same_as`) keyed on a normalised URL string
  with no empty guard, so a value that normalises to empty (whitespace, or a
  bare `#fragment`) made `"" == ""` fire, spuriously pinning two *different*
  things to a 1.0 identity match. Both short-circuits now ignore values whose
  normalised form is empty: `same_canonical_url` returns `false` when the
  normalised `url` is empty, and `shares_same_as` skips empty normalised
  `sameAs` entries so a shared degenerate value is not evidence. Behaviour on
  real URLs is unchanged; no weights or thresholds altered. (Same bug class as
  person-matcher's `passport_books_share_pair` fix.)

### Added — tags (operator labels) as a supporting match signal

- Spec-only addition (§3.1, §3.4, §3.7, §5.9.2, §5.10, §6.8): a
  `tags: Vec<String>` field on `Thing` (defaults to empty).
- New scoring component `tags_score`: plain set **Jaccard** over the
  case-insensitively normalised tag sets (`None` when either side empty);
  added to `MatchBreakdown` (now **12** score fields).
- New config weight `tags_weight` (default `0.05`), added to the
  renormalised weighted sum. A **supporting** signal, never identifying on
  its own.
- Entity domain model (`thing/spec/05-domain-model.md`) flipped: tags is now
  a routed match signal (§5.3 routes `tags` → matcher `tags`, scored by set
  Jaccard, weighted `tags_weight`; removed from the lossy-drop list), no
  longer a registry-only attribute.
- **Code follow-up (not yet implemented):** add the `tags` field to
  `crate::models`, the `tags_score` field + case-insensitive set-Jaccard
  scorer, the `tags_weight` config field, and tests pinning the Jaccard
  behaviour and the renormalisation.

### Added — relationships (typed thing-to-thing references)

- Spec-only addition (§3.1, §3.3.1, §3.4, §3.7, §5.9.1, §5.10, §6.6):
  a `relationships: Vec<RelationshipRef>` field on `Thing`, a
  `RelationshipRef { relation: RelationKind, thing_id: String }` type, and
  a `#[non_exhaustive]` `RelationKind` enum (`Contains` / `ContainedIn`
  containment inverses; `SuperPart` / `SubPart` part-of inverses per
  schema.org `hasPart` / `isPartOf`; extensible).
- New scoring component `relationships_score`: typed-set **Jaccard** over
  the `(relation, thing_id)` pairs (`None` when either side empty); added
  to `MatchBreakdown` (now **11** score fields).
- New config weight `relationships_weight` (default `0.05`), added to the
  renormalised weighted sum. A **supporting** signal, never identifying on
  its own.
- **Code follow-up (not yet implemented):** add the `relationships` field +
  `RelationshipRef` / `RelationKind` types to `crate::models`, the
  `relationships_score` field + Jaccard scorer, the `relationships_weight`
  config field, and tests pinning the typed-set Jaccard behaviour and the
  renormalisation.

## [0.6.1] — 2026-06-15

### Changed — documentation harmonisation pass

- Reconciled the doc set with the implementation after a copy-adapt from
  the sibling `place-matcher` / `person-matcher` crates had left foreign,
  wrong-domain material behind. No behaviour change; code unchanged.
- Fixed the `Combined` name-similarity blend in three docs that still
  cited the old `0.6 × JaroWinkler + 0.4 × Levenshtein` weighting. The
  implementation (`Scorer::combined_similarity`) is and remains
  `0.7 × JaroWinkler + 0.3 × Levenshtein`; spec §5.6, spec §6.1, and
  `AGENTS/matching-algorithm.md` now match the code and
  `AGENTS.md`.
- Fixed the string-similarity primitive name in spec §5.6: the method is
  `Scorer::jaccard_set_similarity`, not `jaccard_similarity`.
- Fixed `AGENTS/testing.md`: the property-test description referenced
  `match_places`; the crate's method is `match_things`.
- Rewrote `CHANGELOG.md` (this file) for the `thing-matcher` domain —
  the prior file documented a `Place` / `PlaceCategory` / `PlaceId`
  model, geographic fields, dozens of national-identifier parsers, and a
  geographic `MatchConfig` weight table, none of which exist in this
  crate. The genuine 0.6.0 history (below) is retained.
- Repurposed `index.md` as a documentation map (navigation + Common Tasks
  table) rather than a near-verbatim duplicate of `README.md`.

### Added — tests pinning previously-unguarded behaviour

- `tests/integration_tests.rs`: a test asserting the exact `Combined`
  blended value for a known JW/Lev pair, locking the `0.7/0.3` weighting
  so it can no longer drift to `0.6/0.4` undetected.
- `tests/integration_tests.rs`: a test asserting the phonetic bonus
  actually raises the overall `score` for a Soundex-equal name pair
  versus the same pair with `use_phonetic_matching = false`.
- `tests/adapter_contract.rs`: pins `Scorer::optional_field_score` as a
  re-exported public symbol so its accidental removal breaks CI.

## [0.6.0]

### Changed — `chrono` eliminated

- `chrono` (an unused manifest dependency flagged for removal) is gone;
  `thing-matcher` carries no date dependency. No functional change.

### Added — adapter-contract test (CI guardrail for the public API)

- New `tests/adapter_contract.rs`. Pins every public symbol downstream
  service adapters depend on: `Thing` / `ThingBuilder` / `Identifier`
  constructors, `MatchingEngine::default_config` / `::new` /
  `match_things` / `deterministic_match` / `match_one_to_many` /
  `rank_one_to_many`, the `MatchResult` field shape (`score`,
  `is_match`, `confidence`, `breakdown`), the `MatchBreakdown`
  per-component fields the adapter inspects, `MatchConfig::strict` /
  `::default` / `::lenient` forming a monotonic threshold ladder,
  `Confidence::{High, Medium, Low}`, and `MatchResult` JSON round-trip.
- A rename or removal of any of the above breaks this test, failing the
  matcher's own CI **before** publish — making cross-crate breakage
  deliberate.
- Documented in `AGENTS/testing.md` and `index.md` (Common Tasks table)
  and cross-referenced from `spec.md` §9.

### Added — spec/code drift CI check

- `.github/workflows/spec-drift.yml` runs on every pull request to
  `main`. It fetches full git history and invokes
  `scripts/spec-drift-check.sh` to enforce that any `src/matcher.rs`
  change is accompanied by a `spec.md` update in the same PR.
- `scripts/spec-drift-check.sh` (POSIX bash, no extra deps) resolves the
  base ref, computes the changed-file set, applies the watched-file
  pattern (`^src/matcher\.rs$`), and consults `.spec-allow` for
  path-pattern exceptions. Exits gracefully if the base ref cannot be
  resolved (e.g. fork CI) so it never produces spurious failures.
- `.spec-allow` (extended-regex path patterns; blank / `#`-prefixed lines
  ignored) ships empty so the discipline starts strict.
- `.github/pull_request_template.md` references the spec-drift check and
  prompts contributors for spec impact, allowlist justification, and a
  CHANGELOG entry.
- The script is runnable locally pre-push:
  `bash scripts/spec-drift-check.sh main HEAD`.

### Fixed — `normalize_url` idempotency on whitespace before a fragment

- `Normalizer::normalize_url` was not idempotent when a `#fragment` was
  preceded by whitespace (e.g. `"http://h/p \u{2000}#x"`): dropping the
  fragment exposed trailing whitespace that the initial trim could not
  reach, so a second pass produced a different string — violating the
  §4 idempotency contract and intermittently failing the
  `normalize_url_is_idempotent` property test on Unicode-whitespace
  seeds. The pre-fragment slice is now re-trimmed. Added a unit
  regression (`normalize_url_retrims_after_fragment_removal`) and two
  seeds to the idempotency unit test. Spec §4 unchanged (the fix restores
  documented behaviour).

## Domain model

`thing-matcher` matches [`schema.org/Thing`](https://schema.org/Thing)
records — books, articles, products, landmarks, software, organisations,
and any other entity describable with the schema.org Thing vocabulary.
The model is `Thing` with `name`, `alternate_names`, `description`,
`disambiguating_description`, `identifiers`, `url`, `image`, `same_as`,
`main_entity_of_page`, `additional_types`, `subject_of`, `owner`, and
`local_id`. Deterministic match fires on a shared `(property_id, value)`
identifier pair, a shared `sameAs` URL, or an equal canonical `url`. The
probabilistic path is a weight-renormalised blend (default name weight
`0.30`, identifiers `0.25`, sameAs `0.15`, …) with a default
`match_threshold` of `0.80`. See [`spec.md`](./spec/index.md) for the
authoritative §1–§13 specification.
