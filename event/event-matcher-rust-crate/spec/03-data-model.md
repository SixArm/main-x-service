## 3. Data model

All public types are `#[derive(Serialize, Deserialize)]` and round-trip through `serde_json`. Public surface lives in `src/lib.rs` re-exports.

### 3.1 `Event`

```rust
#[non_exhaustive]
pub struct Event {
    pub name: Option<String>,
    pub alternate_names: Vec<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub event_ids: Vec<EventId>,
    pub local_id: Option<String>,
    pub category: Option<EventCategory>,
    pub keywords: Vec<String>,
    pub in_language: Option<String>,
    pub typical_age_range: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub door_time: Option<String>,
    pub previous_start_date: Option<String>,
    pub event_status: Option<EventStatus>,
    pub event_attendance_mode: Option<EventAttendanceMode>,
    pub location: Option<Location>,
    pub country_code_as_iso_3166_1_alpha_2: Option<String>,
    pub organizer: Option<String>,
    pub performers: Vec<String>,
    pub maximum_attendee_capacity: Option<u32>,
    pub maximum_physical_attendee_capacity: Option<u32>,
    pub maximum_virtual_attendee_capacity: Option<u32>,
    pub is_accessible_for_free: Option<bool>,
    pub super_event_id: Option<String>,
    pub relationships: Vec<RelationshipRef>,
    pub tags: Vec<String>,
}
```

`Event` carries **26 fields** today, every one optional or defaulting to empty. Field names use Rust conventions but map one-for-one onto schema.org properties (`name` → `schema:name`, `event_ids` → `schema:identifier`, `start_date` → `schema:startDate`, `organizer` → `schema:organizer`, `performers` → `schema:performer`, `super_event_id` → `schema:superEvent`, …); the full mapping table lives in the `src/models.rs` doc comments and the README. `relationships` and `tags` have no schema.org counterpart — they are internal, consuming-registry-scoped matching signals (§3.1.1).

- `Event` MUST be constructed via `Event::builder()` from outside the crate (`#[non_exhaustive]`).
- `Event::validate(&self) -> Result<()>` MUST return `Ok(())` when `name` is set and `Err(MatchingError::MissingField(_))` otherwise. Validation is not invoked automatically by the matcher.
- `Event` MUST be `Send + Sync` and serde-round-trippable.

#### 3.1.1 Field semantics

Scored fields (probabilistic matcher consults — §6): `name`, `alternate_names` (§6.1); `start_date`, `end_date` (§6.3, ISO 8601 date or date-time strings — `"2024-06-26"`, `"2024-06-26T09:00:00Z"`, `"2024-06-26T09:00:00+01:00"`); `location` (§6.4, weighted blend of coordinates, address, venue name, virtual URL); `category` (§6.5); `country_code_as_iso_3166_1_alpha_2` (§6.6, ISO 3166-1 alpha-2, compared case-insensitively after trim); `event_ids` (§6.7, sharing any `(scheme, value)` pair is a deterministic match); `organizer` (§6.8); `performers` (§6.9); `url` (§6.10, exact equality after trim); `relationships` (§6.11); `tags` (§6.12).

```rust
#[non_exhaustive]
pub enum RelationKind { Outer, Inner, ImmediatelyBefore, ImmediatelyAfter }

pub struct RelationshipRef {
    pub relation: RelationKind,
    pub event_id: String,
}
```

`relationships: Vec<RelationshipRef>` holds typed event-to-event references — `RelationshipRef { relation: RelationKind, event_id: String }` where `RelationKind` is a `#[non_exhaustive]` enum mirroring the consuming service: `Outer` / `Inner` (containment) and `ImmediatelyBefore` / `ImmediatelyAfter` (temporal adjacency). `event_id` is an opaque registry id (whitespace-trimmed, non-empty, enforced by `RelationshipRef::new`); the matcher does not resolve, invert, or transitively close the references — it compares the two events' relationship **sets** (§6.11). A supporting signal, not an identifying field on its own; not consulted by `deterministic_match`.

`tags: Vec<String>` holds short, free-text operational labels an operator attached to the record for grouping / filtering / triage / workflow (e.g. `"vip"`, `"review"`, `"fast-track"`); stored verbatim (no trimming or de-duplication at construction — normalisation happens at scoring time, §6.12), defaulting to empty. Distinct from `keywords` (descriptive / discovery terms about *what the record is*): tags are user-applied operational labels. Scores as a plain set Jaccard over the case-insensitively normalised tag sets (§6.12); a **supporting** signal, not an identifying field on its own; not consulted by `deterministic_match`.

