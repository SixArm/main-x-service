# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§25; live work queue in §23); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Changed
- MSRV raised to Rust 1.96 (N-2 policy tightened from N-3; see spec/rust-msrv-n-minus-2/index.md).

## [0.7.0] - 2026-08-28

### Added — relationships and tags as weighted matcher components (T-11 / T-12)

- **`RelationshipRef` / `RelationKind` (T-11).** New public types
  `RelationKind` (`SimilarTo`, `HigherLevelThan`, `LowerLevelThan` —
  `SimilarTo` symmetric, the other two inverses, extensible) and
  `RelationshipRef { relation: RelationKind, course_id: String }`,
  re-exported from the crate root. `RelationshipRef::new` trims
  `course_id` and rejects an empty result. `Course` gains
  `relationships: Vec<RelationshipRef>` (default empty,
  `#[serde(default)]`). Scored as typed-set Jaccard over `(relation,
  course_id)` pairs (`|A ∩ B| / |A ∪ B|`) into the new
  `MatchBreakdown::relationships_score`, `None` when either side's list
  is empty. New `MatchConfig::relationships_weight` (default `0.05`)
  joins the supporting-signal cluster in the weighted-average
  renormalisation.
- **`tags` (T-12).** `Course` gains `tags: Vec<String>` (default
  empty, `#[serde(default)]`). Tags are stored verbatim and compared
  case-insensitively at scoring time via `normalize::fold_set` — the
  same folding `keywords`/`teaches` use — as set Jaccard (`|A ∩ B| /
  |A ∪ B|`) into the new `MatchBreakdown::tags_score`. Unlike
  `keywords`/`teaches` (`Some(0.0)` when only one side is populated),
  `tags_score` returns `None` whenever either side has no *usable*
  tags — empty `Vec`, or every entry folds away to blank — mirroring
  the sibling matcher crates (`worker-matcher`/`person-matcher`/
  `event-matcher`) rather than the keywords/teaches convention. New
  `MatchConfig::tags_weight` (default `0.05`) joins the supporting-
  signal cluster alongside `relationships_weight`. Completes the spec
  addition recorded further down this file under "Tags match
  component (spec)".
- Both fields are purely additive and **not** identifying on their own
  and **not** consulted by the deterministic short-circuit
  (`deterministic_match`). Neither field participates in the weighted
  average unless populated on *both* sides, so existing callers that
  never set `relationships`/`tags` see byte-identical scores before and
  after this release — the two new default weights simply never enter
  the denominator for them.
- `relationships_weight`/`tags_weight` are deliberately **excluded**
  from `MatchConfig::weight_total()` (test-only) and from the "six
  identifying weights sum to 1.00" invariant (§7) — they sit in a
  separate low-weight supporting-signal cluster.
- `agents/matching-algorithm.md`'s probabilistic-components table and
  algorithm trace gain Relationships / Tags rows. `spec/05-algorithm-
  overview.md` §5.1/§5.2, `spec/06-domain-model.md` §6/§6.1/§6.2,
  `spec/07-configuration.md`, and `spec/13a-tags.md` move from
  "planned, not yet implemented" to current behaviour; `spec/23-
  tasks.md` T-11/T-12 marked done.
- Minor version bump (pre-1.0): per the family's release convention
  (see `worker-matcher` 0.7.0), a default-weight addition that changes
  computed behaviour once the new fields are populated — plus two new
  public types and two new public struct fields — is a minor bump, not
  a patch.

### Added — Criterion benchmarks

- `benches/match_pair.rs`, matching the harness the other matcher crates
  carry. Four groups over the paths a downstream integrator actually
  exercises: single-pair scoring in three regimes (identical clone,
  fuzzy near-match through the full pipeline, unrelated pair), the
  deterministic short-circuits (a shared DOI, and a shared provider-scoped course code), `rank` at 10 / 100 / 1000
  candidates with `Throughput::Elements` set so per-candidate cost and
  any super-linear scaling are visible directly, and the same fuzzy pair
  under each shipped config preset.
- Fixtures are built from `Course::new` and derived deterministically from
  an index, so a candidate list of any size is reproducible without
  randomness.

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

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate adopting
  the person-matcher reference scaffolding, with two coverage-guided
  libFuzzer targets: `match_courses` (deserialize a JSON `[course_a, course_b]` tuple →
  `MatchingEngine::match_courses`; finite score in `[0,1]`, both orders) and
  `normalize` (the pure `normalize` free functions — fold / course code / fold-set — over arbitrary
  UTF-8, never-panic). Two targets rather than the reference three because
  this crate exposes its similarity primitives only through the engine (the
  `scoring` module publishes no string-similarity functions). Run on nightly:
  `cargo +nightly fuzz run <target>` (see `fuzz/README.md`). The `fuzz/` crate
  is standalone (not a workspace member), so it never affects the crate’s
  normal stable build/test/clippy. Verified: `cargo +nightly fuzz build`
  compiles both targets and short campaigns run clean (millions of execs, no
  panics).

