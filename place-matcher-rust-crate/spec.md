# Place matcher — specification

**Crate:** `place-matcher` &nbsp;·&nbsp; **Version targeted:** `0.4.0` &nbsp;·&nbsp; **Status:** authoritative

This document is the living, single source of truth (SSOT) for the `place-matcher` Rust crate. Every other document in the repository (`README.md`, `index.md`, `AGENTS.md`, `AGENTS/*.md`, `CHANGELOG.md`) summarises or quotes this file — none contradicts or extends it. When prose elsewhere disagrees with this file, this file wins; when this file disagrees with the code, see §9.

The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted in the sense of RFC 2119 / RFC 8174.

---

## 1. Scope

### 1.1 In scope

- Pairwise matching of two `Place` records to decide whether they refer to the same geographic place.
- A **deterministic** strategy returning `bool`.
- A **probabilistic** strategy returning a renormalised score in `[0.0, 1.0]` with a per-field `MatchBreakdown` for explainability.
- Batch entry points: scoring and ranking a single query against a slice of candidates.
- Supporting text-normalisation primitives (names, postcodes, phones, emails, addresses, phonetic codes).
- Geographic primitives (Haversine distance on a sphere, Gaussian-decay similarity).
- Configurable weights, threshold, and similarity algorithm via `MatchConfig`.
- Serde round-trip of every public data type.

### 1.2 Out of scope

- **Geocoding** (address → coordinates) and **reverse geocoding** (coordinates → address).
- **Routing** or network distance — only great-circle (Haversine) distance is provided.
- **Address parsing as a service**: `Normalizer::parse_address_line` is a best-effort structural decomposition for matching purposes, not a postal-reference lookup.
- **Full-text search** and **place suggestions** — the crate scores known pairs in memory.
- **Persistent storage and indexing** — the crate never reads or writes external state.
- **Candidate blocking** — pre-filtering large candidate sets is a consumer concern.
- **Machine learning** — the algorithm is rule-based; weights are tuneable but the structure is fixed.
- **Locale-aware street-type vocabularies** — only English abbreviations are expanded.

### 1.3 Audience

Data engineers, GIS practitioners, and deduplication-pipeline authors who need an explainable, deterministic library for joining or de-duplicating geographic-place records drawn from heterogeneous sources.

---

## 2. Terminology

The following terms are used throughout this spec with the meanings defined here. Other documents in the repository MUST use the same vocabulary.

- **Place** — a single record about a geographic place, as represented by the `Place` struct (§3.1). May describe a landmark, natural feature, chain-store branch, administrative area, or any other geographically-located entity.
- **Match** — the verdict that two `Place` records refer to the same geographic place. Verdicts come in two flavours, deterministic and probabilistic, with sharply different guarantees.
- **Deterministic match** — a boolean verdict from `MatchingEngine::deterministic_match` (§5.1). Returns `true` only when an objective, transitive criterion is satisfied. Never produces a score.
- **Probabilistic match** — a `MatchResult` from `MatchingEngine::match_places` (§5.2) carrying a score, an `is_match` boolean, a `Confidence` band, and a `MatchBreakdown`.
- **Per-field breakdown** — the `MatchBreakdown` struct (§3.7) containing one `Option<f64>` per scored component. `None` means "not scored on at least one side"; `Some(s)` carries a value in `[0.0, 1.0]`.
- **Renormalisation** — the rule in §5.2.2 by which the weighted sum is divided by the sum of *participating* weights, so missing fields neither contribute to nor penalise the overall score.
- **Confidence band** — a coarse `High` / `Medium` / `Low` bucket derived from the probabilistic score (§3.6). Bands are fixed; they do **not** follow `match_threshold`.
- **Normalisation** — the deterministic, idempotent text transformations in `Normalizer` (§4) applied before comparison.
- **Scheme-local identifier** — a `PlaceId` is identified by both its `scheme` (e.g. `Wikidata`) and its `value`. Identifiers from different schemes never match each other, even if the value string is identical (§3.5).
- **Score** — a real number in `[0.0, 1.0]`, where `1.0` means "identical" and `0.0` means "no observable similarity".
- **Weight** — a dimensionless multiplier on a component score. Weights need not sum to `1.0`; the renormaliser handles that.
- **Strict mode** — the `MatchConfig::strict_mode` flag (§5.2.3): when `true`, the probabilistic `is_match` boolean additionally requires `deterministic_match` to return `true`.