Data-only fields (round-trip honesty; **not** scored): `description`; `local_id` (originating-system identifier — sources may collide); `keywords`; `in_language` (IETF BCP-47 tag, e.g. `"en-GB"`); `typical_age_range`; `door_time`; `previous_start_date`; `event_status` (see §3.6); `event_attendance_mode` (see §3.7); `maximum_attendee_capacity`, `maximum_physical_attendee_capacity`, `maximum_virtual_attendee_capacity` (`0` is meaningful, absence is `None`); `is_accessible_for_free`; `super_event_id`.

Date fields that fail to parse as ISO 8601 MUST be stored as supplied (round-trip honesty) but MUST be treated as missing by the temporal scorers (§6.3). Coordinates inside `location` that are non-finite or fall outside the conventional ranges are likewise stored as supplied but treated as missing by the coordinates sub-scorer (§6.4). JSON shape is the direct serde derivation of `Event`; see `examples/basic_usage.rs` for a populated payload.

### 3.2 `EventBuilder`

```rust
#[derive(Default)]
pub struct EventBuilder { /* private fields */ }
```

Fluent builder for `Event`. All string setters accept `impl Into<String>`. Setters mirror `Event`'s fields one-for-one. The list setters `alternate_names(Vec<String>)`, `event_ids(Vec<EventId>)`, `keywords(Vec<String>)`, `performers(Vec<String>)`, `relationships(Vec<RelationshipRef>)`, and `tags(Vec<String>)` **replace** the entire list; `add_alternate_name(impl Into<String>)`, `add_event_id(EventId)`, `add_keyword(impl Into<String>)`, `add_performer(impl Into<String>)`, `add_relationship(RelationshipRef)`, and `add_tag(impl Into<String>)` **append** a single element. `build() -> Event` consumes the builder.

`EventBuilder` is `#[derive(Default)]`. All fields start unset (`None` / empty `Vec`).

### 3.3 `Location`

```rust
#[non_exhaustive]
pub struct Location {
    pub venue_name: Option<String>,
    pub address: Option<Address>,
    pub latitude_as_decimal_degrees: Option<f64>,
    pub longitude_as_decimal_degrees: Option<f64>,
    pub virtual_url: Option<String>,
}
```

Corresponds to `schema:Event.location`, which schema.org allows to be a `Place`, `PostalAddress`, `Text`, or `VirtualLocation`. One struct models all four flavours: `venue_name` (`Place.name`), `address` (`PostalAddress`), `latitude` / `longitude` (`Place.geo`, decimal degrees, conventionally `[-90, 90]` / `[-180, 180]`), and `virtual_url` (`VirtualLocation.url`, compared by exact equality after trim).

Constructed via `Location::new()` (equivalent to `Location::default()`) plus fluent `with_*` setters (`with_venue_name`, `with_address`, `with_latitude`, `with_longitude`, `with_virtual_url`). Every field is optional; a `Location` with all fields `None` still participates in scoring (the blend degrades to the neutral `0.5` — §6.4), whereas an `Event.location` of `None` on either side skips the component entirely.

### 3.4 `Address`

```rust
#[non_exhaustive]
pub struct Address {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub county: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<String>,
}
```

Constructed via `Address::new()` (equivalent to `Address::default()`) plus fluent `with_*` setters (`with_line1`, `with_line2`, `with_city`, `with_county`, `with_postcode`, `with_country`). All six fields are optional so partial addresses are first-class — a record with only a postcode is still useful for matching.

Address comparison (the address sub-score of §6.4) consults `postcode`, `city`, and `line1`. `line2`, `county`, and `country` are stored but not currently scored.

### 3.5 `EventCategory`

```rust
#[non_exhaustive]
pub enum EventCategory {
    BusinessEvent, ChildrensEvent, ComedyEvent, ConferenceEvent,
    CourseInstance, DanceEvent, DeliveryEvent, EducationEvent,
    EventSeries, ExhibitionEvent, Festival, FoodEvent, Hackathon,
    LiteraryEvent, MusicEvent, PerformingArtsEvent, PublicationEvent,
    SaleEvent, ScreeningEvent, SocialEvent, SportsEvent, TheaterEvent,
    VisualArtsEvent, Other(String),
}
```

`EventCategory` carries **23 unit variants plus one carrying `Other(String)`** (24 in total). The unit variants mirror the direct subtypes of `schema:Event` (full list with doc comments in source: `src/models.rs`). Equality is structural: `EventCategory::Other("foo")` matches `EventCategory::Other("foo")` but not `EventCategory::Other("bar")` and not any enumerated variant. Adding a new variant is non-breaking (`#[non_exhaustive]`).

