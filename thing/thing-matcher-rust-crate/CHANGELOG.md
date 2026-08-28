# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [index.md](./index.md) (documentation map), [spec.md](./spec/index.md) (authoritative behaviour — each entry below corresponds to a section / FR / task in the spec), [README.md](./README.md) (user-facing overview).

## [Unreleased]

## [0.7.0] - 2026-08-28

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

### Added — cargo-fuzz harness (SEC-I2)

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate with
  three coverage-guided libFuzzer targets, adopting the person-matcher
  reference scaffolding: `match_things` (deserialize a JSON `[thing_a, thing_b]`
  tuple → `MatchingEngine::match_things`; finite score in `[0,1]`, both orders),
  `normalizer` (the pure `Normalizer` helpers — name / free text / URL / phonetic — over arbitrary
  UTF-8, never-panic), and `scorer` (the pure `Scorer` similarities;
  finite in `[0,1]`). Run on nightly: `cargo +nightly fuzz run <target>`
  (see `fuzz/README.md`). The `fuzz/` crate is standalone (not a workspace
  member), so it never affects the crate’s normal stable build/test/clippy.
  Verified: `cargo +nightly fuzz build` compiles all three targets and
  short time-boxed campaigns run clean (millions of execs, no panics).

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

### Added — relationships and tags as weighted supporting components (T-PRO-H7)

- **`RelationshipRef` / `RelationKind` (§3.3.1, §5.9.1, §6.6).** New public
  types `RelationKind` (`#[non_exhaustive]`; `Contains` / `ContainedIn`
  containment inverses, `SuperPart` / `SubPart` part-of inverses per
  schema.org `hasPart` / `isPartOf`) and
  `RelationshipRef { relation: RelationKind, thing_id: String }`,
  re-exported from the crate root. `RelationshipRef::new` trims `thing_id`
  and rejects an empty result. `Thing` gains
  `relationships: Vec<RelationshipRef>` (default empty,
  `#[serde(default)]`), with `ThingBuilder::add_relationship` /
  `::relationships` setters. Scored as typed-set Jaccard over `(relation,
  thing_id)` pairs (`|A ∩ B| / |A ∪ B|`) into the new
  `MatchBreakdown::relationships_score` (`#[serde(default)]`), `None`
  when either side's list is empty. New `MatchConfig::relationships_weight`
  (default `0.05`) joins the supporting-signal cluster in the
  weighted-average renormalisation.
- **`tags` (§3.1, §5.9.2, §6.8).** `Thing` gains `tags: Vec<String>`
  (default empty, `#[serde(default)]`), with `ThingBuilder::add_tag` /
  `::tags` setters. Tags are stored verbatim and compared
  case-insensitively at scoring time (consistent with the crate's
  normalise-at-match-time convention for names / URLs) as set Jaccard
  (`|A ∩ B| / |A ∪ B|`) into the new `MatchBreakdown::tags_score`
  (`#[serde(default)]`), `None` when either side's list is empty. New
  `MatchConfig::tags_weight` (default `0.05`) joins the supporting-signal
  cluster alongside `relationships_weight`. `MatchBreakdown` now carries
  **12** score fields.
- Both fields are purely additive and **not** identifying on their own
  (`Thing::validate` is unchanged) and **not** consulted by
  `deterministic_match`. Neither field participates in the weighted
  average unless populated on *both* sides, so existing callers that
  never set `relationships`/`tags` see byte-identical scores before and
  after this release — the two new default weights simply never enter
  the denominator for them.
- `agents/matching-algorithm.md`'s "Component Scoring At-a-Glance" and
  `None`-when tables gain Relationships / Tags rows.
  `spec/03-data-model.md` §3.1 / §3.3.1 / §3.4 / §3.7,
  `spec/05-matching-engine.md` §5.9.1 / §5.9.2 / §5.10, and
  `spec/06-per-field-scoring-algorithms.md` §6.6 / §6.8 move from
  "not yet implemented, spec-only" to current behaviour; OQ-E
  (`spec/10-open-questions.md`) is resolved.
- Entity domain model (`thing/spec/05-domain-model.md`) flipped: tags is
  now a routed match signal (§5.3 routes `tags` → matcher `tags`, scored
  by set Jaccard, weighted `tags_weight`; removed from the lossy-drop
  list), no longer a registry-only attribute.
- Minor version bump (pre-1.0): per `agents/release.md`, a default-weight
  addition that changes computed behaviour once the new fields are
  populated — plus two new public types and four new public struct
  fields — is a minor bump, not a patch.

## [0.6.1] — 2026-06-15

### Changed — documentation harmonisation pass

- Reconciled the doc set with the implementation after a copy-adapt from
  the sibling `place-matcher` / `person-matcher` crates had left foreign,
  wrong-domain material behind. No behaviour change; code unchanged.
- Fixed the `Combined` name-similarity blend in three docs that still
  cited the old `0.6 × JaroWinkler + 0.4 × Levenshtein` weighting. The
  implementation (`Scorer::combined_similarity`) is and remains
  `0.7 × JaroWinkler + 0.3 × Levenshtein`; spec §5.6, spec §6.1, and
  `agents/matching-algorithm.md` now match the code and
  `AGENTS.md`.
- Fixed the string-similarity primitive name in spec §5.6: the method is
  `Scorer::jaccard_set_similarity`, not `jaccard_similarity`.
- Fixed `agents/testing.md`: the property-test description referenced
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
- Documented in `agents/testing.md` and `index.md` (Common Tasks table)
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