### Fixed

- **SEC-M6: `provider_score` is now symmetric.** The structured-id branch
  guarded only side `a`'s `provider_id` non-empty, so a degenerate
  one-sided empty id (`Some("")`) scored `0.0` in one direction but fell
  through to the name fallback (`None`) in the other — a
  direction-dependent result surfaced by the new symmetry property test.
  It now requires **both** sides' `provider_id` non-empty (mirroring
  `course_code_score`); a blank id is treated as "no structured id" in
  both directions. Unit test `provider_score_is_symmetric_with_one_sided_empty_id`;
  the `matching_is_symmetric` property now runs over blank-provider cases.

### Testing

- **SEC-M6: property-based tests.** Added `tests/proptests.rs`
  (`proptest = "1.11"`, dev-dependency only) proving the matcher is
  robust on adversarial / arbitrary input. Invariants pinned over the
  public surface: the engine and every pure helper
  (`normalize::fold` / `course_code` / `fold_set`, `phonetic::soundex` /
  `same`) never panic on arbitrary UTF-8; `MatchResult::score` is always
  finite and in `[0.0, 1.0]` (never NaN); matching is symmetric
  (`match(a,b) == match(b,a)` on score / `is_match` / confidence); an
  identical clone of a well-formed course self-matches above threshold;
  and `soundex` output is `None` or a well-formed `[A-Z][0-9]{3}` code.
  Tests only — no library behaviour, weights, or thresholds changed.

### Security

- **SEC-M2: provider-scoped deterministic rule (R-1) no longer
  short-circuits on an empty course code.** The R-1 rule in
  `deterministic_match` (`src/matcher.rs`) previously guarded that
  `provider_id` was non-empty but not the course code itself, so two
  DIFFERENT courses sharing a provider whose codes both normalise to
  `""` (e.g. `"-"` or `"  "`) would falsely pin to a `1.0` identity
  match. R-1 now additionally requires the normalised code to be
  non-empty (both `provider_id` and the normalised `course_code` must be
  present). Same bug class as the person-matcher `passport_books_share_pair`
  fix. Added a regression test; no weights/thresholds changed.

### Fixed

- Formatting drift in `src/matcher.rs` (two spots not rustfmt-formatted);
  `cargo fmt --check` is clean again. No behaviour change.

### Added

- **Tags match component (spec).** `tags` is now a routed, weighted
  **supporting** match signal: plain set Jaccard over the
  case-insensitively normalised tag sets (identical to `keywords`;
  `None` when either side empty), weighted `tags_weight` (default 0.05,
  supporting-signal cluster), renormalised over the present components.
  Specced in §5 / §5.2 / §6 / §6.2 / §7 / §13a; code follow-up tracked
  as §23 T-12 (add `Course::tags`, `tags_score`, `MatchConfig::tags_weight`,
  `MatchBreakdown::tags_score`). The course-entity domain model (§5.1 /
  §5.3 / §5.5) flips tags from registry-only to a routed match signal.
- **Doc harmonisation pass.** Pinned the deliberately-unscored fields
  with explicit tests (`learning_resource_type` + `in_language` carry
  no scoring weight; off-ladder `EducationalLevel` variants
  `Vocational` / `ProfessionalDevelopment` / `Custom` earn no adjacency
  credit and score `1.0` only when identical, `0.0` otherwise) — 4 new
  unit tests in `matcher` and 1 new public-surface integration test, so
  the suite is now 76 unit + 16 integration. Added a worked
  probabilistic-partial-match example (with per-component breakdown) to
  `index.md`. Fixed the documented `MatchResult` shape in `AGENTS.md`
  (now includes `is_match`) and the file-layout block (`spec/` directory
  with §1–§25 section files, not a single `spec.md`). Corrected stale
  test counts (was "21 unit tests") across `index.md` / `README.md` and
  `agents/testing.md`; recorded the unscored-field invariant in
  `spec.md §24`.
- **Expanded test coverage** (72 embedded unit tests, up from 21,
  plus a new 15-test integration suite). Filled the
  previously-undocumented gaps: `src/course.rs` now pins
  `Course::new` defaults, `IdentifierScheme::is_deterministic` for
  every variant, and serde round-trips; `src/config.rs` pins the
  default weights summing to 1.0 and the `strict`/`lenient` presets
  changing only the threshold. Added edge cases across `matcher`
  (R-2 `same_as` overlap, provider-scoped schemes not short-circuiting,
  cross-provider course-code skipped, provider-name fallback,
  educational-level ladder, Jaccard skip/zero, diacritic round-trip,
  `find_matches` filtering, strict-vs-lenient), `scoring`,
  `normalize`, and `phonetic`. New integration suite
  [`tests/public_api.rs`](./tests/public_api.rs) drives the
  re-exported surface end-to-end (deterministic rules, confidence
  bands, renormalisation, rank/find_matches).
