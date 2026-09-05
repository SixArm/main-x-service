# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added — fuzz/property/bench coverage for `relationships`/`tags` (CPM-T2)

These two components landed 2026-08-28 but were exercised only by the
hand-written unit tests in `src/matcher.rs` — the property suite, the
`cargo-fuzz` harness, and the Criterion bench never touched either
field. Ported organization-matcher's identical ORGM-T2 fix:
`tests/property_tests.rs`'s `pathway()` now generates `relationships`
(a `RelationshipRef` struct literal, the five fixed `RelationKind`
variants plus a free-form `Custom` label) and `tags`, covered by the
existing never-panic/bounded-score/symmetric-matching properties with
no new property needed. `fuzz/fuzz_targets/match_care_pathways.rs`
appends a relationship/tag derived from the raw fuzz bytes to both
pathways after the JSON-tuple decode, so the field paths are reachable
on every run rather than depending on the corpus finding them by
chance. `benches/match_pair.rs` gained a `relationships_and_tags` group
(`partial_overlap`) so a perf regression on either component's Jaccard
cost is visible. See spec/index.md CPM-T2.

### Added — `MatchConfig::validated` guards against adversarial weights (CPM-T1)

Every `MatchConfig` field is `pub` and directly settable, and nothing
validated them — a negative, `NaN`, or infinite weight (or an
out-of-range threshold) reached `scoring::weighted_average` unchecked,
where it could push a returned score outside `[0.0, 1.0]` or produce
`NaN`. Ported the sibling `organization-matcher` crate's ORGM-T1 fix
(identical `MatchConfig` shape): added the additive, opt-in
`MatchConfig::validated(self) -> Result<Self>`, rejecting a malformed
weight or threshold with the new `Error::InvalidConfig` variant; the
plain struct literal is unchanged for the common case. A new proptest
generates adversarial-or-well-formed weight vectors and confirms
`validated`'s accept/reject boundary holds and that an accepted config
never produces an unbounded or NaN score.

### Added — relationships and tags as weighted components (§13.1 / §13.2, §23)

- **`RelationshipRef` / `RelationKind` (§13.1).** New public types
  `RelationKind` (`#[non_exhaustive]`: `PrecededBy` / `FollowedBy`
  sequencing inverses, `SimilarTo` symmetric, `Supersedes` /
  `SupersededBy` versioning inverses, plus `Custom(String)`) and
  `RelationshipRef { relation: RelationKind, pathway_id: String }`,
  re-exported from the crate root. `CarePathway` gains `relationships:
  Vec<RelationshipRef>` (default empty, `#[serde(default)]`). Scored as
  typed-set Jaccard over `(relation, pathway_id)` pairs
  (`|A ∩ B| / |A ∪ B|`) into the new
  `MatchBreakdown::relationships_score` (`#[serde(default)]`), `None`
  when either side's list is empty. `pathway_id` is folded (trimmed,
  case-normalised) at scoring time — consistent with the crate's
  normalise-at-match-time convention — and an entry whose id folds to
  empty is dropped from the comparison set rather than spuriously
  matching another blank id (SEC-M2 discipline, same as the R-0/R-1
  deterministic rules). New `MatchConfig::relationships_weight`
  (default `0.05`) joins the supporting-signal cluster in the
  weighted-average renormalisation. `RelationshipRef` deliberately
  references another pathway **template** (or a template-derived
  instance a consuming service names by the same id space) — it never
  carries patient-identifying data.
- **`tags` (§13.2).** `CarePathway` gains `tags: Vec<String>` (default
  empty, `#[serde(default)]`). Tags are stored verbatim and compared
  case-insensitively (folded) at scoring time, distinct from `keywords`
  (descriptive terms about what the pathway *is*, not operator-applied
  labels), as set Jaccard (`|A ∩ B| / |A ∪ B|`) into the new
  `MatchBreakdown::tags_score` (`#[serde(default)]`), `None` when
  either side's list is empty. New `MatchConfig::tags_weight` (default
  `0.05`) joins the supporting-signal cluster alongside
  `relationships_weight`.
- Both fields are purely additive and **not** identifying on their own
  and **not** consulted by any deterministic short-circuit (R-0/R-1/R-2).
  Neither field participates in the weighted average unless populated on
  *both* sides, so existing callers that never set `relationships`/`tags`
  see byte-identical scores before and after this change — the two new
  default weights simply never enter the denominator for them. This
  differs from the existing `interventions`/`keywords` Jaccard
  components (`None` only when *both* sides are empty): relationships
  and tags are sparser, opt-in data, so "no signal on either side" is
  the "does not participate" case per spec §13.1/§13.2, matching the
  `worker-matcher` T-33/T-34 precedent.
- `agents/matching-algorithm.md`'s component table gains Relationships /
  Tags rows. `spec/index.md` §5 / §6 / §7 / §13.1 / §13.2 / §21 / §23
  move from "planned, not yet implemented" to current behaviour.
- No `Cargo.toml` version bump: this crate has carried an accumulating
  `[Unreleased]` section since `0.1.0` (2026-06-15) rather than cutting a
  release per change (unlike `worker-matcher`'s per-change minor bumps),
  so this entry joins that section per the crate's own established
  precedent; the next release cut will fold it in.
- New tests: `relationships_score` / `tags_score` unit tests (identical,
  disjoint, partial-overlap-Jaccard-ratio, empty-either-side, plus a
  blank-pathway-id SEC-M2 case for relationships), a default-weight pin,
  and three engine-level tests (absent fields don't enter the weighted
  average; present + agreeing fields score 1.0; present + disagreeing
  fields pull the score down but a strong multi-component match still
  clears the default threshold).

### Added — Criterion benchmarks

- `benches/match_pair.rs`, matching the harness the other matcher crates
  carry. Four groups over the paths a downstream integrator actually
  exercises: single-pair scoring in three regimes (identical clone,
  fuzzy near-match through the full pipeline, unrelated pair), the
  deterministic short-circuits (a shared DOI, and a shared provider-scoped pathway code), `rank` at 10 / 100 / 1000
  candidates with `Throughput::Elements` set so per-candidate cost and
  any super-linear scaling are visible directly, and the same fuzzy pair
  under each shipped config preset.
- Fixtures are built from `CarePathway::new` and derived deterministically from
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
  `../spec.md` link display text to `../spec/index.md` in `agents/`;
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
  (§21). This was the design-only entry; the code implementation landed
  later in this same `[Unreleased]` section — see "relationships and
  tags as weighted components" above.
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