---

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

## 4. Normalisation

All normalisers in `Normalizer` are stateless associated functions and **idempotent** (`f(f(x)) == f(x)`), **deterministic** (no clocks, no RNGs), and allocate at most a single new `String`.

Operational examples per rule, plus full input/output tables, live in [`AGENTS/normalization.md`](AGENTS/normalization.md). The behaviour-defining contracts are summarised here.

### 4.1 Names — `Normalizer::normalize_name`

Pipeline: NFKD-decompose, drop combining marks, drop ASCII punctuation, lowercase, collapse consecutive whitespace to single ASCII spaces, trim ends.

**Known limit:** non-ASCII punctuation (e.g. the curly apostrophe `’` U+2019) is **not** stripped. Upstream code should convert to ASCII first. Characters with no NFKD decomposition (e.g. `ł`) survive lowercased.

### 4.2 Postcodes — `Normalizer::normalize_postcode`

Drop all whitespace; uppercase. No locale-specific validation. `"cf10 1aa"` → `"CF101AA"`.

### 4.3 Phone numbers

Two normalisers are provided.

#### 4.3.1 Legacy — `Normalizer::normalize_phone`

Keep ASCII digits only; then strip prefixes in this order: leading `0044` (drop 4 chars); else leading `44` when result is ≥12 digits (drop 2 chars); else leading `0` when result is >1 digit (drop 1 char). UK-centric and infallible. Used as the fallback when E.164 normalisation fails. International numbers from other countries pass through unchanged.

#### 4.3.2 E.164 — `Normalizer::normalize_phone_e164`

```rust
fn normalize_phone_e164(phone: &str, default_country: Option<&str>) -> Option<String>;
```

Match `+CC` / `00CC` / default-country trunk; strip the national trunk prefix; validate the national-significant-number (NSN) length against a per-country range; return `+CCNNN…`. Returns `None` for empty input, unparseable inputs, or unsupported countries. `default_country` is an ISO 3166-1 alpha-2 code (`"GB"`, `"FR"`, `"US"`, …); pass `None` to refuse to assume a default.

Supported countries (per the in-code `COUNTRY_PHONE_TABLE`): `GB`, `FR`, `DE`, `ES`, `IE`, `IT`, `NL`, `BE`, `PT`, `CH`, `AT`, `SE`, `NO`, `DK`, `FI`, `PL`, `AU`, `NZ`, `US`, `CA`, `JP`, `CN`, `IN`, `BR`, `MX`, `ZA`, `BG`, `CZ`, `EE`, `GR`, `HR`, `IS`, `LI`, `LT`, `LV`, `MT`, `RO`, `SI`, `SK`. Each entry pins the dial code, the national trunk prefix (`"0"` for most of Europe and Asia; `"8"` for Lithuania; `None` for NANP / Spain / Portugal / several others), and the min / max NSN length.

The matching engine MUST prefer the E.164 form when both inputs canonicalise, and MUST fall back to the legacy form otherwise (§6.8).

### 4.4 Email — `Normalizer::normalize_email`

```rust
fn normalize_email(email: &str, gmail_dot_folding: bool) -> Option<String>;
```

Trim surrounding whitespace; lowercase the whole address; reject inputs without exactly one `@` or with an empty localpart or domain (return `None`). When `gmail_dot_folding` is `true` AND the domain is `gmail.com` or `googlemail.com`, strip every `.` from the localpart and drop any `+tag` suffix. Non-Gmail domains are unaffected by the folding flag.

### 4.5 Address-line parsing — `ParsedAddressLine`, `Normalizer::parse_address_line`

```rust
pub struct ParsedAddressLine {
    pub house_number: Option<String>,
    pub unit: Option<String>,
    pub street: String,
}
```

Best-effort structural decomposition (format-only — no postal reference is consulted):

