# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [index.md](./index.md) (documentation map), [spec.md](./spec/index.md) (authoritative behaviour — each entry below corresponds to a section / FR / task in the spec), [README.md](./README.md) (user-facing overview).

## [Unreleased]

### Security

- **SEC-M2 (High): empty-normalisation guard on the `name` + `postcode`
  deterministic short-circuit.** The `name_and_postcode_match` rule
  (`src/matcher.rs`) compared normalised name and postcode with no
  post-normalisation empty check, so two unrelated places whose name and
  postcode both normalise to the empty string (e.g. `name="."`,
  `postcode=" "`) satisfied `"" == ""` twice and short-circuited to a
  spurious `1.0` identity match. The rule now returns `false` when either
  the normalised name OR the normalised postcode is empty — a value that
  normalises to empty is not identity evidence. Same bug class as the
  person-matcher `passport_books_share_pair` fix. No weight/threshold or
  other behaviour change.

### Fixed

- Formatting drift in `src/matcher.rs` (one test's builder chain was not
  rustfmt-formatted); `cargo fmt --check` is clean again. No behaviour
  change.

### Added — `tags` weighted component (spec-first; implementation pending)

Spec added the **tags** match component
([spec §3.1 `tags` field + §3.1.1 row](./spec/03-data-model.md),
[§6.11 `tags_score`](./spec/06-per-field-scoring-algorithms.md),
[§7 `tags_weight` = 0.05](./spec/07-configuration.md), and
`MatchBreakdown::tags_score`). Tags are free-text operator labels scored
as a **supporting** set-Jaccard signal — identical in shape to how
`keywords` are scored where present. **Implementation is pending** — this
entry tracks the code follow-up:

- Add `tags: Vec<String>` to `Place` (default empty); add `tags(Vec<String>)`
  (replace) and `add_tag(impl Into<String>)` (append) to the `PlaceBuilder`.
- Implement `tags_score`: case-insensitively normalise (trim + ASCII
  lowercase) each side's tags into a set, dropping empty entries, then
  `|A ∩ B| / |A ∪ B|`; `None` when either side's set is empty (§6.11).
- Add `tags_weight` (default `0.05`) to `MatchConfig`; include `tags` in
  the renormalised weighted average; add `tags_score` to `MatchBreakdown`.
- Wire the service-side adapter (`to_matcher_place`) to route the service
  `tags` field (place-entity spec §5.3) + a bridge test.
- Unit tests for the Jaccard cases (disjoint, partial-overlap, identical,
  either-side-empty); `cargo test` +
  `cargo clippy --all-targets -- -D warnings` clean.

### Added — `setting` (indoor / outdoor) weighted component (spec-first; implementation pending)

Spec added the indoor/outdoor **setting** match component
([spec §3.1 field + `IndoorOutdoor` enum](./spec/03-data-model.md),
[§6.10 `setting_score`](./spec/06-per-field-scoring-algorithms.md),
[§7 `setting_weight` = 0.05](./spec/07-configuration.md), and
`MatchBreakdown::setting_score`). **Implementation is pending** — this entry
tracks the code follow-up:

- Add `setting: Option<IndoorOutdoor>` to `Place` and the `IndoorOutdoor`
  enum (`Indoor` / `Outdoor` / `Mixed`; `#[non_exhaustive]`); re-export from
  `lib.rs`; add to the `PlaceBuilder`.
- Implement `setting_score`: `1.0` equal, `0.5` Mixed-vs-(Indoor|Outdoor),
  `0.0` Indoor-vs-Outdoor, `None` when either side is absent (§6.10).
- Add `setting_weight` (default `0.05`) to `MatchConfig`; include `setting`
  in the renormalised weighted average; add `setting_score` to
  `MatchBreakdown`.
- Wire the service-side adapter (`to_matcher_place`) to route the service
  `setting` field (place-entity spec §5.3) + a bridge test.
