# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added — `MatchConfig::validated`, guarding against caller-supplied negative/NaN weights

Every `MatchConfig` field is `pub` with no validating constructor, and
the property-test suite only ever exercised
`MatchingEngine::default_config()` — a caller-built config with a
negative, `NaN`, or infinite weight (or an out-of-range threshold)
could reach `scoring::weighted_average` unchecked, pushing the
returned score outside `[0.0, 1.0]` or producing `NaN`. Ported
organization-matcher's/care-pathway-matcher's identical
`MatchConfig::validated(self) -> Result<Self>` fix: rejects a
negative/`NaN`/infinite weight on any of the six weight fields, or a
threshold outside `[0.0, 1.0]`, via a new `Error::InvalidConfig(String)`
variant naming the first offending field; the plain struct literal
keeps working for the common case. Six new unit tests in
`src/config.rs`; a new proptest
(`validated_config_never_produces_an_unbounded_score`,
`tests/proptests.rs`) generates adversarial weight/threshold vectors
and asserts an accepted config's score stays finite and bounded while
a rejected one really was malformed. Spec §7 documents the validator;
§19 states explicitly that the bounded-and-finite score guarantee
covers a hand-built `MatchConfig` only once it clears `validated`.
See spec/index.md §23.

### Documented — the caller must bound `subjects`/`keywords` array sizes

This crate has no length cap of its own on `subjects`/`keywords` — both
are scored by set Jaccard over every element, an unbounded O(n·m)
operation. The family's SEC-M1 caps (`MAX_ARRAY_LEN`/`MAX_ITEM_LEN`)
live only in `case-service`'s validation layer, which runs before this
crate is ever called; a standalone integrator has no equivalent. Added
a "the caller must bound array sizes" section to the crate-root docs
and a matching note on `MatchingEngine::match_cases`'s rustdoc, plus a
new `AGENTS.md` golden rule, all pointing at `case-service`'s
`MAX_ARRAY_LEN`/`MAX_ITEM_LEN` as the reference cap to copy. No code
change; see spec/index.md §23.

### Added — Criterion benchmarks

- `benches/match_pair.rs`, matching the harness the other matcher crates
  carry. Four groups over the paths a downstream integrator actually
  exercises: single-pair scoring in three regimes (identical clone,
  fuzzy near-match through the full pipeline, unrelated pair), the
  deterministic short-circuits (a shared docket, and a shared agency-scoped case number), `rank` at 10 / 100 / 1000
  candidates with `Throughput::Elements` set so per-candidate cost and
  any super-linear scaling are visible directly, and the same fuzzy pair
  under each shipped config preset.
- Fixtures are built from `Case::new` and derived deterministically from
  an index, so a candidate list of any size is reproducible without
  randomness.

### Added — declared MSRV (Rust 1.96)

