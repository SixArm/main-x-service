## 3. Data model

All public types are `#[derive(Serialize, Deserialize)]` and round-trip through `serde_json`. Public surface lives in `src/lib.rs` re-exports.

### 3.1 `Place`

```rust
#[non_exhaustive]
pub struct Place {
    pub name: Option<String>,
    pub alternate_names: Vec<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub category: Option<PlaceCategory>,
    pub place_ids: Vec<PlaceId>,
    pub address: Option<Address>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub local_id: Option<String>,
    pub altitude_as_metre: Option<f64>,
    pub elevation_as_metre: Option<f64>,
    pub area_as_metre_2: Option<f64>,
    pub country_code_as_iso_3166_1_alpha_2: Option<String>,
    pub maximum_capacity_count: Option<u32>,
}
```

`Place` carries **15 fields**, every one optional or defaulting to empty.

- `Place` MUST be constructed via `Place::builder()` from outside the crate (`#[non_exhaustive]`).
- `Place::validate(&self) -> Result<()>` MUST return `Ok(())` when `name` is set and `Err(MatchingError::MissingField(_))` otherwise. Validation is not invoked automatically by the matcher.
- `Place` MUST be `Send + Sync` and serde-round-trippable.

#### 3.1.1 Field semantics

The "Scored" column distinguishes fields the probabilistic matcher uses (§6) from fields that are data-only (round-trip honesty; not used in the score).

| Field | Type | Scored? | Meaning |
|---|---|---|---|
| `name` | `Option<String>` | yes (§6.1) | Primary canonical name, e.g. `"Eiffel Tower"`. |
| `alternate_names` | `Vec<String>` | yes (§6.1) | Aliases / endonyms / translations, e.g. `["La Tour Eiffel", "Tour Eiffel"]`. Default empty. |
| `latitude` | `Option<f64>` | yes (§6.3) | Decimal degrees. Conventionally `[-90.0, 90.0]`. |
| `longitude` | `Option<f64>` | yes (§6.3) | Decimal degrees. Conventionally `[-180.0, 180.0]`. |
| `category` | `Option<PlaceCategory>` | yes (§6.5) | Coarse-grained classification (§3.4). |
| `place_ids` | `Vec<PlaceId>` | yes (§6.7) | External scheme-scoped identifiers. Sharing any pair is a deterministic match. |
| `address` | `Option<Address>` | yes (§6.4) | Postal address (§3.3). Optional; partial addresses are first-class. |
| `phone` | `Option<String>` | yes (§6.8) | Primary contact phone. Normalised before comparison. |
| `email` | `Option<String>` | yes (§6.9) | Primary contact email. Normalised before comparison. |
| `local_id` | `Option<String>` | **no** | Originating-system identifier. Stored for round-trip; the matcher MUST NOT score it (different sources may collide). |
| `altitude_as_metre` | `Option<f64>` | **no** | Object height above mean sea level (rooftop, aircraft cruise). May be negative. |
| `elevation_as_metre` | `Option<f64>` | **no** | Ground-surface height above mean sea level (summit, town centre). May be negative. |
| `area_as_metre_2` | `Option<f64>` | **no** | Footprint in square metres. |
| `country_code_as_iso_3166_1_alpha_2` | `Option<String>` | yes (§6.6) | ISO 3166-1 alpha-2 code. Stored as supplied; compared case-insensitively after trim. |
| `maximum_capacity_count` | `Option<u32>` | **no** | Legal / seating capacity. `0` is meaningful; absence is `None`. |

Latitude or longitude values that are non-finite or fall outside the conventional ranges MUST be stored as supplied (round-trip honesty) but MUST be treated as missing by the coordinates scorer (§6.3).

#### 3.1.2 JSON shape

The serde-derived JSON form mirrors the Rust struct one-for-one: field names are the snake-case identifiers in §3.1, optional fields serialise as `null` when absent, `alternate_names` and `place_ids` serialise as JSON arrays, `category` serialises as either a string variant (`"Monument"`) or `{"Other": "..."}`, and `place_ids` entries serialise as `{"scheme": "Wikidata", "value": "Q243"}`.

### 3.2 `PlaceBuilder`

```rust
#[derive(Default)]
pub struct PlaceBuilder { /* private fields */ }
```

Fluent builder for `Place`. All string setters accept `impl Into<String>`. Setters mirror every field on `Place` one-for-one; additionally, `alternate_names(Vec<String>)` and `place_ids(Vec<PlaceId>)` **replace** the entire list, while `add_alternate_name(impl Into<String>)` and `add_place_id(PlaceId)` **append** a single element. `build() -> Place` consumes the builder.