- Unit tests for the four score cases; `cargo test` +
  `cargo clippy --all-targets -- -D warnings` clean.

## [0.6.1] — 2026-06-15

### Changed — documentation harmonisation pass

- Removed cross-domain residue inherited from the prior person/worker
  matcher domain. The `src/normalizer.rs` module doc and the
  `COUNTRY_PHONE_TABLE` doc comments no longer describe places as having
  "demographics" or justify the phone-country coverage by "national
  healthcare identifier"; they now match the spec §4.3.2 framing of the
  supported countries as place-data-source jurisdictions. The email
  normaliser doc no longer references "healthcare data".
- Corrected the `PlaceCategory` variant count in `AGENTS/testing.md`
  (35 unit variants + `Other` = 36 total) and reworded the adapter
  builder-surface description to the actual `Place` builder (name /
  alternate_names, coordinates, category, place_ids, address, phone /
  email, geographic fields) rather than person-domain "demographic /
  identifier" slots. Flagged the `se_personnummer` precedent anecdote as
  cross-crate history, not `place-matcher` behaviour.
- Updated the `place-matcher = "0.4"` dependency snippet in `README.md` /
  `index.md` to `"0.6"` to match the published-target version.
- Rewrote `CHANGELOG.md`: removed a duplicate `[Unreleased]` block that
  was verbatim person/worker-matcher history (national identifiers, blood
  type, passports, birth/death dates, DOB heuristic, middle name,
  nicknames, removed `identifiers.rs` / `nicknames.rs`) describing
  features that do not exist in the place-matcher and citing nonexistent
  spec sections.

### Added — tests

- New `#[test]` cases in `src/matcher.rs` pinning previously
  doctest-only / untested behaviour: `Confidence::from_score` degenerate
  inputs (NaN / negative → Low, `> 1.0` → High); the address line-1
  house-number blend (`0.6 × street + 0.4 × house-number-equality`, with
  street-only fallback when a house number is absent on either side);
  cross-country phone disambiguation (same NSN, UK vs FR, must not
  collide) and the E.164-preferred / legacy-fallback path;
  engine-level email scoring (`None` when unparseable vs `0.0` when both
  parse but differ); and the phonetic bonus gate (`> 0.9` firing lifts
  the renormalised score vs not firing).

No functional change to the library API.

## [0.6.0]

### Changed — `chrono` eliminated

- `chrono` (an unused manifest dependency flagged for removal) is gone;
  `place-matcher` carries no date dependency. See the Dependencies note
  below. No functional change.

### Added — adapter-contract test (CI guardrail for the public API)

- New `tests/adapter_contract.rs` (12 tests). Pins every public
  symbol downstream service adapters depend on: builder methods,
  `MatchingEngine::default_config` / `::new` / `match_*` /
  `deterministic_match` / `match_one_to_many`, `MatchResult` field
  shape (`score`, `is_match`, `confidence`, `breakdown`), the
  `MatchBreakdown` per-component fields the adapter inspects,
  `MatchConfig::strict` / `::default` / `::lenient` forming a
  monotonic threshold ladder, `Confidence::{High, Medium, Low}`,
  and `MatchResult` JSON round-trip.
- A rename or removal of any of the above breaks this test, failing
  the matcher's own CI **before** publish — making cross-crate
  breakage deliberate.

**Migration note.** 0.4.0 was the first release of `place-matcher` as a geographic place-matcher library. Prior 0.3.x releases targeted a different domain entirely. There is **no smooth upgrade path** from 0.3.x to the 0.4.x+ place-matcher line — every public type has different fields, every scoring component has different semantics, and the `MatchConfig` weight table has been replaced. Downstream code must be rewritten against the new surface; treat the upgrade as an integration project, not a version bump.

### Place model (current shape)

