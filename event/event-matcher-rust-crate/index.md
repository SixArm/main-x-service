# Event matcher Rust Crate

A Rust library for deciding whether two records describe the same event, modelled on [schema.org/Event](https://schema.org/Event).

> **Documentation index:** [`index.md`](./index.md) is the top-level map of every doc in this repo (spec, AGENTS guides, CHANGELOG, examples). Start there if you're new.

## What it does

`event-matcher` compares pairs of [`Event`] records — festivals, conferences, concerts, sports fixtures, screenings, conferences, hackathons, meetups, theatre runs — and tells you whether they refer to the same event. It is built for de-duplication and record linkage across event-data sources that disagree on titles, formatting, schedules, venues, and identifier schemes.

The crate provides two strategies behind one engine:

- **Deterministic** — a hard `bool` from a shared external event ID (Eventbrite, Meetup, Ticketmaster, Songkick, Wikidata, …) or from an identical normalised name plus an identical `start_date` instant.
- **Probabilistic** — a weight-renormalised score in `[0.0, 1.0]` over name, start/end date (Gaussian decay over absolute seconds difference), location (venue, address, coordinates), category, country code, external IDs, organiser, performers, and URL, with a per-field [`MatchBreakdown`] so every decision is auditable.

The library is pure: no IO, no clocks, no RNGs, `#![forbid(unsafe_code)]`, `Send + Sync`. It is suitable as a leaf dependency under web servers, batch jobs, or notebooks.

## Installation

```toml
[dependencies]
event-matcher = "0.6"
```

## Quick start — probabilistic match

```rust
use event_matcher::{MatchingEngine, Event};

let a = Event::builder()
    .name("Glastonbury Festival 2024")
    .start_date("2024-06-26T09:00:00Z")
    .end_date("2024-06-30T23:59:00Z")
    .build();

let b = Event::builder()
    .name("Glasto 2024")
    .add_alternate_name("Glastonbury Festival 2024")
    .start_date("2024-06-26T09:15:00Z")
    .end_date("2024-06-30T23:59:00Z")
    .build();

let engine = MatchingEngine::default_config();
let result = engine.match_events(&a, &b);

assert!(result.is_match);
println!("score      = {:.2}", result.score);
println!("name       = {:?}", result.breakdown.name_score);
println!("start_date = {:?}", result.breakdown.start_date_score);
```

The `MatchBreakdown` carries one `Option<f64>` per scored field; a `None` means the field was absent on at least one side and so did not contribute. Missing fields neither raise nor lower the score.

## Quick start — deterministic match

A shared external event ID (any `(scheme, value)` pair across the two records) is enough on its own:

```rust
use event_matcher::{MatchingEngine, Event, EventId, EventIdScheme};

let id = EventId::new(EventIdScheme::Eventbrite, "123456789").unwrap();

let a = Event::builder().name("RustConf 2024").add_event_id(id.clone()).build();
let b = Event::builder().name("RC 2024").add_event_id(id).build();

let engine = MatchingEngine::default_config();
assert!(engine.deterministic_match(&a, &b));
```

Identical normalised name plus a `start_date` that parses to the same instant is also accepted as a deterministic match (useful when no shared external ID is available, and naturally tolerant of equivalent ISO 8601 offsets such as `2024-09-10T09:00:00Z` and `2024-09-10T11:00:00+02:00`).

## The `Event` model

Field names use Rust conventions but the semantics match the schema.org/Event properties one for one.

| Field | Type | Schema.org property |
|---|---|---|
| `name` | `Option<String>` | `schema:name` |
| `alternate_names` | `Vec<String>` | `schema:alternateName` |
| `description` | `Option<String>` | `schema:description` |
| `url` | `Option<String>` | `schema:url` |
| `event_ids` | `Vec<EventId>` | `schema:identifier` |
| `category` | `Option<EventCategory>` | direct subtype of `schema:Event` |
| `keywords` | `Vec<String>` | `schema:keywords` |
| `in_language` | `Option<String>` | `schema:inLanguage` (BCP 47) |
| `typical_age_range` | `Option<String>` | `schema:typicalAgeRange` |
| `start_date` | `Option<String>` | `schema:startDate` (ISO 8601) |
| `end_date` | `Option<String>` | `schema:endDate` (ISO 8601) |
| `door_time` | `Option<String>` | `schema:doorTime` |
| `previous_start_date` | `Option<String>` | `schema:previousStartDate` |
| `event_status` | `Option<EventStatus>` | `schema:eventStatus` |
| `event_attendance_mode` | `Option<EventAttendanceMode>` | `schema:eventAttendanceMode` |
| `location` | `Option<Location>` | `schema:location` (venue + address + geo + virtual URL) |
| `country_code_as_iso_3166_1_alpha_2` | `Option<String>` | (convenience; pairs with `Location.address.country`) |
| `organizer` | `Option<String>` | `schema:organizer` |
| `performers` | `Vec<String>` | `schema:performer` |
| `maximum_attendee_capacity` | `Option<u32>` | `schema:maximumAttendeeCapacity` |
| `maximum_physical_attendee_capacity` | `Option<u32>` | `schema:maximumPhysicalAttendeeCapacity` |
| `maximum_virtual_attendee_capacity` | `Option<u32>` | `schema:maximumVirtualAttendeeCapacity` |
| `is_accessible_for_free` | `Option<bool>` | `schema:isAccessibleForFree` |
| `super_event_id` | `Option<String>` | `schema:superEvent` (id only) |
| `local_id` | `Option<String>` | (originating-system id; not scored) |

`EventCategory` enumerates the direct schema.org/Event subtypes — `BusinessEvent`, `ChildrensEvent`, `ComedyEvent`, `ConferenceEvent`, `CourseInstance`, `DanceEvent`, `DeliveryEvent`, `EducationEvent`, `EventSeries`, `ExhibitionEvent`, `Festival`, `FoodEvent`, `Hackathon`, `LiteraryEvent`, `MusicEvent`, `PerformingArtsEvent`, `PublicationEvent`, `SaleEvent`, `ScreeningEvent`, `SocialEvent`, `SportsEvent`, `TheaterEvent`, `VisualArtsEvent`, and a catch-all `Other(String)`.

`EventStatus` mirrors `schema:EventStatusType` (`EventScheduled`, `EventCancelled`, `EventPostponed`, `EventRescheduled`, `EventMovedOnline`); `EventAttendanceMode` mirrors `schema:EventAttendanceModeEnumeration` (`OfflineEventAttendanceMode`, `OnlineEventAttendanceMode`, `MixedEventAttendanceMode`).

Build records via the fluent [`Event::builder`]; all setters accept `impl Into<String>`.

## The match pipeline

1. Each scoring component yields `Some(score)` in `[0.0, 1.0]` or `None` (missing on at least one side).
2. The weighted sum runs over components that scored.
3. The sum of participating weights divides through — **renormalisation** ensures missing fields do not penalise.
4. A phonetic-name match (Soundex on normalised names) adds a `0.05`-weighted bonus when the gating phonetic score exceeds `0.9`, only when `use_phonetic_matching` is on. The bonus never lowers a score.
5. `is_match = score >= match_threshold` (strict mode additionally requires `deterministic_match`).
6. `confidence = Confidence::from_score(score)` — bands are fixed (`>= 0.90` High, `>= 0.75` Medium, else Low) and **independent** of `match_threshold`.

### Default weights

| Component | Weight | Notes |
|---|---|---|
| Name | `0.20` | Best of cartesian product across primary + alternates, via the configured `SimilarityAlgorithm` (default `Combined` = 0.7 × Jaro-Winkler + 0.3 × Levenshtein). |
| Start date | `0.25` | Gaussian decay `exp(-(d/s)^2)` over the absolute seconds difference. Default scale `s = 3600` s (one hour). |
| End date | `0.05` | Same Gaussian shape as `start_date`, same scale. |
| Location | `0.15` | Coordinates `0.5`, address `0.3`, venue name `0.15`, virtual URL `0.05` — weight-renormalised across populated sub-components. Coordinates use a Gaussian decay with default scale `100` m. |
| Category | `0.08` | `1.0` if equal, `0.0` if both set and differ, `None` if either missing. |
| Country code | `0.04` | Case-insensitive equality after trim. |
| Event IDs | `0.15` | `1.0` if any `(scheme, value)` pair is shared, `0.0` if both non-empty but no overlap, `None` if either empty. |
| Organiser | `0.04` | Combined string similarity after name normalisation. |
| Performers | `0.02` | Best-of cartesian product across performer lists. |
| URL | `0.02` | Exact equality after trim. |
| Phonetic bonus | `+0.05` when gated | Bonus only — never lowers a score. |

## Configuration presets

```rust
use event_matcher::{MatchConfig, MatchingEngine};

let strict  = MatchingEngine::new(MatchConfig::strict());  // threshold 0.95, requires deterministic
let default = MatchingEngine::default_config();            // threshold 0.80
let lenient = MatchingEngine::new(MatchConfig::lenient()); // threshold 0.65, phonetic on
```

- **Default (0.80)** — balanced for everyday de-duplication.
- **Strict (0.95)** — for downstream systems that must rely on the answer; `is_match` additionally requires `deterministic_match`. `score` and `confidence` are unaffected.
- **Lenient (0.65)** — for triaging large candidate sets where false negatives are costlier than false positives.

Every field of `MatchConfig` is overridable; the config is `Serialize + Deserialize` (with `#[serde(default)]`) so tunings can live in a file:

```rust
use event_matcher::MatchConfig;

let cfg: MatchConfig = serde_json::from_str(r#"{
    "match_threshold": 0.85,
    "start_date_scale_seconds": 600.0
}"#).unwrap();
```

## Batch scoring

```rust
use event_matcher::{MatchingEngine, Event};

let engine = MatchingEngine::default_config();
let query = Event::builder().name("RustConf 2024").build();
let candidates = vec![
    Event::builder().name("PyConf 2024").build(),
    Event::builder().name("RustConf 2024").build(),
    Event::builder().name("GoConf 2024").build(),
];

// Parallel to the input slice:
let results = engine.match_one_to_many(&query, &candidates);

// Sorted by descending score, deterministic tiebreak on original index:
let ranked = engine.rank_one_to_many(&query, &candidates);
let (best_idx, best) = &ranked[0];
println!("best is candidate[{best_idx}] with score {:.2}", best.score);
```

The engine is `Send + Sync`. Wrap calls in `rayon::par_iter` (or any parallelism primitive) without changes to this crate. Candidate pre-filtering — Soundex prefix blocking, country-code blocking, year-bucket blocking — is intentionally a consumer concern.

## Temporal and geographic primitives

`Scorer` exposes the helpers the engine uses internally:

```rust
use event_matcher::Scorer;

// Temporal proximity.
let secs = Scorer::seconds_between("2024-06-26T09:00:00Z", "2024-06-26T10:30:00Z").unwrap();
let t    = Scorer::start_date_score(secs as f64, 3600.0);
println!("90-minute gap @ scale=1 h: {t:.6}");

// Geographic proximity (used inside `location` matching).
let d = Scorer::haversine_metres(51.507_22, -0.127_5, 48.853_0, 2.349_2);
let s = Scorer::coordinates_score(d, 100.0);
println!("London-Paris: {:.1} km, score @ scale=100 m: {s:.6}", d / 1000.0);
```

`Scorer::seconds_between` parses both inputs via `Normalizer::parse_iso8601_unix_seconds` (supporting `YYYY-MM-DD`, `…T…Z`, `…±HH:MM`, fractional seconds) and returns `None` on any unparseable input. The Gaussian scorers return `exp(-(d/s)^2)` clamped to `[0.0, 1.0]`; pathological inputs (negative distance, non-positive scale, non-finite) return `0.0`.

## Determinism and safety

- **`#![forbid(unsafe_code)]`** at the crate root.
- **No IO.** The library does not read files, open sockets, or log.
- **No clocks, no RNGs, no environment variables.** Same inputs always produce the same outputs.
- **No panics** in library code; every fallible parser returns `None` and every fallible operation returns `Result`.
- **`Send + Sync`.** Engines are immutable after construction and cheap to clone.
- **Serde-clean.** Every public data type round-trips through `serde_json` (and any other `serde` format).

## Limitations / out of scope

- **Not a calendar engine.** This crate does not produce recurrence expansions for `schema:eventSchedule` / `schema:Schedule`. Feed concrete instances in if you need to compare them.
- **Not a geocoder or router.** Distances are great-circle (Haversine) only; addresses are not resolved.
- **No machine learning.** Scoring is rule-based and transparent; weights are tuneable but the algorithm is fixed.
- **No persistence layer.** The crate scores pairs in memory; storage and indexing belong upstream.
- **English-only abbreviation table** for street types (`St`, `Rd`, `Ave`, …). Locale-aware vocabularies are an Open Question.

## License

MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause — see [`LICENSE.md`](./LICENSE.md).

## Contributing

Contributions welcome. Before opening a PR:

- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

See [`AGENTS.md`](./AGENTS.md) for the working guide and [`spec.md`](./spec/index.md) for the authoritative behaviour spec.

## Contact

Joel Parker Henderson — <joel@joelparkerhenderson.com>.