1. **Unit prefix.** First token matched against `{flat, apartment, apt, unit, suite, ste, room, rm}`. If recognised, the next alphanumeric run is the identifier; result is `format!("{keyword} {identifier}")` lowercased.
2. **House number.** Leading run of ASCII digits, optionally followed by a single alphabetic suffix (`"10A"`). The suffix is taken **only when not followed by another alphanumeric**, so `"10 Apple Tree Lane"` does not absorb the `A` of `Apple`. Uppercased.
3. **Street.** The remainder, run through `Normalizer::normalize_address_line` (= `expand_street_abbreviations` + `normalize_name`).

Inputs that do not match the regular structure (e.g. a postcode-only string, a city name) degrade gracefully: `house_number` and `unit` are `None`, and `street` carries the normalised input.

#### 4.5.1 `expand_street_abbreviations`

Whole-token expansion of English abbreviations: `St` → `street`, `Rd` → `road`, `Ave` → `avenue`, `Blvd` → `boulevard`, `N` → `north`, etc. The ambiguous `"St"` (Street vs Saint) is **always** expanded to `street`; pre-process upstream if you need finer disambiguation (see OQ-D).

### 4.6 Phonetic code — `Normalizer::phonetic_code`

Normalise the name (§4.1), then apply American Soundex (`soundex::american_soundex`). Empty input returns `""` (not a default Soundex value). Soundex is tuned for English-language names; non-English phonemes may be lost. A locale-aware encoder (Double Metaphone, Daitch-Mokotoff) is tracked as OQ-E.

---

## 5. Matching pipeline

The crate exposes two strategies under one engine.

### 5.1 Deterministic match

```rust
fn deterministic_match(&self, p1: &Place, p2: &Place) -> bool;
```

Returns `true` iff **any one** of the following holds:

1. The two `place_ids` lists share at least one element under `PlaceId` equality (i.e. an identical `(scheme, value)` pair).
2. Both places carry a `name` that normalises (via `Normalizer::normalize_name`) to the same string **AND** both carry an `address.postcode` that normalises (via `Normalizer::normalize_postcode`) to the same string.

Otherwise returns `false`. Boolean result; no score.

### 5.2 Probabilistic match

```rust
fn match_places(&self, p1: &Place, p2: &Place) -> MatchResult;
```

Produces `MatchResult { score, is_match, confidence, breakdown }`: `breakdown` is the per-field `Option<f64>` contributions (§6); `score` is the weight-renormalised sum across components that scored (§5.2.2); `is_match` is `score >= MatchConfig::match_threshold` under default mode, additionally requiring `deterministic_match` under strict mode (§5.2.3); `confidence` is the fixed-band derivative of `score` (§3.6).

#### 5.2.1 Per-field scoring

Each field is scored as described in §6. A `None` from any component means "did not participate"; the field is skipped by the renormaliser.

#### 5.2.2 Weighted-sum reducer

```text
weighted_sum  = sum of (score_i * weight_i)  over components that scored
total_weight  = sum of (weight_i)            over components that scored
if name_phonetic_score is Some and > 0.9:
    weighted_sum += name_phonetic_score * 0.05
    total_weight += 0.05
score = if total_weight > 0 then weighted_sum / total_weight else 0.0
```

The renormalisation rule MUST hold: missing fields neither contribute to nor penalise the overall score. A worked numeric trace lives in §11.1.

#### 5.2.3 Strict mode

When `MatchConfig::strict_mode = true`, the engine computes the same `score` and `confidence`, but tightens `is_match` to `score >= match_threshold && deterministic_match`. A fuzzy near-match that clears the threshold but has no shared place ID and no normalised-name + postcode agreement is rejected as `is_match = false`.

### 5.3 Batch entry points

```rust
fn match_one_to_many(&self, query: &Place, candidates: &[Place]) -> Vec<MatchResult>;
fn rank_one_to_many(&self, query: &Place, candidates: &[Place]) -> Vec<(usize, MatchResult)>;
```

- `match_one_to_many` returns results parallel to the input slice.
- `rank_one_to_many` returns the same results sorted by descending `score`, with ties broken by ascending original index. Ordering MUST be deterministic across calls with the same inputs.

### 5.4 Behaviour when a field is missing

If a field is `None` on either side, the corresponding `MatchBreakdown` entry is `None` and the field is skipped by the weighted-sum reducer. This is the renormalisation rule: missing data is treated as "no information", not as "disagreement".

