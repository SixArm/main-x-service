# Changelog archive

Historical entries from versions prior to the current major. Live
entries (Unreleased + the most recent released version) remain in
[`CHANGELOG.md`](./CHANGELOG.md).

## [0.3.0] - 2026-05-26

### Dependencies
- Upgraded `thiserror` from `1.0` to `2.0` (no public-API impact; `MatchingError` still derives `thiserror::Error`).
- Upgraded `nhs-number` from `0.3.0` to `1.0` (no public-API impact; `parse_uk_nhs_number` and `parse_uk_hc_number` continue to delegate to `NHSNumber::from_str` and read the `[i8; 10]` `digits` field).
- `cargo audit` reports zero vulnerabilities across the 85-crate dependency closure.

### Manifest
- Fixed malformed `authors` entry (missing closing `>`).
- Corrected `repository` URL to `https://github.com/sixarm/Event-matcher-rust-crate`.
- Expanded `description` to mention multinational identifier coverage.
- Tightened `keywords`: replaced generic `algorithms` and `medical` with `healthcare`, `identity`, `fhir`.
- Added `CHANGELOG.md` to the published include list.

### Added
- International phone-number support (spec §14.3.2, FR-17/FR-30, task T-18).
  - `Normalizer::normalize_phone_e164(phone, default_country)` returns the E.164 canonical form (`+CCNNN…`) when the input parses against a supported country, else `None`.
  - Supported countries cover all six national-identifier jurisdictions (UK, FR, ES, IE, plus UK Northern Ireland via the GB dial code, plus US for SSN) and the major place-mobility partners (CA, DE, IT, NL, BE, PT, CH, AT, SE, NO, DK, FI, PL, AU, NZ, JP, CN, IN, BR, MX, ZA).
  - `MatchConfig::phone_default_country: Option<String>` selects the ISO 3166-1 alpha-2 country code used when an input lacks an explicit international marker. Defaults to `Some("GB")` to preserve the prior UK-centric behaviour.
  - `MatchingEngine::match_places` now scores phone equality using the E.164 form when both inputs parse, falling back to the legacy `Normalizer::normalize_phone` comparison when either input cannot be parsed. The fallback preserves the previous behaviour for inputs the country table does not cover.

### Behaviour change
- Phone numbers from different countries that share the same national-significant digits (e.g. a French `01 23 45 67 89` and the UK-default interpretation of the same string) no longer collide as a phone-field match. Consumers relying on the previous collision behaviour MUST set `MatchConfig::phone_default_country` to the predominant country, or to `None` to disable the assumption.

### API surface
- `MatchConfig` gains a `phone_default_country` field. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add the field (use `..MatchConfig::default()` to absorb it automatically).

### Added (serialisable config)
- `MatchConfig`, `SimilarityAlgorithm`, and `NicknameTable` are now `Serialize + Deserialize` (spec §16, task T-1).
  - `MatchConfig` carries `#[serde(default)]`, so a JSON document overriding a subset of fields deserialises with the remaining fields filled from `MatchConfig::default()`. Production deployments can ship a minimal config file without coupling to the full schema.
  - `SimilarityAlgorithm` serialises as the bare variant name (`"JaroWinkler"`, `"Levenshtein"`, `"Exact"`, `"Combined"`).
  - `NicknameTable` serialises as `{"classes": [["michael", "mike", …], …]}` with entries already in normalised form.

### Added (Confidence enum)
- `Confidence` enum on `MatchResult` (spec FR-37, §12.5, task T-2).
  - New public `Confidence { High, Medium, Low }` enum in `Event_matcher::matcher` (re-exported as `Event_matcher::Confidence`). `Confidence::from_score(f64)` exposes the banding function; it is total over `f64` (NaN/negative → `Low`, > 1.0 → `High`).
  - `MatchResult::confidence` populated on every `match_places` call. Boundaries: `score >= 0.90 → High`, `score >= 0.75 → Medium`, else `Low` (inclusive on the low side). Bands are **independent of `MatchConfig::match_threshold`** so the same score always maps to the same band.
  - `confidence` is `#[serde(default)]` (defaults to `Low`); legacy JSON payloads predating the field deserialise cleanly as "needs re-scoring".