- `Place` fields: `name`, `alternate_names`, `latitude`, `longitude`, `category`, `place_ids`, `address`, `phone`, `email`, `local_id`, `altitude_as_metre`, `elevation_as_metre`, `area_as_metre_2`, `country_code_as_iso_3166_1_alpha_2`, `maximum_capacity_count`.
- `Place` is `#[non_exhaustive]`; construct via `Place::builder()`. The builder accepts `impl Into<String>` on every string setter.
- `Place::validate` requires only that `name` is set; otherwise returns `MatchingError::MissingField`.

### Address model

- `Address` fields: `line1`, `line2`, `city`, `county`, `postcode`, `country`. All `Option<String>`. `#[non_exhaustive]`. Fluent `with_*` setters.

### MatchBreakdown

- Fields: `name_score`, `name_phonetic_score`, `coordinates_score`, `address_score`, `category_score`, `country_code_score`, `place_ids_score`, `phone_score`, `email_score`. Each is `Option<f64>` in `[0.0, 1.0]`; `None` means the field did not participate.

### MatchConfig — weight table

- `name_weight` = `0.20`
- `coordinates_weight` = `0.30`, with `coordinates_scale_metres` = `50.0`
- `address_weight` = `0.10`
- `category_weight` = `0.10`
- `country_code_weight` = `0.05`
- `place_ids_weight` = `0.15`
- `phone_weight` = `0.03`
- `email_weight` = `0.02`
- Thresholds: default `0.80`, strict `0.95`, lenient `0.65`.
- `use_phonetic_matching` defaults to `false`; lenient preset turns it on. When on and the gating phonetic score exceeds `0.9`, a `0.05`-weighted bonus is added (never lowers the score).
- `phone_default_country` retained (defaults to `Some("GB")`).
- `gmail_dot_folding` retained (defaults to `false`).
- `strict_mode` retained; when true, `is_match` additionally requires `deterministic_match`.

### Deterministic match

- Any shared `(scheme, value)` pair in `place_ids`, OR identical normalised `name` plus identical normalised `address.postcode`.

### Added — geographic primitives and types

- `PlaceCategory` enum — coarse-grained category (Hotel, Restaurant, Cafe, Bar, Shop, Mall, Hospital, School, University, Library, Museum, Theatre, Cinema, Park, Beach, Stadium, Airport, RailwayStation, BusStation, Bank, PostOffice, Government, Monument, ReligiousBuilding, Cemetery, Mountain, Lake, River, City, Town, Village, Neighborhood, OfficeBuilding, Residence, Warehouse, plus `Other(String)`). `#[non_exhaustive]`.
- `PlaceId { scheme: PlaceIdScheme, value: String }` and `PlaceIdScheme` enum (Google, OsmNode, OsmWay, OsmRelation, GeoNames, Wikidata, Foursquare, Here, Mapbox, Other(String)). `#[non_exhaustive]`. `PlaceId::new` trims and rejects empty values.
- Geographic primitives on `Scorer`:
  - `Scorer::haversine_metres(lat1, lon1, lat2, lon2)` — great-circle distance in metres using Earth radius `6_371_000` m. Total over `f64`; handles equator and date-line crossings.
  - `Scorer::coordinates_score(distance_metres, scale_metres)` — Gaussian decay `exp(-(d/s)^2)` clamped to `[0.0, 1.0]`. Non-finite / non-positive scale / negative distance yields `0.0`.
- Geographic fields on `Place`: `latitude`, `longitude`, `altitude_as_metre`, `elevation_as_metre`, `area_as_metre_2`, `country_code_as_iso_3166_1_alpha_2`, `maximum_capacity_count`.
- `name + alternate_names` best-of cartesian product name scoring.
- `MatchingError` (`#[non_exhaustive]`, single variant `MissingField`) returned by `Place::validate` against the `name` requirement.

### Dependencies

- Retained: `serde`, `serde_json`, `unicode-normalization`, `strsim`, `thiserror`, `soundex`.

---

Earlier history (pre-0.4.0, including the prior non-place domain) is not
applicable to the geographic place-matcher and is intentionally omitted.