`PlaceBuilder` is `#[derive(Default)]`. All fields start unset (`None` / empty `Vec`).

### 3.3 `Address`

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

Address comparison (§6.4) consults `postcode`, `city`, and `line1`. `line2`, `county`, and `country` are stored but not currently scored.

### 3.4 `PlaceCategory`

```rust
#[non_exhaustive]
pub enum PlaceCategory {
    Hotel, Restaurant, Cafe, Bar, Shop, Mall, Hospital, School,
    University, Library, Museum, Theatre, Cinema, Park, Beach,
    Stadium, Airport, RailwayStation, BusStation, Bank, PostOffice,
    Government, Monument, ReligiousBuilding, Cemetery, Mountain,
    Lake, River, City, Town, Village, Neighborhood, OfficeBuilding,
    Residence, Warehouse, Other(String),
}
```

`PlaceCategory` carries **35 unit variants plus one carrying `Other(String)`** (36 in total). Equality is structural: `PlaceCategory::Other("foo")` matches `PlaceCategory::Other("foo")` but not `PlaceCategory::Other("bar")` and not any enumerated variant. Adding a new variant is non-breaking (`#[non_exhaustive]`). The unit variants cover lodging, food and drink (Hotel, Restaurant, Cafe, Bar), retail (Shop, Mall), health and education (Hospital, School, University, Library), arts (Museum, Theatre, Cinema), open space (Park, Beach), sport (Stadium), transport (Airport, RailwayStation, BusStation), civic (Bank, PostOffice, Government), heritage (Monument, ReligiousBuilding, Cemetery), nature (Mountain, Lake, River), settlement scale (City, Town, Village, Neighborhood), and built form (OfficeBuilding, Residence, Warehouse). `Other(String)` is the catch-all; its carried string participates in equality.

### 3.5 `PlaceId` and `PlaceIdScheme`

```rust
#[non_exhaustive]
pub enum PlaceIdScheme {
    Google, OsmNode, OsmWay, OsmRelation, GeoNames, Wikidata,
    Foursquare, Here, Mapbox, Other(String),
}

pub struct PlaceId {
    pub scheme: PlaceIdScheme,
    pub value: String,
}
```

`PlaceIdScheme` carries **9 unit variants plus `Other(String)`** (10 in total).

- `PlaceId::new(scheme, value) -> Option<Self>` MUST trim surrounding whitespace from `value` and MUST return `None` if the trimmed value is empty.
- Equality MUST be `(scheme, value)`-structural. No per-scheme canonicalisation is performed.
- Cross-scheme identifiers MUST NEVER match: a `(Google, "abc")` and an `(OsmNode, "abc")` denote different things.

The 9 unit schemes are `Google` (opaque IDs typically prefixed `ChIJ…`), `OsmNode` / `OsmWay` / `OsmRelation` (OpenStreetMap point / line-or-area / aggregate feature IDs), `GeoNames` (numeric `geonameid`), `Wikidata` (QIDs such as `Q243`), `Foursquare`, `Here`, and `Mapbox`. `Other(String)` is the catch-all; the carried string participates in equality.

### 3.6 `Confidence`

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

### 3.7 `MatchResult` and `MatchBreakdown`

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
    pub coordinates_score: Option<f64>,
    pub address_score: Option<f64>,
    pub category_score: Option<f64>,
    pub country_code_score: Option<f64>,
    pub place_ids_score: Option<f64>,
    pub phone_score: Option<f64>,
    pub email_score: Option<f64>,
}
```

`MatchBreakdown` carries **9 score fields**, each `Option<f64>` in `[0.0, 1.0]`. `None` means "the field did not participate in the weighted sum"; `Some(s)` carries the per-field score. `MatchResult::confidence` carries `#[serde(default = "default_confidence")]` where `default_confidence()` returns `Confidence::Low`, so legacy payloads predating the field round-trip as `Low`. `MatchBreakdown::email_score` also carries `#[serde(default)]` for the same reason.

### 3.8 `MatchConfig`

See §7 for every knob, default, and preset.

### 3.9 `MatchingError` and `Result`

```rust
#[non_exhaustive]
pub enum MatchingError {
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, MatchingError>;
```

- The matching engine is **infallible**: `match_places`, `deterministic_match`, `match_one_to_many`, and `rank_one_to_many` cannot fail.
- The only fallible operation in the public surface today is `Place::validate`, which returns `MissingField` when `name` is absent.
- `MatchingError` is `#[non_exhaustive]`; adding variants is non-breaking.

---