The single exception is `name_phonetic_score`: when `MatchConfig::use_phonetic_matching = false`, it is always `None` regardless of input data.

---

## 6. Per-field scoring algorithms

Each component returns `Option<f64>` in `[0.0, 1.0]`. A `None` means the component did not participate (§5.4).

### 6.1 Name — `name_score`

`collect_names(p)` gathers `p.name` plus `p.alternate_names`, skipping empty / whitespace-only strings. The matcher computes the similarity for every pair drawn from `collect_names(p1) × collect_names(p2)` (after `normalize_name` on each side) using `MatchConfig::name_algorithm`, and returns the maximum.

`MatchConfig::name_algorithm` selects one of `JaroWinkler` (`Scorer::jaro_winkler_similarity`; prefix-bonused, good for short names), `Levenshtein` (`Scorer::levenshtein_similarity` = `1 - edit_distance / max_len`), `Exact` (`Scorer::exact_match`; binary, case-sensitive), or `Combined` (`0.7 * JaroWinkler + 0.3 * Levenshtein`; **default**). Edge cases for the underlying scorers: two empty strings score `1.0`; one empty and one non-empty scores `0.0`.

`name_score` is `None` when either side has no usable names.

### 6.2 Phonetic — `name_phonetic_score`

Only computed when `MatchConfig::use_phonetic_matching = true`. For every pair across `collect_names`, compute `Normalizer::phonetic_code(name)` on each side and take the maximum equality (`1.0` if any pair of non-empty codes matches, else `0.0`).

`name_phonetic_score` contributes only as a **bonus**: when `> 0.9`, the weighted-sum reducer adds `name_phonetic_score * 0.05` to the weighted sum and `0.05` to the total weight (§5.2.2). The bonus MUST NOT lower the overall score: if the gate does not fire, the bonus is simply not added.

`name_phonetic_score` is `None` when phonetic matching is off, or when either side has no usable names.

### 6.3 Coordinates — `coordinates_score`

Compute the Haversine distance and apply Gaussian decay:

```text
d = Scorer::haversine_metres(lat1, lon1, lat2, lon2)
s = MatchConfig::coordinates_scale_metres
coordinates_score = exp(-(d / s)^2)
```

Contract for the decay function:

- `d == 0` → `1.0`.
- `d == s` → `1/e` (~`0.368`).
- `d == 3 * s` → ~`0.0001`.
- Non-finite `d`, non-positive `s`, negative `d` → `0.0`.

`Scorer::haversine_metres` uses mean Earth radius `6_371_000.0` m on a sphere. The function is total over `f64`; non-finite inputs produce `NaN` without panic. The implementation handles equator and date-line crossings correctly.

`coordinates_score` is `None` when any of `lat1`, `lon1`, `lat2`, `lon2` is missing, non-finite, or outside the conventional ranges (`[-90, 90]` for latitude, `[-180, 180]` for longitude).

### 6.4 Address — `address_score`

When both sides have an `address`, run a weight-renormalised average across populated sub-components:

| Sub-component | Sub-weight | Sub-score |
|---|---|---|
| Postcode | `0.5` | `1.0` if `normalize_postcode(a) == normalize_postcode(b)`, else `0.0`. |
| City | `0.3` | `Scorer::jaro_winkler_similarity` on the normalised city names. |
| Line 1 | `0.2` | `parse_address_line` on each side; with house numbers present on both, `0.6 * jaro_winkler(street) + 0.4 * (house_number_a == house_number_b)`. Otherwise the street similarity alone. |

If no sub-component is populated on both sides, the address score is the neutral `0.5`.

`address_score` is `None` only when at least one side has no `address` at all (the entire `Option<Address>` is `None`).

### 6.5 Category — `category_score`

`1.0` if both `category` values are set and structurally equal; `0.0` if both set but differ; `None` if either is `None`.

### 6.6 Country code — `country_code_score`

`1.0` if both `country_code_as_iso_3166_1_alpha_2` values are set and equal after trim + ASCII lowercase; `0.0` otherwise; `None` if either is `None`.

### 6.7 Place IDs — `place_ids_score`

`1.0` if both `place_ids` lists are non-empty AND any `(scheme, value)` pair is shared (under `PlaceId` equality); `0.0` if both lists are non-empty but no pair is shared; `None` if either list is empty.

