## 10. Open questions

The following design questions are deliberately unresolved. Proposing a resolution is welcome; do so in a PR rather than a unilateral code change.

- **OQ-A — Category hierarchy.** `PlaceCategory` is flat; should an ancestor-level hierarchy allow partial credit (e.g. `Cafe < FoodService`)? Trade-off: explainability vs recall.
- **OQ-B — Country-code canonicalisation at construction.** Should `PlaceBuilder::country_code_as_iso_3166_1_alpha_2` uppercase and validate at construction, or preserve round-trip honesty as today?
- **OQ-C — Multi-polygon / area definitions.** `area_as_metre_2` is scalar. Should `Place` gain an optional polygonal extent (WKT, GeoJSON, `Vec<(f64, f64)>`) and a point-in-polygon / overlap scorer?
- **OQ-D — Locale-aware street-type vocabulary.** Should `expand_street_abbreviations` gain locale vocabularies for `rue` / `straße` / `via` / `calle` / `straat`? If so: opt-in field, Cargo feature, or always-on?
- **OQ-E — Phonetic-encoder choice.** American Soundex is English-tuned. Add Double Metaphone or Daitch-Mokotoff behind a Cargo feature, default unchanged?
- **OQ-F — `local_id` scoring opt-in.** Should a caller be able to opt in to scoring `local_id` when comparing records from a single source?
- **OQ-G — Address `line2`, `county`, `country` scoring.** These are stored but not scored (§6.4). Should they contribute, and with what sub-weights?
- **OQ-H — Per-category default for `coordinates_scale_metres`.** The `50.0` m default suits venue precision; dense urban chains may need a per-category default.

The following are grounded follow-up **tasks** rather than open design
questions — each has a concrete, verified gap and a clear direction, so
they are recorded here (this crate carries no `spec/13-tasks.md`
checklist — see `spec/index.md`'s table of contents, §1–§13 with no
"Tasks" section) rather than left purely as prose in `CHANGELOG.md`.

- **OQ-I (task, M, security) — Empty-value identity bypass on `PlaceId`
  via direct construction/deserialization.** `PlaceId::new` rejects an
  empty trimmed `value`, but `PlaceId` is **not** `#[non_exhaustive]`
  and both fields (`scheme: PlaceIdScheme`, `value: String`) are `pub`
  with `#[derive(Deserialize)]` — so `PlaceId { scheme:
  PlaceIdScheme::Google, value: String::new() }` is constructible via a
  struct literal, and **is likewise reachable via `serde_json`
  deserialization of untrusted input**, entirely bypassing the
  constructor's guard. `shares_place_id` (`src/matcher.rs`) only checks
  `p1.place_ids.is_empty() || p2.place_ids.is_empty()` before comparing
  `id1 == id2` — two places each carrying one `PlaceId { scheme:
  Google, value: "" }` (e.g. from a JSON payload with `"value": ""`)
  satisfy that equality and spuriously trip `deterministic_match`,
  exactly the SEC-M2 false-identity class this crate already fixed for
  `name_and_postcode_match` (0.7.0, see `CHANGELOG.md`) — but that fix
  never touched this construction-bypass path. `shares_same_as`
  (same file) already has the right pattern to copy: it skips entries
  whose normalised form is empty. *(verified: `sed -n '318,325p'
  src/models.rs` shows `PlaceId`'s fields are `pub` with no
  `#[non_exhaustive]`, deriving `Deserialize`; `sed -n '949,960p'
  src/matcher.rs` shows `shares_place_id`'s only empty-guard is on the
  outer `Vec`, not per-entry.)* **Acceptance:** `shares_place_id`
  additionally skips any `PlaceId` whose `value` is empty (mirroring
  `shares_same_as`'s empty-skip), so a struct-literal- or
  deserialize-constructed empty-value id can never contribute to a
  match; a unit test constructs two `PlaceId`s with `value: String::new()`
  directly (bypassing `new`) on both sides and asserts
  `deterministic_match` returns `false`; existing `place_ids_scheme_
  local_no_cross_match`-style tests stay green; `cargo test` +
  `cargo clippy --all-targets -- -D warnings` clean; `CHANGELOG.md`
  entry under "Security" (same class as the 0.7.0 SEC-M2 fix).

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

- **OQ-K (task, S) — Resolve OQ-F: opt-in `local_id` scoring.**
  `local_id` is deliberately unscored (`AGENTS.md`: "Do not score
  `local_id`. Different organisations may issue colliding values.") —
  correct as a default, but OQ-F above has stood unresolved since this
  file's inception with no way for a caller who legitimately compares
  records from one known source to opt in. *(verified: `grep -n
  local_id src/matcher.rs src/scorer.rs` shows the field is read
  nowhere in the scoring path — confirming OQ-F's premise still
  holds.)* **Acceptance:** `MatchConfig` gains an opt-in
  `score_local_id: bool` (default `false`, preserving today's
  behaviour byte-for-byte) plus a `local_id_weight`; when `true`,
  `local_id_score` (exact-match after trim, `None` if either side is
  absent) joins the weighted sum; a unit test pins the default-off
  behaviour is unchanged and the opt-in behaviour scores as specified;
  `cargo test` + clippy pedantic clean; spec §6/§7 updated in the same
  change (this closes OQ-F, so remove it from the list above once
  landed).

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

