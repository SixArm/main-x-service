# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [index.md](./index.md) (documentation map), [spec.md](./spec/index.md) (authoritative behaviour — each entry below corresponds to a section / FR / task in the spec), [README.md](./README.md) (user-facing overview).

## [Unreleased]


### Added — `tags` weighted component (spec-first; implementation pending)

Spec added the operator-applied **tags** match component
([§3 `Event.tags` field](./spec/03-data-model.md),
[§6.12 `tags_score`](./spec/06-per-field-scoring-algorithms.md),
[§7 `tags_weight` = 0.05](./spec/07-configuration.md), and
`MatchBreakdown::tags_score`). **Implementation is pending** — this
entry tracks the code follow-up:

- Add `tags: Vec<String>` to `Event` (default empty); add the `tags` /
  `add_tag` builder setters mirroring `keywords`.
- Implement `tags_score`: plain set Jaccard over case-insensitively
  normalised tags (trim + ASCII lowercase, empties dropped); `None`
  when either side empty (§6.12).
- Add `tags_weight` (default `0.05`) to `MatchConfig`; include `tags`
  in the renormalised weighted average; add `tags_score` to
  `MatchBreakdown`.
- Wire the service adapter (`to_matcher_event`) to route the service
  `tags` field (event-entity spec §5.3) + a bridge test.
- Unit tests (overlap Jaccard, empty-skip); `cargo test` +
  `cargo clippy --all-targets -- -D warnings` clean.

### Added — `relationships` weighted component (spec-first; implementation pending)

Spec added the typed event-to-event **relationships** match component
([§3 `Event.relationships` field + `RelationshipRef` / `RelationKind`](./spec/03-data-model.md),
[§6.11 `relationships_score`](./spec/06-per-field-scoring-algorithms.md),
[§7 `relationships_weight` = 0.05](./spec/07-configuration.md), and
`MatchBreakdown::relationships_score`). **Implementation is pending** — this
entry tracks the code follow-up:

- Add `relationships: Vec<RelationshipRef>` to `Event` + `RelationshipRef`
  / `RelationKind` (`Outer` / `Inner` / `ImmediatelyBefore` /
  `ImmediatelyAfter`; `#[non_exhaustive]`); re-export from `lib.rs`; add the
  `relationships` / `add_relationship` builder setters.
- Implement `relationships_score`: typed-set Jaccard over `(relation,
  event_id)` pairs; `None` when either side empty (§6.11).
- Add `relationships_weight` (default `0.05`) to `MatchConfig`; include
  `relationships` in the renormalised weighted average; add
  `relationships_score` to `MatchBreakdown`.
- Wire the service adapter (`to_matcher_event`) to route the service
  `relationships` field (event-entity spec §5.3) + a bridge test.
- Unit tests (kind-keyed agreement, empty-skip); `cargo test` +
  `cargo clippy --all-targets -- -D warnings` clean.

## [0.7.0] - 2026-08-24

### Changed — coordinate fields named for their units (BREAKING)

- **`Location::latitude` / `Location::longitude` renamed to
  `latitude_as_decimal_degrees` / `longitude_as_decimal_degrees`**,
  across the Rust struct, the `Location` builder setters
  (`with_latitude`/`with_longitude` keep their names but now set the
  renamed field), and the serde JSON wire format — there is no serde
  alias for the old keys, so this is a breaking change for any caller
  constructing a `Location` by struct literal (blocked by
  `#[non_exhaustive]`) or deserialising JSON that used the old field
  names.
- The companion to the place-matcher rename landing the same day (same
  commit, both matcher crates): the fields now say what unit they hold,
  so a reader cannot mistake degrees for radians without opening the
  spec.
- Version bumped `0.6.1` → `0.7.0` under semver: a public field rename
  is breaking.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

### Documentation — spec/AGENTS accuracy pass (DOC-3)

No behaviour change. Fixed two classes of documentation drift found during
the family-wide matcher-crate doc audit:

