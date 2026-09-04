## 10. Open questions

The following design questions are deliberately unresolved. Proposing a resolution is welcome; do so in a PR rather than a unilateral code change.

- **OQ-A — Category hierarchy.** Today `EventCategory` is a flat enum; a `MusicEvent` and a `Festival` either match or they don't. Should the spec define a hierarchy (e.g. `ScreeningEvent < PerformingArtsEvent < Event`) and allow partial credit when categories agree at an ancestor level? Trade-off: explainability vs recall.
- **OQ-B — Country-code canonicalisation at construction.** `country_code_as_iso_3166_1_alpha_2` is stored as supplied; only the matcher trims and lowercases. Should `EventBuilder::country_code_as_iso_3166_1_alpha_2` canonicalise (uppercase, validate as exactly two ASCII letters) at construction time? Trade-off: round-trip honesty vs caller convenience.
- **OQ-C — Window overlap instead of endpoint proximity.** The temporal components score `start_date` and `end_date` independently by Gaussian decay over endpoint distance (§6.3). Should the matcher instead (or additionally) score the **overlap fraction** of the two `[start, end]` windows, so that a 4-day festival listed with slightly different day boundaries scores higher than two adjacent one-hour meetups with the same endpoint gap? If so, how should records with only a `start_date` participate?
- **OQ-D — Locale-aware street-type vocabulary.** Today only English abbreviations are expanded (`St`, `Rd`, `Ave`, …). Should the crate gain locale-aware vocabularies for `rue` / `straße` / `via` / `calle` / `straat`? If so, opt-in via a new `MatchConfig` field, gated behind a Cargo feature, or always-on?
- **OQ-E — Phonetic-encoder choice.** American Soundex is tuned for English. A locale-aware encoder (Double Metaphone, Daitch-Mokotoff) would improve recall for non-English names. Add behind a Cargo feature flag with the default unchanged?
- **OQ-F — `local_id` scoring opt-in.** `local_id` is currently never scored because different sources may issue colliding values. Should a caller be able to opt in to scoring `local_id` when they know they are comparing records from a single source?
- **OQ-G — Address `line2`, `county`, `country` scoring.** These fields are stored but not scored (§6.4). Should they contribute, and if so with what sub-weights?
- **OQ-H — Per-category temporal / spatial scale defaults.** The defaults `start_date_scale_seconds = 3600.0` and `coordinates_scale_metres = 100.0` suit single-venue, clock-scheduled events. Multi-day festivals tolerate hours of start-time drift across sources; stadium fixtures may need a wider coordinate scale than club gigs. Should defaults vary by `EventCategory`?
- **OQ-I — URL canonicalisation.** `url_score` and the `virtual_url` sub-score compare by exact equality after trim (§6.10). Should the matcher canonicalise URLs first (case-fold scheme and host, strip trailing slash and tracking query parameters)? Trade-off: recall vs false positives on path-significant platforms.

### Implementation tasks

Concrete, scoped follow-ups distinct from the open design questions
above — these have a clear "done" state and don't require a design
decision before starting. This crate carries no `spec/23-tasks.md`-style
queue (its `spec/` stops at `13-references.md` per the family
matcher-crate footnote), so these are tracked here per the crate's own
AGENTS.md guidance to prefer Open Questions over a unilateral decision.

- [ ] **T-1 (S)** Widen `tests/property_tests.rs::event_strategy()` to
  populate `location`, `event_ids`, `organizer`, `performers`, `url`,
  `country_code_as_iso_3166_1_alpha_2`, `relationships`, and `tags`
  (currently only `name`/`alternate_names`/`start_date`/`end_date`/
  `category` are generated), so the existing symmetry / bounded-score /
  self-match / JSON-round-trip properties actually exercise all 26
  `Event` fields, not 5 of them.
  *(verified: `event_strategy()` at `tests/property_tests.rs:52-73`
  sets only `name`, `alternate_names`, `start_date`, `end_date`,
  `category`; `relationships`/`tags` — added in 0.8.0 — are never
  populated by any property test.)*
  **Acceptance:** `event_strategy()` generates all scoreable fields
  (each via `prop::option::of`/`prop::collection::vec` as appropriate);
  `cargo test --test property_tests` passes with `cases: 500` unchanged;
  no existing property is weakened to accommodate the wider input.

