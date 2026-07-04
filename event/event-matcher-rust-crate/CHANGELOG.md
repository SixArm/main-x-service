# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [index.md](./index.md) (documentation map), [spec.md](./spec/index.md) (authoritative behaviour — each entry below corresponds to a section / FR / task in the spec), [README.md](./README.md) (user-facing overview).

## [Unreleased]

### Fixed

- Formatting drift in `examples/location_matching.rs` and
  `tests/integration_tests.rs` (builder chains and long `println!`/
  `assert!` lines were not rustfmt-formatted); `cargo fmt --check` is
  clean again. No behaviour change.

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
- Documented in `AGENTS/testing.md` and `index.md` (Common Tasks
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