- **`agents/*.md` still described the pre-0.5.0 place-matcher domain.**
  `architecture.md`, `coding-style.md`, `matching-algorithm.md`,
  `normalization.md`, `release.md`, `security-and-privacy.md`, and
  `spec-driven-development.md` all still referenced `Place`, `PlaceBuilder`,
  `PlaceCategory`, `PlaceId`/`PlaceIdScheme`, `match_places`, coordinates-only
  scoring weights, and a fabricated `MatchingEngine::score_phone` /
  `phone_default_country` / `gmail_dot_folding` matcher path — none of which
  exist in the current `Event`-domain code (`Event` has no `phone` or
  `email` field at all; `normalize_phone*`/`normalize_email` are unused by
  `match_events` and remain library-only utilities). Rewritten against the
  actual `src/models.rs` / `src/matcher.rs` surface, including the
  correct default-weight table (name/start_date/end_date/location/
  category/country_code/event_ids/organizer/performers/url) and a note
  that this crate does **not** implement window-overlap temporal scoring
  (`spec.md` §10 OQ-C is still open).
- **`spec/03-data-model.md`, `spec/05-matching-pipeline.md`,
  `spec/06-per-field-scoring-algorithms.md`, and `spec/07-configuration.md`
  presented the planned `relationships`/`tags` fields, their
  `MatchBreakdown`/`MatchConfig` members, and §5.2.1's "eleven weighted
  components" as already-shipped fact** (`Event` carries 24 fields and
  `MatchBreakdown` 11 today, not 26/13) — contradicting this crate's own
  §9.3 "code wins on divergence" rule and the honest "implementation
  pending" framing already used elsewhere in this CHANGELOG. Added
  explicit "planned, not yet implemented" callouts rather than deleting
  the design; §6.11/§6.12 and the two `*_weight` config rows are otherwise
  unchanged, since they document the target design that CHANGELOG.md's
  own pending entries below track.

### Added — cargo-fuzz harness (SEC-I2)

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate with
  three coverage-guided libFuzzer targets, adopting the person-matcher
  reference scaffolding: `match_events` (deserialize a JSON `[event_a, event_b]`
  tuple → `MatchingEngine::match_events`; finite score in `[0,1]`, both orders),
  `normalizer` (the pure `Normalizer` helpers — name / postcode / phone / E.164 / address / phonetic / email / ISO-8601 — over arbitrary
  UTF-8, never-panic), and `scorer` (the pure `Scorer` similarities;
  finite in `[0,1]`). Run on nightly: `cargo +nightly fuzz run <target>`
  (see `fuzz/README.md`). The `fuzz/` crate is standalone (not a workspace
  member), so it never affects the crate’s normal stable build/test/clippy.
  Verified: `cargo +nightly fuzz build` compiles all three targets and
  short time-boxed campaigns run clean (millions of execs, no panics).

### Security

- **SEC-M2** — the deterministic `name_and_start_date_match` short-circuit
  now guards against empty normalised names: a `name` that normalises to an
  empty string (e.g. `"###"`, `"  "`) no longer satisfies the name leg, so
  two unrelated events sharing only an empty name plus a start_date can no
  longer deterministically match. Added a regression test.

### Fixed

- Formatting drift in `examples/location_matching.rs` and
  `tests/integration_tests.rs` (builder chains and long `println!`/
  `assert!` lines were not rustfmt-formatted); `cargo fmt --check` is
  clean again. No behaviour change.

## [0.6.1] - 2026-06-15

### Documentation — spec/doc harmonisation pass

- Fixed the install snippet in `README.md` / `index.md` to
  `event-matcher = "0.6"` (was `"0.4"`, which is the upgrade-incompatible
  place-matcher line per `spec/09-public-api-contract-semver.md`).
- Reconciled this CHANGELOG with `Cargo.toml` `version = "0.6.1"`: the
  0.5.0 place→event domain change now lives under its own dated [0.5.0]
  section, and [0.6.0] / [0.6.1] are released sections.