- [ ] **T-2 (S)** Add property assertions (or a dedicated `proptest!`
  block) pinning `MatchConfig`'s full round-trip: extend
  `match_config_default_round_trips_through_json` to also assert
  `relationships_weight`, `tags_weight`, `event_ids_weight`,
  `organizer_weight`, `performers_weight`, `url_weight`, and
  `country_code_weight` survive JSON round-trip.
  *(verified: `match_config_default_round_trips_through_json`
  (`tests/property_tests.rs:152`) checks only `match_threshold`,
  `name_weight`, `start_date_weight`, `location_weight` — 4 of
  `MatchConfig`'s 12 weight fields.)*
  **Acceptance:** all `MatchConfig` weight fields present in
  `MatchConfig::default()` are asserted post-round-trip; test passes.

- [ ] **T-3 (S)** Add property (or fuzz) coverage for
  `score_relationships`/`score_tags` (Jaccard, added 0.8.0): properties
  for score-in-`[0,1]`, symmetry, identical-sets ⇒ `Some(1.0)`,
  disjoint-non-empty-sets ⇒ `Some(0.0)`, either-side-empty ⇒ `None` —
  today these are covered only by hand-written example tests
  (`matcher.rs` "relationships & tags" unit tests).
  *(verified: `grep -n "relationships_score\|tags_score" tests/property_tests.rs`
  returns no hits; only `src/matcher.rs`'s own example-based
  `#[test]`s exercise these two scorers.)*
  **Acceptance:** new `proptest!` cases in `tests/property_tests.rs`
  (or a new `fuzz/fuzz_targets/relationships_tags.rs`) exercise
  `score_relationships`/`score_tags` directly with arbitrary
  `Vec<RelationshipRef>`/`Vec<String>` inputs.

- [ ] **T-4 (S)** Add `deterministic_match` to the fuzz corpus: today
  `fuzz/fuzz_targets/match_events.rs` only calls
  `engine.match_events(&a, &b)` (the probabilistic path);
  `engine.deterministic_match(&a, &b)` — the other public infallible
  entry point per spec §8.6 — has no coverage-guided fuzzing at all.
  *(verified: `grep -n "deterministic_match" fuzz/fuzz_targets/*.rs`
  returns nothing; `fuzz/` has exactly 3 targets — `match_events.rs`,
  `normalizer.rs`, `scorer.rs` — none calling `deterministic_match`.)*
  **Acceptance:** `match_events.rs` (or a new target) additionally
  calls `engine.deterministic_match(&a, &b)` and
  `engine.deterministic_match(&b, &a)` on the same fuzzed pair,
  asserting no panic (mirrors the existing never-panic invariant
  pattern for the probabilistic path).

- [ ] **T-5 (S)** Clarify the crates.io publish status of the current
  0.8.0 line in `CHANGELOG.md`: neither the `[0.8.0]` nor
  `[Unreleased]` entries state whether 0.8.0 was published to
  crates.io, and `Cargo.toml` carries no `publish = false` marker
  either way, leaving the release state ambiguous to a reader.
  *(verified: `grep -n "crates.io\|publish" CHANGELOG.md` returns no
  hits in the `[0.8.0]`/`[Unreleased]` entries; `Cargo.toml`'s
  `version = "0.8.0"` carries no publish marker.)*
  **Acceptance:** `CHANGELOG.md`'s `[0.8.0]` entry (or a new
  `[Unreleased]` line) states explicitly whether/when 0.8.0 was
  published to crates.io, matching whatever `agents/release.md`'s
  checklist already requires be recorded.

---
