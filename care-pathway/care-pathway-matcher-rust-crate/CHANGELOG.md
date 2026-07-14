# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added — cargo-fuzz harness (SEC-I2)

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate adopting
  the person-matcher reference scaffolding, with two coverage-guided
  libFuzzer targets: `match_care_pathways` (deserialize a JSON `[care_pathway_a, care_pathway_b]` tuple →
  `MatchingEngine::match_care_pathways`; finite score in `[0,1]`, both orders) and
  `normalize` (the pure `normalize` free functions — fold / pathway code / fold-set — over arbitrary
  UTF-8, never-panic). Two targets rather than the reference three because
  this crate exposes its similarity primitives only through the engine (the
  `scoring` module publishes no string-similarity functions). Run on nightly:
  `cargo +nightly fuzz run <target>` (see `fuzz/README.md`). The `fuzz/` crate
  is standalone (not a workspace member), so it never affects the crate’s
  normal stable build/test/clippy. Verified: `cargo +nightly fuzz build`
  compiles both targets and short campaigns run clean (millions of execs, no
  panics).

### Added

- SEC-M6: property-based tests (`tests/property_tests.rs`, `proptest`
  dev-dependency) proving the matcher never panics and its scores stay
  well-behaved on arbitrary input. Invariants: the pure normalise /
  phonetic helpers never panic on arbitrary strings; every
  `match_care_pathways` score is finite and within `[0.0, 1.0]` (never
  NaN); matching is symmetric (score, `is_match`, and `confidence`
  invariant under argument swap); and an identical clone of a well-formed
  pathway matches itself. Tests-only; no behaviour, weight, or threshold
  change.

### Security

- SEC-M2: the provider-scoped deterministic rule (R-1) now requires the
  normalised pathway code to be non-empty before short-circuiting. Two
  different pathways sharing a provider with blank / punctuation-only
  codes (e.g. `"-"`, `"  "` — both normalise to `""`) no longer
  spuriously match to a 1.0 identity. Same fix class as person-matcher's
  `passport_books_share_pair`.

### Fixed

- Formatting drift in `src/matcher.rs` (two spots not rustfmt-formatted);
  `cargo fmt --check` is clean again. No behaviour change.

### Changed

- Documentation harmonization pass: fixed the `repository` URL in
  `Cargo.toml` to a valid `/tree/main/...` monorepo subpath; corrected
  `../spec.md` link display text to `../spec/index.md` in `AGENTS/`;
  added spec notes clarifying that `Error`/`Result` are reserved for
  future fallible APIs (§21) and that `provider_name` is
  informational-only (§6); added a renormalised partial-score worked
  example to `index.md` plus R-1 / R-2 worked examples.

### Added

- Spec: a `relationships` concept on `CarePathway` — `RelationshipRef
  { relation: RelationKind, pathway_id }` with `RelationKind`
  (`PrecededBy` / `FollowedBy` sequencing inverses, `SimilarTo`
  symmetric, `Supersedes` / `SupersededBy` versioning inverses, plus
  `Custom`), scored by a typed-set Jaccard over the `(relation,
  pathway_id)` pairs (§13.1) and weighted `relationships_weight`
  (default `0.05`, §7) as a supporting signal in the renormalised
  weighted average (§17). Re-exports `RelationshipRef` / `RelationKind`
  (§21). Code implementation is tracked in spec §23.
- Tests closing documented coverage gaps: phonetic (Soundex) +0.05
  bonus wiring in `name_score` (lift but never reaching the High band);
  `set_jaccard` `Some(0.0)` one-side-populated branch; an
  `alternate_names` rescue where primary names diverge.

## [0.1.0] - 2026-06-15

### Added

- **Inaugural release.** Pairwise care-pathway (clinical
  pathway) record matching, copy-adapted from the course-matcher
  template.
  - Domain model: `CarePathway` (name / alternateName / pathway code /
    provider / care setting / condition codes / interventions / keywords
    / identifiers / sameAs), `ConditionCode`, `CodeSystem`,
    `CareSetting`, `PathwayIdentifier`, `IdentifierScheme`.
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (DOI, Wikidata, guideline-registry id, URI, UUID); R-1
    same-provider pathway code; R-2 `same_as` URL overlap.
    Provider-scoped (`PathwayCode`/`LocalId`) and `Custom` never
    short-circuit.
  - **Probabilistic components**: name (Jaro-Winkler + Soundex bonus),
    target condition codes (Jaccard over `system:code` tokens),
    provider-scoped pathway code, care setting, interventions (Jaccard),
    keywords (Jaccard); renormalised over present components.
  - Normalisation: `fold`, `pathway_code` (alphanumeric-only), `fold_set`.
  - 38 embedded unit tests + a 10-test public-API integration suite + 7
    doctests. Green `cargo build`, clippy clean, rustfmt formatted.