### 6.8 Phone — `phone_score`

Compute `normalize_phone_e164(phone, cc)` for both sides where `cc = MatchConfig::phone_default_country`. If both canonicalise, `1.0` iff the canonical strings are equal, else `0.0`. Otherwise (either side fails to canonicalise) compare `normalize_phone(phone)` on both sides: `1.0` iff equal, else `0.0`.

`phone_score` is `None` only when either side has `phone = None`.

### 6.9 Email — `email_score`

Compute `normalize_email(email, gmail_dot_folding)` for both sides. `1.0` iff both canonicalise and are equal; `0.0` if both canonicalise but differ. `None` if either side has `email = None`, or if either side fails to canonicalise (`normalize_email` returned `None`).

---

## 7. Configuration

```rust
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    pub match_threshold: f64,
    pub name_weight: f64,
    pub coordinates_weight: f64,
    pub coordinates_scale_metres: f64,
    pub address_weight: f64,
    pub category_weight: f64,
    pub country_code_weight: f64,
    pub place_ids_weight: f64,
    pub phone_weight: f64,
    pub email_weight: f64,
    pub use_phonetic_matching: bool,
    pub name_algorithm: SimilarityAlgorithm,
    pub strict_mode: bool,
    pub gmail_dot_folding: bool,
    pub phone_default_country: Option<String>,
}
```

`MatchConfig` carries `#[serde(default)]`; a partial JSON document fills missing fields from `MatchConfig::default()`.

### 7.1 Defaults — `MatchConfig::default()`

| Field | Default | Meaning |
|---|---|---|
| `match_threshold` | `0.80` | Score at or above which `is_match` is `true`. |
| `name_weight` | `0.20` | Weight of the name component (§6.1). |
| `coordinates_weight` | `0.30` | Weight of the coordinates component (§6.3). |
| `coordinates_scale_metres` | `50.0` | Gaussian-decay scale `s` in metres (§6.3). |
| `address_weight` | `0.10` | Weight of the address component (§6.4). |
| `category_weight` | `0.10` | Weight of the category component (§6.5). |
| `country_code_weight` | `0.05` | Weight of the country-code component (§6.6). |
| `place_ids_weight` | `0.15` | Weight of the place-IDs component (§6.7). |
| `phone_weight` | `0.03` | Weight of the phone component (§6.8). |
| `email_weight` | `0.02` | Weight of the email component (§6.9). |
| `use_phonetic_matching` | `false` | Compute Soundex-bonus (§6.2). |
| `name_algorithm` | `SimilarityAlgorithm::Combined` | Algorithm used by `name_score`. |
| `strict_mode` | `false` | Tighten `is_match` to also require `deterministic_match` (§5.2.3). |
| `gmail_dot_folding` | `false` | Apply Gmail-specific localpart canonicalisation (§4.4). |
| `phone_default_country` | `Some("GB")` | Fallback ISO 3166-1 alpha-2 country for E.164 parsing (§4.3.2). |

### 7.2 Strict — `MatchConfig::strict()`

Overrides `match_threshold = 0.95` and `strict_mode = true`; every other field inherits `default()`. Under strict mode, `is_match` additionally requires `deterministic_match`; `score` and `confidence` are unaffected.

### 7.3 Lenient — `MatchConfig::lenient()`

Overrides `match_threshold = 0.65` and `use_phonetic_matching = true`; every other field inherits `default()`.

### 7.4 Tuning guidance

