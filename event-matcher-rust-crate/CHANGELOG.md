# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [index.md](./index.md) (documentation map), [spec.md](./spec.md) (authoritative behaviour — each entry below corresponds to a section / FR / task in the spec), [README.md](./README.md) (user-facing overview).

## [Unreleased] → 0.5.0

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

Earlier history has moved to
[`CHANGELOG-archive.md`](./CHANGELOG-archive.md) to keep this file
under 40 KB.
