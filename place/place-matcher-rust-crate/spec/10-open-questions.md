## 10. Open questions

The following design questions are deliberately unresolved. Proposing a resolution is welcome; do so in a PR rather than a unilateral code change.

- **OQ-A — Category hierarchy.** `PlaceCategory` is flat; should an ancestor-level hierarchy allow partial credit (e.g. `Cafe < FoodService`)? Trade-off: explainability vs recall.
- **OQ-B — Country-code canonicalisation at construction.** Should `PlaceBuilder::country_code_as_iso_3166_1_alpha_2` uppercase and validate at construction, or preserve round-trip honesty as today?
- **OQ-C — Multi-polygon / area definitions.** `area_as_metre_2` is scalar. Should `Place` gain an optional polygonal extent (WKT, GeoJSON, `Vec<(f64, f64)>`) and a point-in-polygon / overlap scorer?
- **OQ-D — Locale-aware street-type vocabulary.** Should `expand_street_abbreviations` gain locale vocabularies for `rue` / `straße` / `via` / `calle` / `straat`? If so: opt-in field, Cargo feature, or always-on?
- **OQ-E — Phonetic-encoder choice.** American Soundex is English-tuned. Add Double Metaphone or Daitch-Mokotoff behind a Cargo feature, default unchanged?
- **OQ-H — Per-category default for `coordinates_scale_metres`.** The `50.0` m default suits venue precision; dense urban chains may need a per-category default.

The following are grounded follow-up **tasks** rather than open design
questions — each has a concrete, verified gap and a clear direction, so
they are recorded here (this crate carries no `spec/13-tasks.md`
checklist — see `spec/index.md`'s table of contents, §1–§13 with no
"Tasks" section) rather than left purely as prose in `CHANGELOG.md`.

- ~~**OQ-I (task, M, security) — Empty-value identity bypass on `PlaceId`
  via direct construction/deserialization.**~~ — **RESOLVED (2026-09-04).**
  `PlaceId::new` rejects an empty trimmed `value`, but `PlaceId` is not
  `#[non_exhaustive]` and both fields are `pub` with `#[derive(Deserialize)]`,
  so a struct literal or `serde_json` deserialization of untrusted input
  bypassed the constructor's guard entirely. `shares_place_id`
  (`src/matcher.rs`) now skips any `PlaceId` whose trimmed `value` is
  empty on either side — matching the constructor's own definition of
  "empty" — before comparing `id1 == id2`, closing the same false-identity
  class already fixed for `name_and_postcode_match` (0.7.0). One
  correction found while implementing: the acceptance note above cited a
  `shares_same_as` function as "the right pattern to copy"; no such
  function exists in this crate (verified: `grep -rn "same_as"
  src/*.rs` returns nothing) — the fix was written directly against
  `shares_place_id` instead. Two unit tests pin it:
  `empty_value_place_id_never_matches_even_via_constructor_bypass` and
  `whitespace_only_value_place_id_never_matches` (`src/matcher.rs`).
  `cargo test` (165 lib tests passed, up from 163) + `cargo clippy
  --all-targets -- -D warnings` + `cargo doc --no-deps` all clean. See
  `CHANGELOG.md` "Security" under `[Unreleased]`.