- Fixed `scripts/spec-drift-check.sh`: it previously grepped for a
  non-existent top-level `spec.md`; it now matches any path under
  `spec/`, and the watched source pattern was widened from `src/matcher.rs`
  to also cover `src/scorer.rs`, `src/normalizer.rs`, `src/models.rs`.
- Added integration tests pinning the coordinates location sub-score, the
  all-empty-`Location` neutral-`0.5` fallback, the house-number-weighted
  line-1 address blend, the phonetic-bonus direction (nudges up, never
  down), and the §11.1 Glastonbury renormalised score (`≈ 0.976`).
- Added a `location_matching` example demonstrating the coordinates path
  and strict-mode rejection (§11.3).

## [0.6.0] - 2026-06-10

### Changed — version-aligned with the matcher-family `chrono` elimination

- Bumped to 0.6.0. `event-matcher` carries no date dependency and
  `chrono` is not in the manifest. No functional change.

### Added — adapter-contract test (CI guardrail for the public API)

- New `tests/adapter_contract.rs` (14 tests). Pins every public
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
  breakage deliberate. Precedent: worker-matcher 0.3.0 renamed
  `se_personnummer` → `se_workernummer`, silently breaking
  `person-service`; the contract test would have caught that.
- Documented in `agents/testing.md` and `index.md` (Common Tasks
  table) and cross-referenced from `spec.md` (§18.5 for person /
  worker — full §1–§25 shape; §9 callout for place / thing / event —
  shorter §1–§13 shape).

## [0.5.0] - 2026-06-09