### 3.6 `EventStatus`

```rust
#[non_exhaustive]
pub enum EventStatus {
    EventScheduled, EventCancelled, EventPostponed,
    EventRescheduled, EventMovedOnline,
}
```

The five enumeration values of `schema:EventStatusType`. Carried as data; **not** scored — a cancelled listing and its scheduled twin still describe the same event.

### 3.7 `EventAttendanceMode`

```rust
#[non_exhaustive]
pub enum EventAttendanceMode {
    OfflineEventAttendanceMode, OnlineEventAttendanceMode,
    MixedEventAttendanceMode,
}
```

The three enumeration values of `schema:EventAttendanceModeEnumeration` (physical / virtual / hybrid). Carried as data; **not** scored.

### 3.8 `EventId` and `EventIdScheme`

```rust
#[non_exhaustive]
pub enum EventIdScheme {
    Wikidata, Eventbrite, Meetup, Ticketmaster, Songkick,
    Bandsintown, Facebook, Luma, GoogleCalendar, ICalendarUid,
    Other(String),
}

pub struct EventId {
    pub scheme: EventIdScheme,
    pub value: String,
}
```

`EventIdScheme` carries **10 unit variants plus `Other(String)`** (11 in total): `Wikidata`, `Eventbrite`, `Meetup`, `Ticketmaster`, `Songkick`, `Bandsintown`, `Facebook`, `Luma`, `GoogleCalendar`, `ICalendarUid` (RFC 5545 `UID`). Per-scheme format conventions (informational) are documented in `src/models.rs`.

- `EventId::new(scheme, value) -> Option<Self>` MUST trim surrounding whitespace from `value` and MUST return `None` if the trimmed value is empty.
- Equality MUST be `(scheme, value)`-structural. No per-scheme canonicalisation is performed.
- Cross-scheme identifiers MUST NEVER match: an `(Eventbrite, "abc")` and a `(Meetup, "abc")` denote different things.

### 3.9 `Confidence`

```rust
pub enum Confidence { High, Medium, Low }
```

`Confidence::from_score(score)` bands:

| Score range | Band |
|---|---|
| `score >= 0.90` | `High` |
| `0.75 <= score < 0.90` | `Medium` |
| `score < 0.75` | `Low` |

Boundary semantics: the lower bound of each band is **inclusive**. The function is total over `f64`: NaN → `Low`, negative → `Low`, `> 1.0` → `High`. Bands are independent of `MatchConfig::match_threshold`.

### 3.10 `MatchResult` and `MatchBreakdown`

```rust
pub struct MatchResult {
    pub score: f64,
    pub is_match: bool,
    pub confidence: Confidence,
    pub breakdown: MatchBreakdown,
}

pub struct MatchBreakdown {
    pub name_score: Option<f64>,
    pub name_phonetic_score: Option<f64>,
    pub start_date_score: Option<f64>,
    pub end_date_score: Option<f64>,
    pub location_score: Option<f64>,
    pub category_score: Option<f64>,
    pub country_code_score: Option<f64>,
    pub event_ids_score: Option<f64>,
    pub organizer_score: Option<f64>,
    pub performers_score: Option<f64>,
    pub url_score: Option<f64>,
    pub relationships_score: Option<f64>,
    pub tags_score: Option<f64>,
}
```

`MatchBreakdown` carries **13 score fields** today, each `Option<f64>` in `[0.0, 1.0]`. `None` means "the field did not participate in the weighted sum"; `Some(s)` carries the per-field score. `MatchResult::confidence` carries `#[serde(default = "default_confidence")]` where `default_confidence()` returns `Confidence::Low`, so legacy payloads predating the field round-trip as `Low`. `relationships_score` and `tags_score` carry `#[serde(default)]` so legacy payloads predating them still deserialise, defaulting to `None`.

### 3.11 `MatchConfig`

See §7 for every knob, default, and preset.

### 3.12 `MatchingError` and `Result`

```rust
#[non_exhaustive]
pub enum MatchingError {
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, MatchingError>;
```

- The matching engine is **infallible**: `match_events`, `deterministic_match`, `match_one_to_many`, and `rank_one_to_many` cannot fail.
- The only fallible operation in the public surface today is `Event::validate`, which returns `MissingField` when `name` is absent.
- `MatchingError` is `#[non_exhaustive]`; adding variants is non-breaking.

---
