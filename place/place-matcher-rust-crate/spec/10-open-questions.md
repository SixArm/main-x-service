## 10. Open questions

The following design questions are deliberately unresolved. Proposing a resolution is welcome; do so in a PR rather than a unilateral code change.

- **OQ-A — Category hierarchy.** `PlaceCategory` is flat; should an ancestor-level hierarchy allow partial credit (e.g. `Cafe < FoodService`)? Trade-off: explainability vs recall.
- **OQ-B — Country-code canonicalisation at construction.** Should `PlaceBuilder::country_code_as_iso_3166_1_alpha_2` uppercase and validate at construction, or preserve round-trip honesty as today?
- **OQ-C — Multi-polygon / area definitions.** `area_as_metre_2` is scalar. Should `Place` gain an optional polygonal extent (WKT, GeoJSON, `Vec<(f64, f64)>`) and a point-in-polygon / overlap scorer?
- **OQ-D — Locale-aware street-type vocabulary.** Should `expand_street_abbreviations` gain locale vocabularies for `rue` / `straße` / `via` / `calle` / `straat`? If so: opt-in field, Cargo feature, or always-on?
- **OQ-E — Phonetic-encoder choice.** American Soundex is English-tuned. Add Double Metaphone or Daitch-Mokotoff behind a Cargo feature, default unchanged?
- **OQ-G — Address `line2`, `county`, `country` scoring.** These are stored but not scored (§6.4). Should they contribute, and with what sub-weights?
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

- **OQ-L (task, S) — Resolve OQ-G: score address `line2`/`county`/
  `country` as low-weight supporting fields.** These three fields are
  stored on `Address` but never read by `src/scorer.rs`'s address
  scoring, per the spec's own §6.4 note this open question already
  cites. *(verified: `grep -n "line2\|county\|country"
  src/scorer.rs` returns nothing — the address scorer only reads
  `line1`/`locality`/`region`/`postcode`.)* **Acceptance:** each of
  `line2`/`county`/`country` contributes a small sub-weight within the
  existing weighted field-by-field address score (only when present on
  both sides, per the crate's "only fields present in both records
  contribute" rule), with the address component's own weight unchanged
  in `MatchConfig`'s top-level sum (so this is a redistribution within
  address scoring, not a new top-level component); unit tests cover
  each field's presence/absence contribution; `cargo test` + clippy
  pedantic clean; spec §6.4 updated (this closes OQ-G — remove it from
  the list above once landed).

---