- **Demo binary** ([`src/main.rs`](./src/main.rs)). A runnable
  `cargo run` walkthrough of the public API — deterministic course-code
  and DOI short-circuits, fuzzy name + keyword overlap, the Soundex
  bonus, educational-level adjacency, strict-vs-lenient thresholds, and
  one-to-many ranking. Harmonises with the sibling matcher crates,
  each of which already ships a demo binary, and is the consumer of the
  musl-gated `mimalloc` allocator. Not part of the SemVer surface.
- `#[must_use]` on the pure constructors, accessors, and scoring
  functions across the public surface (`Course::new`,
  `IdentifierScheme::is_deterministic`, `MatchConfig::{strict,lenient}`,
  the `MatchingEngine` query methods, `Confidence::classify`,
  `normalize::*`, `phonetic::*`, `scoring::weighted_average`).
- Worked rustdoc `# Examples` on the primary public API
  (`Course::new`, `IdentifierScheme::is_deterministic`,
  `MatchConfig::{strict,lenient}`, `MatchingEngine::{new,match_courses}`,
  `Confidence::classify`), exercised as doctests.

### Changed

- Cleaned up `clippy::pedantic` lints (let-chains, `let…else`,
  `usize` cast avoidance via `abs_diff`, explicit-import over
  glob) with no change to matching behaviour.
- **Spec/code drift fixed.** `spec.md §9` now documents the Soundex
  phonetic bonus (T-6) that the code and `agents/matching-algorithm.md`
  already described; §2.1 lists `match_one_to_many`; §4 notes the demo
  binary and that `mimalloc` is demo-only.
- Removed the duplicate `serde_json` `[dev-dependencies]` entry (it is
  already a normal dependency; siblings list it once).

## [0.6.1] — 2026-06-10

### Changed

- **Edition 2024.** Crate now builds on the Rust 2024 edition (enables
  let-chains used in the matcher).
- **`mimalloc` relocated.** The musl-gated global allocator was removed
  from `lib.rs`; the library now sets no global allocator. The
  allocator lives only in the demo binary, matching the family layout.
- **Family version alignment.** Versions 0.3.0–0.6.0 were coordinated
  family-wide bumps (no course-matcher behavioural change). 0.4.0
  tracked the matcher-family release that eliminated `chrono`;
  `course-matcher` never carried a date dependency, so that bump was a
  no-op here. Now aligned at 0.6.1 with the sibling matcher crates.

## [0.2.0] — 2026-06-05

### Added

- **index.md "Quick examples" updated for T-6 + T-10.** Previously
  only Identical / DOI / Ranking examples. Added a Phonetic name
  bonus block (Smyth↔Smith with the initial-letter caveat) and a
  `match_one_to_many` block showing the input-order variant
  alongside `rank` so readers can pick the right shape for their
  use case.
- **agents/testing.md realigned post-T-6/T-10.** Coverage table
  was missing the `phonetic` module entirely (added with the four
  Russell-style tests + the homophone-pair helper) and didn't
  mention the new `match_one_to_many` / Soundex-bonus tests on
  `matcher`. Benchmarks section claimed "Out of MVP scope. Once
  criterion is wired in..." though benches live in the embedding
  course-service crate at
  [`benches/matching_bench.rs`](../course-service-with-loco/benches/matching_bench.rs).
  Updated both, and added two new symptom-decoder rows ("Phonetic
  bonus suddenly stops firing" / "...never caps") so future
  failures decode straight to the underlying invariant.

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
- `AGENTS.md` + `agents/{index, spec-driven-development,
  matching-algorithm, normalization, testing}.md`.
- `README.md` with quick-start, algorithm summary, public surface,
  build / test commands.

### Pending (next iterations)

Tracked in `spec.md §23`:

- T-6: Soundex phonetic bonus on the `name` component.
- T-7: Service-side adapter + bridge test in
  [`course-service-with-loco/tests/duplicate_detection.rs`](../course-service-with-loco/tests/duplicate_detection.rs).
- T-8: Criterion benchmarks (name match, full match, rank-of-100).
- T-10: Expose `MatchingEngine::match_one_to_many` for parity with
  sibling matcher crates.

### Cross-references

- Sibling matcher crates for the algorithmic family:
  [`person-matcher-rust-crate`](../../person/person-matcher-rust-crate/),
  [`event-matcher-rust-crate`](../../event/event-matcher-rust-crate/),
  [`place-matcher-rust-crate`](../../place/place-matcher-rust-crate/),
  [`thing-matcher-rust-crate`](../../thing/thing-matcher-rust-crate/),
  [`worker-matcher-rust-crate`](../../worker/worker-matcher-rust-crate/).
- Embedding service:
  [`course-service-with-loco`](../course-service-with-loco/).