### Added (email scoring)
- Email-address scoring (spec FR-35/FR-36, §14.3a, task T-11; resolves OQ-2).
  - `Normalizer::normalize_email(email, gmail_dot_folding) -> Option<String>` returns the canonical lowercase form when the input has a single `@` and non-empty localpart/domain, else `None`.
  - When `gmail_dot_folding = true` and the domain is `gmail.com` or `googlemail.com`, the localpart has every `.` removed and any `+tag` suffix dropped (mirroring Gmail's routing rules).
  - `MatchConfig` gains `email_weight: f64` (default `0.05`) and `gmail_dot_folding: bool` (default `false`).
  - `MatchBreakdown` gains `email_score: Option<f64>` — `Some(1.0)` for canonical-form equality, `Some(0.0)` for canonical-form mismatch, `None` for missing or unparseable input on either side.
  - `local_id` is **deliberately not scored** (no `local_id_score` field) because different organisations may issue colliding values.

### API surface (email scoring)
- `MatchConfig` gains `email_weight` and `gmail_dot_folding` fields. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add the fields (use `..MatchConfig::default()` to absorb them automatically).
- `MatchBreakdown` gains an `email_score` field. The field is `#[serde(default)]` so existing JSON payloads round-trip unchanged.

### Added (nickname dictionary)
- Nickname-aware name scoring (spec FR-33/FR-34, task T-10).
  - New public `NicknameTable` type in `Event_matcher::nicknames` (re-exported as `Event_matcher::NicknameTable`). API: `empty()`, `english()`, `with_class(names)`, `are_equivalent(a, b)`, `is_empty()`, `len()`.
  - `MatchConfig::nickname_table: NicknameTable` field; defaults to `NicknameTable::empty()` so existing callers see no change.
  - When the configured table considers two normalised names equivalent, `MatchingEngine` lifts the per-name component score to `max(score, 0.9)`. The boost never lowers a score.
  - Built-in `NicknameTable::english()` ships with the most common English nicknames (`Michael`/`Mike`, `Elizabeth`/`Liz`, `Robert`/`Bob`, `William`/`Bill`, `Richard`/`Dick`, …, ≥40 classes). The exact contents are NOT part of the stable contract — entries may be added in minor releases.

### API surface (nickname dictionary)
- `MatchConfig` gains a `nickname_table` field. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add the field (use `..MatchConfig::default()` to absorb it automatically).

### Added (United States SSN)
- United States Social Security Number support (spec FR-32, task T-21).
  - `identifiers::parse_us_ssn(s)` returns the 9-digit canonical compact form (`"AAAGGSSSS"`) when the input parses, or `None` for structurally-impossible values (area `000` / `666` / `900..=999`, group `00`, serial `0000`, wrong length, non-digits).
  - `Place` gains an `us_ssn: Option<String>` field with a matching `PlaceBuilder::us_ssn` setter.
  - `MatchConfig` gains `us_ssn_weight: f64` (default `0.30`).
  - `MatchBreakdown` gains `us_ssn_score: Option<f64>`.
  - `MatchingEngine::deterministic_match` treats US SSN as an independent scheme-local identifier; cross-scheme matching is still forbidden.
  - `Place::validate` now accepts a solo `us_ssn` as a sufficient identifying field.

### Added (address parsing)
- Sophisticated address parsing (spec §14.4a, FR-31, task T-20).
  - `Normalizer::expand_street_abbreviations(line)` — whole-token expansion of street-type (`St`/`Rd`/`Ave`/`Blvd`/`Ln`/`Dr`/`Ct`/`Pl`/`Sq`/`Ter`/`Hwy`/`Pkwy`/`Mt`/`Cres`/`Gdns`/`Gr`/`Cl`/`Pk`/`Plz`/`Expy`/`Trl`) and directional (`N`/`S`/`E`/`W`/`NE`/`NW`/`SE`/`SW`) abbreviations.
  - `Normalizer::normalize_address_line(line)` — abbreviation expansion plus the name-normalisation pipeline. Idempotent.
  - `Normalizer::parse_address_line(line) -> ParsedAddressLine { house_number, unit, street }`. The struct is `Serialize + Deserialize` and is re-exported from the crate root as `Event_matcher::ParsedAddressLine`.
  - `MatchingEngine`'s line-1 comparison now uses the abbreviation-aware normaliser plus a structural house-number sub-component (60% street similarity + 40% house-number exact match when both sides have a number; street similarity alone otherwise). The surrounding postcode/city/line-1 aggregation arithmetic is unchanged.

## [0.1.0] - 2025-11-25

### Added
- Initial release of Event matcher crate
- Core `Place` data structure with builder pattern
- `MatchingEngine` with configurable weights and thresholds
- Deterministic matching algorithm (exact matches)
- Probabilistic matching algorithm (fuzzy matching)
- String similarity scoring (Jaro-Winkler, Levenshtein)
- Phonetic name matching (Soundex)
- Text normalization utilities:
  - Name normalization (diacritics, punctuation, case)
  - NHS number normalization
  - Postcode normalization (whitespace strip + uppercase)
  - Phone number normalization (international prefix `0044`, dialling code `44`, trunk `0`)
- Address comparison with postcode, city, and street matching
- Support for NHS-format check-digit identifiers
- Diacritic handling via NFKD decomposition (Siân → Sian, José → Jose)
- Three matching configurations: Default, Strict, Lenient
- Comprehensive test suite (31 tests total):
  - 14 unit tests
  - 17 integration tests
  - Doc tests
- Serialization support via serde
- Detailed match breakdown with component scores
- Confidence levels (High, Medium, Low)

### Research Basis
- Implemented based on findings from:
  - "Person matcher within a Health Information Exchange" (Grannis et al., 2014)
  - "Patient Identification Techniques – Approaches, Implications, and Findings" (Reisman, 2020)

### Features
- Achieves 90%+ matching accuracy on similar records
- Fast in-memory matching
- Configurable weights and thresholds
- Unicode diacritic handling via NFKD decomposition
- NHS-format check-digit identifier handling
- Detailed matching breakdowns for transparency
- Full test coverage

### Documentation
- Comprehensive README with examples
- API documentation with inline examples
- Integration tests demonstrating real-world scenarios
- Demo application showing various matching scenarios

## [0.4.0] (place matcher — historical)

**Migration note.** 0.4.0 is the first release of `Event-matcher` as a geographic Event-matcher library. Prior 0.3.x releases targeted a different domain entirely. There is **no smooth upgrade path** from 0.3.x to 0.4.0 — every public type has different fields, every scoring component has different semantics, and the `MatchConfig` weight table has been replaced. Downstream code must be rewritten against the new surface; treat the upgrade as an integration project, not a version bump. Pin to `0.3.x` if you depend on the prior behaviour and migrate when ready.

### Breaking — domain
- Crate domain changed from prior subject matter to **geographic Event matcher**. The crate now matches places such as landmarks, natural features, chain branches, and administrative areas.

### Breaking — `Place` model
- Replaced. The new fields are: `name`, `alternate_names`, `latitude`, `longitude`, `category`, `place_ids`, `address`, `phone`, `email`, `local_id`, `altitude_as_metre`, `elevation_as_metre`, `area_as_metre_2`, `country_code_as_iso_3166_1_alpha_2`, `maximum_capacity_count`.
- `Place` remains `#[non_exhaustive]`; construct via `Place::builder()`. The builder accepts `impl Into<String>` on every string setter.
- `Place::validate` now requires only that `name` is set; otherwise returns `MatchingError::MissingField`.

### Breaking — `Address` model
- Trimmed to `line1`, `line2`, `city`, `county`, `postcode`, `country`. All fields remain `Option<String>`. `#[non_exhaustive]` retained. Fluent `with_*` setters preserved.

### Breaking — `MatchBreakdown`
- New shape. Fields: `name_score`, `name_phonetic_score`, `coordinates_score`, `address_score`, `category_score`, `country_code_score`, `place_ids_score`, `phone_score`, `email_score`. Each is `Option<f64>` in `[0.0, 1.0]`; `None` means the field did not participate. Legacy `MatchBreakdown` fields and all per-scheme identifier sub-scores have been removed.

### Breaking — `MatchConfig`
- New weight table:
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

### Breaking — removed types and modules
- Removed: `Gender`, `BloodType`, `PassportBook`, `NicknameTable`, every national-identifier parser (UK NHS, FR NIR, ES TSI, IE IHI, UK NI H&C, US SSN, AU IHI, DE KVNR, IT CF, NL BSN, SE Personnummer, UK CHI, BE NN, BG EGN, CZ RČ, DK CPR, EE IK, ES DNI, FI HETU, HR OIB, IS KT, LT AK, LV PK, MT ID, NO FNR, PL PESEL, RO CNP, SI EMŠO, SK RČ, UK NINO, GR DSS, LI ID, NL ID, PL NIP, PT NIF, BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID), every per-country passport-format validator, the corresponding `Place` fields, `MatchConfig` weights, and `MatchBreakdown` sub-scores.
- Removed modules: `src/identifiers.rs`, `src/nicknames.rs`.
- Removed model fields and helpers tied to the prior domain: `previous_addresses`, `middle_name`, `birth_place`, `death_place`, `multiple_birth`, `death_date`, `date_of_birth`.

### Breaking — deterministic match
- Deterministic match is now defined as: any shared `(scheme, value)` pair in `place_ids`, OR identical normalised `name` plus identical normalised `address.postcode`.

### Added
- `PlaceCategory` enum — coarse-grained category (Hotel, Restaurant, Cafe, Bar, Shop, Mall, Hospital, School, University, Library, Museum, Theatre, Cinema, Park, Beach, Stadium, Airport, RailwayStation, BusStation, Bank, PostOffice, Government, Monument, ReligiousBuilding, Cemetery, Mountain, Lake, River, City, Town, Village, Neighborhood, OfficeBuilding, Residence, Warehouse, Other(String)). `#[non_exhaustive]`.
- `PlaceId { scheme: PlaceIdScheme, value: String }` and `PlaceIdScheme` enum (Google, OsmNode, OsmWay, OsmRelation, GeoNames, Wikidata, Foursquare, Here, Mapbox, Other(String)). `#[non_exhaustive]`. `PlaceId::new` trims and rejects empty values.
- Geographic primitives on `Scorer`:
  - `Scorer::haversine_metres(lat1, lon1, lat2, lon2)` — great-circle distance in metres using Earth radius `6_371_000` m. Total over `f64`; handles equator and date-line crossings.
  - `Scorer::coordinates_score(distance_metres, scale_metres)` — Gaussian decay `exp(-(d/s)^2)` clamped to `[0.0, 1.0]`. Non-finite / non-positive scale / negative distance yields `0.0`.
- Geographic fields on `Place`: `latitude`, `longitude`, `altitude_as_metre`, `elevation_as_metre`, `area_as_metre_2`, `country_code_as_iso_3166_1_alpha_2`, `maximum_capacity_count`.
- `name + alternate_names` best-of cartesian product name scoring.
- `MatchingError` is unchanged in shape (`#[non_exhaustive]`, single variant `MissingField`) but is now returned only by `Place::validate` against the new `name` requirement.

### Dependencies
- Dropped: `nhs-number` (no per-scheme identifier parsers remain).
- Retained: `chrono`, `serde`, `serde_json`, `unicode-normalization`, `strsim`, `thiserror`, `soundex`. `chrono` is no longer used by the library surface and is a candidate for removal in a follow-up; it stays in the manifest for now to keep this release scoped to documentation.