- **OQ-J (task, M) — Implement the spec'd-but-missing `setting`/`tags`
  weighted components.** `spec/03-data-model.md` §3.1.3, `spec/06-*.md`,
  and `spec/07-configuration.md` already fully specify `setting:
  Option<IndoorOutdoor>` and `tags: Vec<String>` (default weight `0.05`
  each) and say in as many words "not yet implemented" — and
  `CHANGELOG.md`'s `[Unreleased]` section carries a fully-scoped
  implementation plan for both, dated before this task list existed.
  *(verified: `grep -n "setting_score\|tags_score\|setting_weight\|
  tags_weight" src/models.rs src/scorer.rs` returns nothing anywhere in
  `src/`; `grep -n "not yet implemented" spec/03-data-model.md` at line
  185 confirms the spec's own accounting.)* **Acceptance:** `Place`
  gains `setting: Option<IndoorOutdoor>` (`Indoor`/`Outdoor`/`Mixed`,
  `#[non_exhaustive]`) and `tags: Vec<String>`, both on the builder;
  `setting_score` (1.0 equal / 0.5 Mixed-vs-either / 0.0 Indoor-vs-
  Outdoor / `None` if either absent) and `tags_score` (case-insensitive
  set Jaccard, `None` if either side's tag set is empty) land on
  `MatchBreakdown`; `setting_weight`/`tags_weight` (default `0.05`
  each) join `MatchConfig`'s renormalised weighted sum; existing
  callers that never set either field see byte-identical scores
  (neither weight enters the denominator until populated on both
  sides); `cargo test` + clippy pedantic clean; `CHANGELOG.md`
  "Unreleased" entry closed out rather than left open.

- ~~**OQ-K (task, S) — Resolve OQ-F: opt-in `local_id` scoring.**~~ —
  **RESOLVED (2026-09-05).** `local_id` was deliberately unscored
  (`AGENTS.md`: "Do not score `local_id`. Different organisations may
  issue colliding values.") — correct as a default, but OQ-F had stood
  unresolved since this file's inception with no way for a caller who
  legitimately compares records from one known source to opt in.
  `MatchConfig` gained an opt-in `score_local_id: bool` (default
  `false`, preserving the previous behaviour byte-for-byte) plus
  `local_id_weight: f64` (default `0.05`); when `true`,
  `score_local_id` (`src/matcher.rs`) joins `local_id_score` into the
  weighted sum — exact match after trim, `1.0`/`0.0`, `None` if either
  side's trimmed value is empty or absent (mirroring the empty-value
  guard already on `place_ids`/`PlaceId`, closed as OQ-I). Adding the
  fourth boolean config flag tripped clippy pedantic's
  `struct_excessive_bools`; resolved with a narrow, documented
  `#[allow(clippy::struct_excessive_bools)]` on `MatchConfig` rather
  than changing the field's type, since the acceptance criterion
  specifically called for a `bool`. Four new unit tests
  (`local_id_unscored_by_default_even_when_identical`,
  `local_id_scores_one_when_opted_in_and_equal`,
  `local_id_scores_zero_when_opted_in_and_different`,
  `local_id_none_when_opted_in_but_either_side_absent_or_blank`) pin
  both the default-off behaviour and the opt-in scoring; `cargo test`
  (169 lib tests, up from 165) + `cargo clippy --all-targets -- -D
  warnings` + `cargo doc --no-deps` all clean. §6 (new §6.7a) and §7
  updated in the same change. This resolves OQ-F, removed from the
  list above.

- ~~**OQ-L (task, S) — Resolve OQ-G: score address `line2`/`county`/
  `country` as low-weight supporting fields.**~~ — **RESOLVED.** These
  three fields are stored on `Address` but were never read by address
  scoring — *(verified, with one correction to the task's own claim:
  the scoring function is `MatchingEngine::compare_addresses` in
  `src/matcher.rs`, not `src/scorer.rs` as originally cited, and the
  existing fields it reads are `postcode`/`city`/`line1`, not
  `locality`/`region`; `grep -n "line2\|county\|country"
  src/matcher.rs` before this change confirmed the three were absent
  from that function).* Each of `line2` (`0.1`), `county` (`0.1`),
  `country` (`0.05`) now contributes a small sub-weight in
  `compare_addresses`'s existing weight-renormalised average, only when
  populated on both sides. **Chosen interpretation of "redistribution
  within address scoring, not a new top-level component":** the three
  new sub-weights are **additive** on top of the unchanged
  `postcode`/`city`/`line1` weights (`0.5`/`0.3`/`0.2`, still summing to
  `1.0` among themselves) rather than shrinking them to make room — this
  keeps `address_score` for the overwhelmingly common case (line2/
  county/country absent) byte-identical to before, and satisfies "not a
  new top-level component" because `MatchConfig::address_weight` itself
  is untouched; a true redistribution would have silently changed every
  existing address comparison that never populates the three new
  fields. Each is guarded against a shared **blank** value scoring a
  spurious `1.0` (`Scorer::jaro_winkler_similarity("", "")` is `1.0` by
  design) — normalise first, then require non-empty on both sides,
  mirroring `local_id_score`'s "blank on both sides is not shared
  identity" rule (§6.7a); `city`/line 1 predate this guard and were left
  as found (a separate, narrower finding, not in this task's scope).
  Four new unit tests in `src/matcher.rs` (each field alone; one-sided
  presence doesn't participate; a real mismatch stays bounded and
  postcode still dominates; three blank values together fall back to
  the neutral `0.5`, not a spurious near-1.0). `cargo test` (173 lib
  tests, up from 169; 97 integration/doctest) + `cargo clippy
  --all-targets -- -D warnings` + `cargo fmt --check` + `cargo doc
  --no-deps` all clean. §6.4 updated; this closes OQ-G, removed from
  the list above.

---