- `Cargo.toml` now declares `rust-version = "1.96"`, the repository's
  **current stable minus two** floor
  (`spec/rust-msrv-n-minus-2/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.96 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption. *(Corrected 2026-09-06: this entry
  originally said 1.95 / N-3, matching the policy at the time it was
  written; the repository-wide MSRV policy has since tightened to N-2,
  and `Cargo.toml` already declares 1.96 — this entry is edited in place,
  since it was still `[Unreleased]`, rather than left to misstate the
  crate's actual floor.)*

### Added — cargo-fuzz harness (SEC-I2)

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate adopting
  the person-matcher reference scaffolding, with two coverage-guided
  libFuzzer targets: `match_cases` (deserialize a JSON `[case_a, case_b]` tuple →
  `MatchingEngine::match_cases`; finite score in `[0,1]`, both orders) and
  `normalize` (the pure `normalize` free functions — fold / case number / URL / fold-set — over arbitrary
  UTF-8, never-panic). Two targets rather than the reference three because
  this crate exposes its similarity primitives only through the engine (the
  `scoring` module publishes no string-similarity functions). Run on nightly:
  `cargo +nightly fuzz run <target>` (see `fuzz/README.md`). The `fuzz/` crate
  is standalone (not a workspace member), so it never affects the crate’s
  normal stable build/test/clippy. Verified: `cargo +nightly fuzz build`
  compiles both targets and short campaigns run clean (millions of execs, no
  panics).

### Security

- **SEC-M2 — trivial/sentinel values no longer force a deterministic
  match.** The deterministic short-circuits in `src/matcher.rs` now skip
  non-identity placeholder values so two *different* cases cannot pin to
  `1.0` on shared junk:
  - `R-0` (globally-unique identifiers) skips a *trivial* value — one with
    no alphanumeric character other than `'0'`, i.e. empty/punctuation-only,
    the sentinel `"0"`, or an all-zeros UUID — via the new
    `is_trivial_identifier` guard.
  - `R-2` (`same_as` URL overlap) skips a bare root `"/"` (which
    `normalize::url` intentionally keeps non-empty) in addition to the
    empty case.
  - Added `trivial_zero_identifier_does_not_short_circuit` and
    `trivial_root_same_as_does_not_short_circuit` tests (each keeps a
    positive control that a real shared id/URL still matches). No weights,
    thresholds, or probabilistic behaviour changed.

### Changed

- **Docs/CI harmonization pass** (no behavioural change):
  - Fixed `Cargo.toml` `repository` URL to the nested, valid GitHub tree
    path `…/tree/main/case/case-matcher-rust-crate`.
  - Quoted the clippy gate as `cargo clippy --all-targets --all-features
    -- -D warnings` in spec §24, `AGENTS.md`, `agents/testing.md`, and
    `README.md` to match CI and the repo-wide harmonization.
  - Classified `in_language` as a data-only field (spec §6/§14/§22,
    `AGENTS.md`, `README.md`, `agents/matching-algorithm.md`,
    `agents/spec-driven-development.md`): carried but never scored.
  - Corrected `../spec.md` link display text to `../spec/index.md` in
    `agents/index.md` and `agents/spec-driven-development.md`.

### Added

- **SEC-M6 — property-based tests.** New `tests/proptests.rs` (dev-only,
  `proptest = "1.11"`) drives many random inputs to prove the matcher is
  well-behaved: the engine never panics and `MatchResult::score` (and every
  breakdown sub-score) is finite and in `[0.0, 1.0]` (never `NaN`);
  `match_cases` is symmetric in argument order (score, `is_match`,
  confidence); an identical clone of a well-formed case matches itself;
  the pure helpers (`fold` / `case_number` / `url` / `fold_set` /
  `phonetic::soundex` / `phonetic::same` / `Confidence::classify`) never
  panic on arbitrary UTF-8 / floats; and `soundex` returns `None` or a
  `[A-Z][0-9]{3}` code. Tests + dev-dependency only — no behaviour,
  weights, or thresholds changed.
- Tests for previously-uncovered spec'd behaviour: the Soundex `+0.05`
  title bonus integration (§9), `alternate_titles` contribution and
  symmetry (§9), the `keywords` Jaccard component end-to-end (§13), and
  the documented serde wire shape of unit vs `Custom` enum variants (§6).
  Lib unit tests 42 → 46; public-API integration tests 12 → 13.
- Worked examples for the Soundex phonetic title bonus, renormalisation
  over partial components, and the strict/lenient threshold presets in
  `index.md` and `agents/matching-algorithm.md`.

## [0.1.0] - 2026-06-13

### Added

- **Inaugural release.** Pairwise governmental case-management record
  matching, copy-adapted from the care-pathway-matcher template.
  - Domain model: `Case` (title / alternate titles / case number /
    agency / case type / status / priority / opened date / subjects /
    keywords / identifiers / sameAs / in language), `CaseType`,
    `CaseStatus`, `Priority`, `CaseIdentifier`, `IdentifierScheme`.
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (`Docket`, `ExternalCaseId`, URI, UUID); R-1 same-agency case
    number; R-2 `same_as` URL overlap. Agency-scoped
    (`AgencyCaseNumber`/`LocalId`) and `Custom` never short-circuit.
  - **Probabilistic components**: title (Jaro-Winkler + Soundex bonus,
    0.30), subjects (Jaccard, 0.25), agency-scoped case number (0.15),
    case type (exact, 0.10), status (exact, 0.05), keywords (Jaccard,
    0.15); renormalised over present components. `priority` and
    `opened_date` are carried but never scored.
  - Normalisation: `fold`, `case_number` (alphanumeric-only), `url`,
    `fold_set`.
  - 42 embedded unit tests + a 12-test public-API integration suite + 7
    doctests. Green `cargo build`, clippy clean, rustfmt formatted.