**Domain change.** 0.5.0 repurposes the crate from geographic *place* matching to **event matcher** modelled on [schema.org/Event](https://schema.org/Event). Prior 0.4.x releases matched landmarks, natural features, chain branches, and administrative areas; 0.5.0 matches festivals, conferences, concerts, sports fixtures, screenings, hackathons, meetups, theatre runs, and other instances of `schema:Event`. There is **no smooth upgrade path** from 0.4.x. Every public type, scoring component, and `MatchConfig` weight has been renamed or replaced. Pin to `0.4.x` for the place-matcher behaviour; treat the upgrade as an integration project against the new surface.

### Breaking — crate name
- Renamed from `Event-matcher` (non-conventional capital `E`) to `event-matcher` (Rust snake-case convention). The import path becomes `event_matcher::…`.

### Breaking — primary model
- `Place` → `Event`. `PlaceBuilder` → `EventBuilder`. The `Event` shape mirrors `schema:Event` field for field; see the README "Event model" table for the full mapping.
- `PlaceCategory` → `EventCategory`. Variants now enumerate the direct schema.org/Event subtypes (`BusinessEvent`, `ChildrensEvent`, `ComedyEvent`, `ConferenceEvent`, `CourseInstance`, `DanceEvent`, `DeliveryEvent`, `EducationEvent`, `EventSeries`, `ExhibitionEvent`, `Festival`, `FoodEvent`, `Hackathon`, `LiteraryEvent`, `MusicEvent`, `PerformingArtsEvent`, `PublicationEvent`, `SaleEvent`, `ScreeningEvent`, `SocialEvent`, `SportsEvent`, `TheaterEvent`, `VisualArtsEvent`, `Other(String)`).
- `PlaceId` → `EventId`. `PlaceIdScheme` → `EventIdScheme` with variants `Wikidata`, `Eventbrite`, `Meetup`, `Ticketmaster`, `Songkick`, `Bandsintown`, `Facebook`, `Luma`, `GoogleCalendar`, `ICalendarUid`, `Other(String)`.
- New enums `EventStatus` (`schema:EventStatusType`) and `EventAttendanceMode` (`schema:EventAttendanceModeEnumeration`).
- New `Location` struct gathers `venue_name`, `address`, `latitude`, `longitude`, and `virtual_url` — modelling the four `schema:Event.location` flavours (`Place` / `PostalAddress` / `Text` / `VirtualLocation`) inside a single field.

### Breaking — `Event` fields vs. former `Place`
- **Removed (not schema.org/Event properties):** top-level `latitude`/`longitude` (moved into `Location`), `phone`, `email`, `altitude_as_metre`, `elevation_as_metre`, `area_as_metre_2`.
- **Renamed:** `place_ids` → `event_ids`. `maximum_capacity_count` → `maximum_attendee_capacity`.
- **Added (schema.org/Event properties):** `description`, `url`, `keywords`, `in_language`, `typical_age_range`, `start_date`, `end_date`, `door_time`, `previous_start_date`, `event_status`, `event_attendance_mode`, `location`, `organizer`, `performers`, `maximum_physical_attendee_capacity`, `maximum_virtual_attendee_capacity`, `is_accessible_for_free`, `super_event_id`.
- Retained: `name`, `alternate_names`, `country_code_as_iso_3166_1_alpha_2`, `local_id`.

### Breaking — engine API
- `MatchingEngine::match_places` → `MatchingEngine::match_events`. `match_one_to_many` and `rank_one_to_many` take `&Event` / `&[Event]`.
- `deterministic_match` now returns `true` iff the events share any external `EventId` pair, OR both have a normalised name equality AND a `start_date` that parses to the same Unix instant. The previous `name + postcode` rule is removed.

### Breaking — `MatchConfig` weight table
- Removed: `coordinates_weight`, `coordinates_scale_metres`-as-primary, `address_weight`, `place_ids_weight`, `phone_weight`, `email_weight`, `gmail_dot_folding`, `phone_default_country`.
- Added: `start_date_weight` (`0.25`), `start_date_scale_seconds` (`3600.0`), `end_date_weight` (`0.05`), `location_weight` (`0.15`), `coordinates_scale_metres` (`100.0`, now scoped to the location sub-score), `event_ids_weight` (`0.15`), `organizer_weight` (`0.04`), `performers_weight` (`0.02`), `url_weight` (`0.02`).
- Adjusted: `name_weight` (`0.20`, unchanged), `category_weight` (`0.10` → `0.08`), `country_code_weight` (`0.05` → `0.04`).

### Breaking — `MatchBreakdown` fields
- Removed: `coordinates_score`, `address_score`, `place_ids_score`, `phone_score`, `email_score`.
- Added: `start_date_score`, `end_date_score`, `location_score`, `event_ids_score`, `organizer_score`, `performers_score`, `url_score`.
- Retained: `name_score`, `name_phonetic_score`, `category_score`, `country_code_score`.

### Added — scorer / normaliser
- `Scorer::seconds_between(t1, t2)` — absolute difference in seconds between two ISO 8601 timestamps. Returns `None` on unparseable input.
- `Scorer::start_date_score(diff_seconds, scale_seconds)` — Gaussian-decay similarity for temporal differences. Same shape as `Scorer::coordinates_score`.
- `Normalizer::parse_iso8601_unix_seconds(s)` — total parser over ISO 8601 / RFC 3339 date and date-time strings. Accepts `YYYY-MM-DD`, `…T…Z`, `…±HH:MM`, fractional seconds. Returns `None` on out-of-range or malformed input.

### Retained from 0.4.x
- `Address`, `Normalizer` (string and address utilities), `Scorer` (Jaro-Winkler, Levenshtein, Combined, Haversine), `SimilarityAlgorithm`, `Confidence` bands, `MatchingError`, `Result`, and all serde round-trip guarantees.
- `#![forbid(unsafe_code)]`, panic-free library code, `Send + Sync` engine, weight-renormalised scoring that skips missing fields without penalty.

---

Earlier history has moved to keep this file under 40 KB:

- [`CHANGELOG-archive.md`](./CHANGELOG-archive.md) — released
  place-matcher versions ([0.3.0], [0.1.0]) and the [0.4.0] inherited
  history.
- [`CHANGELOG-pre-event-rebrand.md`](./CHANGELOG-pre-event-rebrand.md)
  — pre-0.5.0 "Unreleased" entries authored under the
  `place-matcher` name that never shipped.