The "symptom → knob" tuning table is maintained in [`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md) under "Tuning guidance".

---

## 8. Determinism and safety

The crate MUST satisfy the following:

1. **No `unsafe`** — enforced by `#![forbid(unsafe_code)]` in `lib.rs`.
2. **`#![deny(missing_docs)]`** — every public item carries a `///` doc comment.
3. **No IO** — no `std::fs`, no `std::net`, no `tokio`, no `reqwest`, no `tracing`, no `log` from library code. `src/main.rs` is a demo binary (prints to stdout) and is **not** part of the library API.
4. **No global state** — no `static mut`, no `OnceCell` holding place data, no thread-local caches.
5. **No clocks, no RNGs, no environment-variable reads** — same inputs always produce the same outputs, byte-for-byte.
6. **No panics on user data** — library code MUST use `Option` / `Result` and MUST NOT unwrap on caller-derived values. The matching engine itself is infallible.
7. **`Send + Sync`** — every public type is `Send + Sync`. `MatchingEngine` is immutable after construction and cheap to clone.
8. **Serde-round-trippable** — every public data type MUST round-trip through `serde_json` (and any other `serde` format).

---

## 9. Public API contract (SemVer)

The crate follows Semantic Versioning. Pre-1.0, minor bumps MAY contain breaking API changes (Cargo convention) and MUST be documented under "Breaking" in the CHANGELOG entry for that version. **0.4.0** is the first release of `place-matcher` under the geographic place-matcher domain; the 0.3.x line targeted a different domain and is not upgrade-compatible. The set of items re-exported from `lib.rs` is the public surface — adding is non-breaking, removing or renaming is breaking. Default-weight or default-threshold changes count as observable behaviour changes and MUST be documented under "Behaviour Change" in the CHANGELOG. Every behavioural change SHOULD ship with a new CHANGELOG entry (under "Unreleased" until the next release).

### 9.1 `#[non_exhaustive]` items

The following items carry `#[non_exhaustive]`:

- `Place` and `Address` — adding fields is non-breaking. Downstream code MUST construct via the builder / `new` rather than struct-literal syntax.
- `PlaceCategory` and `PlaceIdScheme` — adding variants is non-breaking. Downstream `match` statements MUST include a `_ => ...` arm.
- `MatchingError` — adding variants is non-breaking.

Removing fields or variants is breaking.

### 9.2 JSON shape stability

Every public data type derives `serde::{Serialize, Deserialize}`. `MatchConfig` carries `#[serde(default)]` at struct level so partial documents inherit defaults from `MatchConfig::default()`. `MatchBreakdown::email_score` and `MatchResult::confidence` carry `#[serde(default)]` to allow legacy payloads predating those fields to round-trip.

Renaming a JSON key or changing its type is breaking.

### 9.3 Code wins on divergence

When `spec.md` and the code disagree, the **code** is what is shipped to users — the spec is updated to match the code, and the divergence is flagged in the CHANGELOG so a human can decide whether the spec's original intent should be restored in a follow-up.

---

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

---

## 11. Worked examples

The examples below trace the spec through realistic place data. They are illustrative; the authoritative pinned behaviour lives in `tests/integration_tests.rs`, `tests/property_tests.rs`, and the doctests under `src/*.rs`. Further worked examples (Big Ben / Elizabeth Tower, Mt Snowdon / Yr Wyddfa, Wembley Stadium, batch ranking, strict mode) live in [`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md).

### 11.1 Eiffel Tower / La Tour Eiffel — probabilistic match

```rust
let a = Place::builder()
    .name("Eiffel Tower")
    .add_alternate_name("La Tour Eiffel")
    .latitude(48.858_222)
    .longitude(2.294_500)
    .category(PlaceCategory::Monument)
    .country_code_as_iso_3166_1_alpha_2("FR")
    .build();

let b = Place::builder()
    .name("Tour Eiffel")
    .latitude(48.858_3)
    .longitude(2.294_5)
    .category(PlaceCategory::Monument)
    .country_code_as_iso_3166_1_alpha_2("FR")
    .build();

let r = MatchingEngine::default_config().match_places(&a, &b);
// r.is_match == true; r.confidence == High
```

Per-field sketch under default weights: `name_score ≈ 1.00` (the alternate matches exactly; weight `0.20`); `coordinates_score = exp(-(9/50)^2) ≈ 0.97` (≈ 9 m apart; weight `0.30`); `category_score = 1.0` (both `Monument`; weight `0.10`); `country_code_score = 1.0` (both `"FR"`; weight `0.05`); every other component is `None`. Renormalised: `(0.20 + 0.291 + 0.10 + 0.05) / 0.65 ≈ 0.984` ≥ `0.80` → `is_match = true`; `Confidence::High`.

### 11.2 Two Starbucks branches — same name, different cities

```rust
let manhattan = Place::builder()
    .name("Starbucks")
    .latitude(40.7589)
    .longitude(-73.9851)
    .category(PlaceCategory::Cafe)
    .country_code_as_iso_3166_1_alpha_2("US")
    .build();

let los_angeles = Place::builder()
    .name("Starbucks")
    .latitude(34.0522)
    .longitude(-118.2437)
    .category(PlaceCategory::Cafe)
    .country_code_as_iso_3166_1_alpha_2("US")
    .build();

let r = MatchingEngine::default_config().match_places(&manhattan, &los_angeles);
// r.is_match == false; coordinates_score decays essentially to 0
```

`name_score = 1.0`, `category_score = 1.0`, `country_code_score = 1.0` — yet `coordinates_score ≈ 0.0` (separation ~3940 km, far beyond `3 × scale`). With `coordinates_weight = 0.30` dominating, the renormalised total drops well below `0.80`.

### 11.3 Shared Google Place ID — deterministic short-circuit

```rust
let id = PlaceId::new(PlaceIdScheme::Google, "ChIJLU7jZClu5kcR4PcOOO6p3I0").unwrap();
let a = Place::builder().name("Eiffel Tower").add_place_id(id.clone()).build();
let b = Place::builder().name("Wholly Different Name").add_place_id(id).build();

assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
```

`deterministic_match` short-circuits on the shared `(Google, "ChIJ…")` pair. Names and coordinates are irrelevant.

---

## 12. Glossary cross-reference

| Symbol | One-line meaning | Defined in |
|---|---|---|
| `Address` | Postal address with optional line1/line2/city/county/postcode/country. | §3.3 |
| `Confidence` | High / Medium / Low band derived from probabilistic score. | §3.6 |
| `MatchBreakdown` | Per-field `Option<f64>` contributions returned with every `MatchResult`. | §3.7 |
| `MatchConfig` | Tunable weights, threshold, algorithm, presets. | §3.8 / §7 |
| `MatchResult` | `{ score, is_match, confidence, breakdown }` returned by `match_places`. | §3.7 |
| `MatchingEngine` | The engine. Immutable after construction. `Send + Sync`. | §5 |
| `MatchingError` | Sum-type for fallible operations. Only variant today is `MissingField`. | §3.9 |
| `Normalizer` | Stateless namespace for text normalisation. | §4 |
| `ParsedAddressLine` | `{ house_number, unit, street }` decomposition of an address line. | §4.5 |
| `Place` | Core place record. 15 fields, every one optional or defaulting to empty. | §3.1 |
| `PlaceBuilder` | Fluent builder for `Place`. | §3.2 |
| `PlaceCategory` | 35 enumerated variants plus `Other(String)`. | §3.4 |
| `PlaceId` | `{ scheme, value }`. Scheme-local equality. | §3.5 |
| `PlaceIdScheme` | 9 enumerated variants plus `Other(String)`. | §3.5 |
| `Result<T>` | Alias for `std::result::Result<T, MatchingError>`. | §3.9 |
| `Scorer` | Stateless namespace for similarity primitives (string, geographic). | §6 |
| `SimilarityAlgorithm` | `JaroWinkler` / `Levenshtein` / `Exact` / `Combined`. | §6.1 |
| `deterministic_match` | Boolean verdict. Shared place ID OR equal normalised name + postcode. | §5.1 |
| `match_one_to_many` | Score query against candidate slice; preserve input order. | §5.3 |
| `match_places` | Probabilistic single-pair match. Returns `MatchResult`. | §5.2 |
| `rank_one_to_many` | Score and sort by descending score; deterministic ascending-index tiebreak. | §5.3 |
| Renormalisation | Divide weighted sum by sum of participating weights. Missing fields skip. | §5.2.2 |
| Strict mode | `is_match` additionally requires `deterministic_match`. | §5.2.3 |

---

## 13. References

- `src/lib.rs` — re-exports and top-level crate docs.
- `examples/basic_usage.rs`, `examples/custom_config.rs` — runnable end-to-end examples.
- `benches/match_pair.rs` — criterion harness exercising hot paths.
- `tests/integration_tests.rs`, `tests/property_tests.rs` — pinned behaviour suite.
- `CHANGELOG.md` — version history.
- `AGENTS.md` and `AGENTS/*.md` — contributor and agent guidance.
- `index.md` — documentation entry point.
