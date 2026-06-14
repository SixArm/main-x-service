# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [index.md](./index.md) (documentation map), [spec.md](./spec/index.md) (authoritative behaviour — each entry below corresponds to a section / FR / task in the spec), [README.md](./README.md) (user-facing overview).

## [Unreleased] → 0.6.0

### Fixed — `normalize_url` idempotency on whitespace before a fragment

- `Normalizer::normalize_url` was not idempotent when a `#fragment` was
  preceded by whitespace (e.g. `"http://h/p \u{2000}#x"`): dropping the
  fragment exposed trailing whitespace that the initial trim could not
  reach, so a second pass produced a different string — violating the
  §4 idempotency contract and intermittently failing the
  `normalize_url_is_idempotent` property test on Unicode-whitespace
  seeds. The pre-fragment slice is now re-trimmed. Added a unit
  regression (`normalize_url_retrims_after_fragment_removal`) and two
  seeds to the idempotency unit test. Spec §4 unchanged (the fix
  restores documented behaviour).

### Changed — `chrono` eliminated

- Bumped to 0.6.0. `chrono` (an unused manifest dependency flagged for
  removal) is gone; `thing-matcher` carries no date dependency. See the
  Dependencies note below. No functional change.

### Added — adapter-contract test (CI guardrail for the public API)

- New `tests/adapter_contract.rs` (10 tests). Pins every public
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

**Migration note.** 0.4.0 is the first release of `thing-matcher` as a geographic thing-matcher library. Prior 0.3.x releases targeted a different domain entirely. There is **no smooth upgrade path** from 0.3.x to 0.4.0 — every public type has different fields, every scoring component has different semantics, and the `MatchConfig` weight table has been replaced. Downstream code must be rewritten against the new surface; treat the upgrade as an integration project, not a version bump. Pin to `0.3.x` if you depend on the prior behaviour and migrate when ready.

### Breaking — domain
- Crate domain changed from prior subject matter to **geographic thing matcher**. The crate now matches places such as landmarks, natural features, chain branches, and administrative areas.

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
- Dropped: `nhs-number` (no per-scheme identifier parsers remain) and `chrono` (never used by the library surface; the crate has no date dependency).
- Retained: `serde`, `serde_json`, `unicode-normalization`, `strsim`, `thiserror`, `soundex`.

## [Unreleased]

### Added (spec/code drift CI check, T-7)
- First CI workflow in the repo: `.github/workflows/spec-drift.yml`. Runs on every pull request to `main`. Fetches full git history and invokes `scripts/spec-drift-check.sh` to enforce that any `src/matcher.rs` change is accompanied by a `spec.md` update in the same PR.
- New `scripts/spec-drift-check.sh` (POSIX bash, no extra deps). Resolves the base ref, computes the changed-file set via `git merge-base` + `git diff --name-only`, applies the watched-file pattern (`^src/matcher\.rs$`), and consults `.spec-allow` for path-pattern exceptions. Exits gracefully if the base ref cannot be resolved (e.g. fork CI) so it never produces spurious failures.
- New `.spec-allow` (extended-regex path patterns, blank / `#`-prefixed lines ignored). Ships empty so the discipline starts strict.
- New `.github/pull_request_template.md` references the spec-drift check and prompts contributors for spec impact, allowlist justification, and a CHANGELOG entry.
- The script is also runnable locally pre-push: `bash scripts/spec-drift-check.sh main HEAD`.

### Added (seven next-batch national identifier schemes, T-17.1)
- **Brazil CPF** (*Cadastro de Pessoas Físicas*) — `identifiers::parse_br_cpf`, 11 digits with two Mod-11 check digits; rejects all-equal sentinel sequences.
- **China Resident Identity Card** (*居民身份证*) — `identifiers::parse_cn_rrn`, 18 chars (17 digits + check char `0..9` or `X`); validates the embedded `YYYYMMDD` date substring and the weighted Mod-11 check.
- **India Aadhaar** — `identifiers::parse_in_aadhaar`, 12 digits with Verhoeff check digit; rejects all-equal sequences and the UIDAI-reserved `0xxxxxxxxxxx` / `1xxxxxxxxxxx` prefixes.
- **Japan My Number** (*個人番号*) — `identifiers::parse_jp_my_number`, 12 digits with weighted Mod-11 check digit per the Japanese e-Gov Cabinet Order.
- **Mexico CURP** (*Clave Única de Registro de Población*) — `identifiers::parse_mx_curp`, 18 alphanumeric chars with structural validation, embedded `YYMMDD` date check, and Mod-10 weighted check digit using the standard CURP value table (`0..9` literal, `A..N` = 10..23, `Ñ` = 24, `O..Z` = 25..36).
- **New Zealand NHI** (National Health Index, original 7-char format) — `identifiers::parse_nz_nhi`, 3 letters (`A..Z` excluding `I` and `O`) + 4 digits, Mod-11 weighted check digit with letter-to-int lookup. The 2019 alphanumeric NHI revision is not supported.
- **South Africa ID Number** — `identifiers::parse_za_id`, 13 digits with embedded `YYMMDD` date substring and Luhn check digit over all 13.
- Each scheme adds: `Place::<cc>_<scheme>: Option<String>` (`#[serde(default)]`), `PlaceBuilder::<cc>_<scheme>(value)` setter, `MatchConfig::<cc>_<scheme>_weight` (default `0.30`), `MatchBreakdown::<cc>_<scheme>_score` (`Some({0.0, 1.0})` / `None`, `#[serde(default)]`), `MatchingEngine` deterministic-match branch + breakdown wiring + weighted-score wiring, and inclusion in `Place::validate`'s identifying-field set.
- **Total scheme coverage: 35 → 42.** Spec: FR-85..FR-91, §6.4 / §13.1 / §23 updates.
- 39 new unit tests + 14 new integration tests pin each scheme's canonical form, formatted-input round-trip, check-digit rejection, length / character rejection, scheme-local isolation, breakdown wiring, serde round-trip, and legacy-payload `None` defaulting.

### API surface (T-17.1)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain 7 new fields. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add `br_cpf_weight`, `cn_rrn_weight`, `in_aadhaar_weight`, `jp_my_number_weight`, `mx_curp_weight`, `nz_nhi_weight`, `za_id_weight` (or use `..MatchConfig::default()`). All new `Place` fields carry `#[serde(default)]` so legacy JSON deserialises with `None`.

### Decided (more national identifiers, T-17, no code change)
- T-17 research spike completed. The original T-17 candidate list (CHI, KVNR, Codice Fiscale, BSN, PESEL, Placenummer, IHI) all shipped under T-23 / T-27 / T-28 — total scheme coverage is now **35**. Gap analysis vs the 39-jurisdiction phone table identified the **7 jurisdictions where the crate parses phones but not identifiers** as the recommended next batch: Brazil CPF, China RRN, India Aadhaar, Japan My Number, Mexico CURP, New Zealand NHI, South Africa ID.
- Each of the 7 has a per-scheme parser sketch and check-digit algorithm documented in spec.md §21.4 (BR uses two weighted Mod-11 digits, CN combines Mod-11 with date-substring validation, IN uses Verhoeff, MX requires structural validation + Mod-10, ZA layers Luhn over a date-encoding 13-digit format, etc.).
- Implementation follow-up tracked as **T-17.1** in spec §23.2 (follows the per-scheme pattern from T-23 / T-27 / T-28; no new architectural decisions needed). Brings total coverage to 42 schemes once shipped.

### Decided (locale-aware phonetic encoder, T-9, no code change)
- T-9 research spike completed. Survey of phonetic encoders (American Soundex, Double Metaphone, NYSIIS, Daitch-Mokotoff, Beider-Morse, locale-specific) and Rust crate availability (`rphonetic` is the de facto multi-encoder port of Apache Commons Codec).
- **Recommendation: keep American Soundex as the default**, expose a `MatchConfig::phonetic_encoder` opt-in enum (`Soundex` / `DoubleMetaphone` / `DaitchMokotoff`) behind a `phonetic-rphonetic` Cargo feature flag, defer the default-switch decision until a multinational place corpus is available.
- Rationale: the phonetic component is a `0.05`-weighted **bonus that only lifts** scores (FR-22 / FR-23), so the worst-case risk of any opt-in encoder is bounded; defaulting to a new encoder without a labelled corpus is irresponsible.
- Full option matrix, sample-size proposal, corpus specification, and evaluation methodology in spec.md §21.4.
- Implementation follow-up tracked as **T-9.1** in spec §23.2 (small additive change; defer until the empirical-validation corpus exists so the opt-in ships with defensible per-jurisdiction guidance rather than blind).

### Added (broader phone country table, T-19)
- `COUNTRY_PHONE_TABLE` in `src/normalizer.rs` now covers **39 jurisdictions** — every country for which the crate parses a national identifier scheme. The 13 jurisdictions added in T-19 are: Bulgaria, Czech Republic, Estonia, Greece, Croatia, Iceland, Liechtenstein, Lithuania, Latvia, Malta, Romania, Slovenia, Slovakia.
- 6 new e164 unit tests pin Lithuania's `8` trunk prefix, Greece's no-trunk form, Romania's `0` trunk, Czech canonical form, Iceland's 7-digit NSN, and the Croatia/Slovenia overlapping-3-digit-dial-code disambiguation.

### Changed (phone country metadata, T-19)
- Refactored `CountryPhoneInfo::has_trunk_prefix: bool` → `trunk_prefix: Option<&'static str>` so non-`0` trunk conventions work cleanly. Lithuania uses `8`, not `0`; the previous `bool` shape would have produced incorrect canonicalisation. Internal type change only — no public-API impact.

### Decided (broader phone strategy, T-19, design decision)
- T-19 research spike completed. **Declined** taking a dependency on the `phonenumber` crate (Rust port of Google's libphonenumber): the marginal recall lift for thing matcher is small, the compile-time and binary-size cost is real, and the API surface overshoots the matcher's actual need (canonicalise + compare). **Declined** per-country mobile/landline prefix validation: classification doesn't change the comparison outcome and belongs in the consumer's ingest pipeline if needed.
- Full option-by-option decision matrix in spec.md §21.4.

### Decided (address-parser exploration, T-14, no code change)
- T-14 research spike completed. Recommendation: **decline** integrating an external postal-address reference (libpostal, commercial APIs, national datasets) at the thing-matcher layer. Rationale and option-by-option comparison recorded in spec.md §21.4.
- The in-house `Normalizer::parse_address_line` + `expand_street_abbreviations` (T-20) is retained as the sole address-line parser. Consumers that need higher address-matching recall SHOULD standardise upstream in their ingest pipeline.
- Two additive follow-ups identified but not scheduled: locale-aware street-type vocabulary (FR `rue`, DE `straße`, IT `via`, ES `calle`, NL `straat`, …) and an optional `uprn`-style property identifier on `Address`. Tracked in §21.4.

### Changed (documentation harmonisation, T-12)
- Every top-level doc (`README.md`, `AGENTS.md`, `spec.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `IMPLEMENTATION_SUMMARY.md`) now carries a banner pointing readers at `index.md` as the entry point. Previously several of those files had no intra-repo navigation.
- `AGENTS/national-place-identifiers.md` (35-scheme reference table) was orphaned — no doc linked to it. Now linked from both `AGENTS.md` and `index.md`.
- `IMPLEMENTATION_SUMMARY.md` carries an explicit "superseded by `spec.md`" banner so readers don't mistake the historical snapshot for current behaviour.
- All 17 intra-repo doc paths verified to resolve.

### Changed (error-model cleanup, T-13 / resolves OQ-6)
- Removed four `MatchingError` variants that no code path returned in 0.3.0: `InvalidData`, `InvalidNhsNumber`, `InvalidDate`, `ConfigError`. The 35 national-identifier parsers in the `identifiers` module return `Option<String>` (the parser is the source of truth on validity); `MatchConfig::default` / `strict` / `lenient` are infallible; the crate does not parse date strings. The reserved-namespace argument that motivated keeping them no longer applied.
- `MatchingError` is now `#[non_exhaustive]` so future fallible code paths can add variants without breaking SemVer for downstream pattern-matches.
- `MissingField` remains the sole variant. It is still returned by `Place::validate`.

### API surface (T-13)
- **Breaking (pre-1.0):** downstream code that pattern-matches on `MatchingError::InvalidData`, `InvalidNhsNumber`, `InvalidDate`, or `ConfigError` will no longer compile. Replace those arms with a catch-all `MatchingError::MissingField(msg) => …` plus a `_` wildcard (the latter is required because the enum is now `#[non_exhaustive]`).

### Added (property tests, T-6)
- New `tests/property_tests.rs` driven by `proptest 1.5` with **1000 cases per property** (pinned in `proptest_config!`). Covers `normalize_name` idempotency and lowercase/trim shape, `score ∈ [0.0, 1.0]` for arbitrary `Place` pairs, `match_places(p, p).is_match == true` for any `p` passing `validate()`, self-match always lands in `Confidence::High`, `match_places` and `deterministic_match` are both symmetric in their two arguments, the DOB sub-score is order-independent, `Confidence::from_score` is monotonic, and JSON round-trips for `MatchConfig::default()` and arbitrary `Place` records.
- `proptest` added as a dev-dependency.
- `tests/property_tests.proptest-regressions` is checked in so historical shrunk failure seeds are re-tried on every `cargo test` run (one seed persisted from initial development; passes against the corrected `confidence_is_monotonic` property).

### Added (criterion benchmarks, T-5)
- New `benches/match_pair.rs` exercising the hot paths a downstream integrator will care about: single-pair `match_places` (identical / fuzzy / unrelated), `deterministic_match`, `rank_one_to_many` (`n ∈ {10, 100, 1000}` with criterion throughput reporting), and a config-variant sweep (default vs strict vs nickname-table-loaded).
- New `[[bench]]` entry in `Cargo.toml` with `harness = false`; `criterion 0.5` added as a dev-dependency with `html_reports`.
- Indicative numbers on a 2024 Apple Silicon machine (`cargo bench --quick`): single-pair fuzzy match ~4 µs, deterministic identifier hit ~160 ns, batch ranking ~3 µs per candidate. All comfortably under the spec §17 budget of `< 50 µs` per pair.

### Changed (address sub-score arithmetic, T-3 / resolves OQ-4)
- `MatchingEngine::compare_addresses` now uses the **weighted-average** form `Σ(score × weight) / Σ(weight)` over the sub-components that fired (postcode `0.5`, city `0.3`, line 1 `0.2`). Previously the per-component scores were pre-multiplied by their weight and then averaged by **count**, producing inconsistent double-weighting and capping the practical maximum below `1.0`.
- Observable behaviour change: an exact postcode + slightly different street now clears `0.7` (it previously scored ~`0.33`); a postcode-only match now scores `1.0`; an empty address still returns the neutral `0.5` fallback. `address_score = None` semantics are unchanged.
- Spec §12.4 / §22 OQ-4 / §23 task T-3 all updated to mark the resolution.

### Added (date-of-death and place-of-death scoring)
- `Place::death_date: Option<NaiveDate>` field (FHIR `Patient.deceasedDateTime`, date precision) — spec FR-83, §12.4c, task T-32.
- `Place::death_place: Option<Address>` field (parallel to `birth_place`) — spec FR-84, §12.4b.
- `PlaceBuilder::death_date(value)` and `PlaceBuilder::death_place(value)` setters.
- `MatchConfig::death_date_weight` (default `0.10`) and `MatchConfig::death_place_weight` (default `0.05`).
- `MatchBreakdown::death_date_score` (`Some({0.0, 0.5, 1.0})` / `None`) and `MatchBreakdown::death_place_score` (`Some([0.0, 1.0])` / `None`).
- `score_death_date` reuses the existing `score_dob_pair` helper so DD/MM ↔ MM/DD transpositions on death records are recognised as half-credit, mirroring the date-of-birth behaviour.
- New `score_named_place(&Address, &Address) -> Option<f64>` free helper extracted from the prior `score_birth_place` body. `score_birth_place` and the new `score_death_place` now both delegate to it, so birth-place and death-place sub-scores share a single source of truth for the `0.7 × city + 0.3 × country` blend.
- Neither field contributes to `deterministic_match` (too weak alone) nor to `Place::validate`'s identifying-field set.

### API surface (T-32)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain two new fields. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add `death_date_weight` and `death_place_weight` (or use `..MatchConfig::default()`). Both new `Place` fields carry `#[serde(default)]` so legacy JSON deserialises with `None`.

### Added (multiple-birth disambiguation)
- `Place::multiple_birth: Option<u8>` field carrying FHIR `Patient.multipleBirth` as a 1-indexed birth order — spec FR-82, task T-31.
- `PlaceBuilder::multiple_birth(value)` setter.
- `MatchConfig::multiple_birth_weight` (default `0.05`); `MatchBreakdown::multiple_birth_score` (`Some(1.0)` / `Some(0.0)` / `None`).
- `score_multiple_birth` helper resolves the canonical identical-twin failure mode: two records that share name, DOB, address, and demographic data but record different birth orders (twin 1 vs twin 2) now produce a `0.0` per-field score, surfacing the disagreement clearly in the breakdown.
- Not part of `deterministic_match` (too weak alone) and not part of `Place::validate`'s identifying-field set.

### API surface (T-31)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain one new field. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add `multiple_birth_weight` (or use `..MatchConfig::default()`). `Place::multiple_birth` carries `#[serde(default)]` so legacy JSON deserialises with `None`.

### Added (place-of-birth scoring)
- `Place::birth_place: Option<Address>` field (FHIR `Patient.birthPlace` parity) — spec FR-81, §12.4a, task T-30.
- `PlaceBuilder::birth_place(value)` setter.
- `MatchConfig::birth_place_weight` (default `0.05`); `MatchBreakdown::birth_place_score` (`Some([0.0, 1.0])` / `None`).
- Dedicated `score_birth_place` helper considers only `city` (Jaro-Winkler) and `country` (exact) — street and postcode are not meaningful for a birth place. Blend: `0.7 × city + 0.3 × country` when both present; single signal when only one; `None` when no comparable subset. Diacritic-tolerant via the shared name-normalisation pipeline.
- Not part of `deterministic_match` (too weak alone) and not part of `Place::validate`'s identifying-field set.

### API surface (T-30)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain one new field. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add `birth_place_weight` (or use `..MatchConfig::default()`). `Place::birth_place` carries `#[serde(default)]` so legacy JSON deserialises with `None`.

### Added (blood-type scoring)
- Public `BloodType` enum (8 ABO+RhD variants) with canonical short-form serde rename (`"A+"`, `"AB-"`, etc.) — spec FR-78/79/80, §8.2a, task T-29.
- `BloodType::parse(s)` ingests canonical, lowercase, word (`"A positive"`, `"A pos"`), `+VE`/`-VE`, separator (`A_pos`, `A-neg`), and zero-to-O (`"0+"`) variants. Rare phenotypes and unparseable inputs return `None`.
- `Place::blood_type: Option<BloodType>` with `PlaceBuilder::blood_type` setter. `MatchConfig::blood_type_weight` (default `0.05`); `MatchBreakdown::blood_type_score` (`Some(1.0)` for agreement, `Some(0.0)` for disagreement, `None` for missing).
- Blood type is **not** part of `deterministic_match` (too weak as a positive signal alone) and **not** an identifying field for `Place::validate`. The disagreement signal surfaces in the breakdown so downstream consumers can flag suspicious mismatches even when the overall score remains high.

### API surface (T-29)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain one new field. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add `blood_type_weight` (or use `..MatchConfig::default()`). `Place::blood_type` carries `#[serde(default)]` so legacy JSON payloads deserialise with `None`.

### Added (five further placeal IDs + nine passport-format validators)
- Driven by `AGENTS/national-place-identifiers.tsv` (spec FR-72..77, task T-28). Total placeal-identifier schemes supported: **35**.
  - **Greece DSS** (`gr_dss`) — 10-digit Hellenic Central Securities Depository investor share code, format-only.
  - **Liechtenstein National ID** (`li_id`) — 2 letters + 8–9 digits (spec text and example differ; parser accepts both), format-only with renewal caveat.
  - **Netherlands National ID** (`nl_id`) — 9-character `[A-Z\O]{2}[A-Z0-9\O]{6}[0-9]` (the `O` letter is banned to avoid confusion with the digit `0`), distinct from the BSN.
  - **Poland NIP** (`pl_nip`) — 10-digit tax identification number with weighted Mod-11 check; remainder 10 is invalid per spec.
  - **Portugal NIF** (`pt_nif`) — 9-digit tax identification number with weighted Mod-11 check.
- Per-country **passport-number format validators** in `thing_matcher::identifiers`: `parse_cy_passport` (`E\d{6}` or `K\d{8}`), `parse_cz_passport` (8–12 digits), `parse_li_passport` (1 letter + 5 digits), `parse_lt_passport` (8 digits), `parse_mt_passport` (7 digits), `parse_nl_passport` (NL-ID shape), `parse_pt_passport` (1 letter + 6 digits), `parse_ro_passport` (2 letters + 6 digits), `parse_sk_passport` (2 letters + 7 digits). These are pure validators with no `Place` field — passport data flows through the `Place::passport_books: Vec<PassportBook>` model (per FR-50..52).

### API surface (T-28)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain 5 new fields. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add the new `*_weight` fields (or use `..MatchConfig::default()`).

### Added (eighteen additional national placeal identifiers)
- Eighteen new identifier parsers (spec FR-54..71, task T-27). Total scheme count: **30**.
  - **Belgium NN** (`be_nn`) — 11 digits, Mod-97 (pre-2000 + "2"-prefixed post-2000).
  - **Bulgaria EGN** (`bg_egn`) — 10 digits, weighted Mod-11.
  - **Czech Republic *Rodné číslo*** (`cz_rc`) — 9 or 10 digits, Mod-11 divisibility.
  - **Denmark CPR** (`dk_cpr`) — 10 digits, format-only.
  - **Estonia *Isikukood*** (`ee_ik`) — 11 digits, cascading Mod-11.
  - **Spain DNI / NIE** (`es_dni`) — 8 digits + Mod-23 control letter.
  - **Finland HETU** (`fi_hetu`) — 11 chars with century sign + Mod-31 check character.
  - **Croatia OIB** (`hr_oib`) — 11 digits, ISO 7064 MOD 11,10.
  - **Iceland *Kennitala*** (`is_kt`) — 10 digits, weighted Mod-11.
  - **Lithuania *Asmens kodas*** (`lt_ak`) — 11 digits, cascading Mod-11.
  - **Latvia *Placeas kods*** (`lv_pk`) — 11 digits, weighted Mod-11.
  - **Malta National ID** (`mt_id`) — 7 digits + letter from `{M, G, A, P, L, H, B, Z}`.
  - **Norway *Fødselsnummer*** (`no_fnr`) — 11 digits, dual Mod-11.
  - **Poland PESEL** (`pl_pesel`) — 11 digits, weighted Mod-10.
  - **Romania CNP** (`ro_cnp`) — 13 digits, weighted Mod-11.
  - **Slovenia EMŠO** (`si_emso`) — 13 digits, weighted Mod-11.
  - **Slovakia *Rodné číslo*** (`sk_rc`) — 9 or 10 digits, Mod-11 divisibility.
  - **UK NINO** (`uk_nino`) — format `AA999999A` with admin-prefix blacklist.
- Each scheme adds a `Place` field, builder setter, `MatchConfig` weight (default 0.30), `MatchBreakdown` score (with `#[serde(default)]`), and a deterministic-match branch. All are scheme-local — the three UK Mod-11 schemes (NHS, NI H&C, Scotland CHI) and the format-only UK NINO never cross-match.

### API surface (eighteen new schemes)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain 18 new fields. Code constructing `MatchConfig { … }` via struct-literal syntax MUST add the 18 new `*_weight` fields (use `..MatchConfig::default()` to absorb them automatically).

### Added (`#[non_exhaustive]` on Place and Address)
- `Place` and `Address` now carry `#[non_exhaustive]` (spec FR-53, §11 stability rules, task T-8; resolves OQ-3).
- `Address` gains fluent `with_line1` / `with_line2` / `with_city` / `with_county` / `with_postcode` / `with_country` setters so downstream consumers have an ergonomic alternative to struct-literal syntax.

### Behaviour change
- External code that constructed `Place` or `Address` via struct-literal syntax (`Place { ... }`, `Address { ... }`) will no longer compile. Use `Place::builder()` and `Address::new()` / `Address::new().with_postcode(...)` instead. Field-assignment syntax (`let mut a = Address::new(); a.line1 = Some(...)`) still works because `#[non_exhaustive]` does not block individual field access. This formalises the long-standing expectation that field additions are non-breaking under SemVer.

### Added (passport books)
- Public `PassportBook { country, number, issued, expires }` type and `Place::passport_books: Vec<PassportBook>` field (spec FR-50/51/52, §6.4a, §8.6, task T-26).
  - **Scheme-local provenance**: the comparison key is `(country, number)`. A UK `"AB123456"` is a different identifier from a US `"AB123456"`; the matcher never cross-matches them.
  - **Multi-country**: a single place may carry passports from any number of countries simultaneously — one `PassportBook` per book.
  - **Time-varying**: when a passport is renewed, the new book has a different number. Records may carry the current book, prior books, or both. Matching treats any shared `(country, number)` pair as evidence the records refer to the same place, regardless of issue date.
  - Constructor `PassportBook::new(country, number)` canonicalises country (trimmed, uppercased; must be exactly 2 ASCII letters) and number (whitespace stripped, uppercased); returns `Option<PassportBook>`. Builder methods `with_issued(date)` / `with_expires(date)` attach optional metadata (NOT used in matching).
  - `MatchConfig::passport_book_weight` (default `0.30`); `MatchBreakdown::passport_book_score: Option<f64>` with `#[serde(default)]`.
  - `MatchingEngine::deterministic_match` returns `true` when at least one `(country, number)` pair is shared after canonicalisation.
  - `Place::validate` accepts a non-empty `passport_books` as a sufficient identifying field.

### API surface (passport books)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain one new field. `passport_books` carries `#[serde(default)]` on the Place field so legacy JSON payloads deserialise cleanly with an empty list.

### Added (middle-name scoring)
- Middle-name participation in the given-name component (spec FR-49, §12.2, task T-25; resolves OQ-1).
  - When both `Place` records carry a `middle_name`, the given-name component score is blended as `0.95 × given_sim + 0.05 × middle_sim`. The middle-name similarity uses the same `name_algorithm` and `MatchConfig::nickname_table` as the given-name path, so a "James" ↔ "Jim" middle name benefits from the nickname boost just like the given name.
  - One-sided middle-name data (only one record has a middle name) leaves the score unchanged.

### Behaviour change
- Records that previously scored identically on given name alone may now score slightly differently when both sides carry a middle name. The shift is bounded by the 5% blend weight, so matches at the score boundary may move by up to ±0.05 on the given-name component (≤ 0.0075 of the overall score at default weights). Consumers reading the given-name component directly will see the new behaviour.

### Added (previous_addresses scoring)
- Best-of cartesian address matching (spec FR-48, §12.4.2, task T-24).
  - `MatchBreakdown::address_score` now reflects the highest score across every pair drawn from `(p1.address ∪ p1.previous_addresses) × (p2.address ∪ p2.previous_addresses)`.
  - Catches the "place moved house" failure mode where the current addresses no longer agree but a prior address on one side still matches the other side's current.
  - `address_score` is `None` only when at least one side has no address data at all; previously it was `None` whenever either `address` field was absent.

### Behaviour change
- Records that previously produced `MatchBreakdown::address_score = None` because one side's current address was missing may now produce `Some(score)` if either side carries `previous_addresses`. The overall match outcome rarely changes (address contributes only 5% by default), but consumers reading the breakdown directly will see the new behaviour.

### Added (strict_mode enforcement)
- `MatchConfig::strict_mode = true` now also requires a deterministic match for `MatchResult::is_match` to be `true` (spec FR-47, §13.2, task T-4; resolves OQ-5).
- Probabilistic `score` and `confidence` are unchanged across strict and non-strict configurations — strict mode tightens only the binary `is_match` decision.

### Behaviour change
- Records that produced `is_match = true` under `MatchConfig::strict()` purely on a high fuzzy score (e.g. typo near-clones) now produce `is_match = false`. The fix narrows the false-positive surface for production workflows. Consumers that relied on the previous behaviour should switch to `MatchConfig::default()` (threshold 0.85, no determinism requirement) or read the `score` / `confidence` fields directly.

### Added (batch API)
- Batch scoring (spec FR-45 / FR-46, §12.6, task T-15).
  - `MatchingEngine::match_one_to_many(query, candidates) -> Vec<MatchResult>` — scores a single query against many candidates. Output is parallel to the input slice; empty candidates yield an empty `Vec`. The building block for screening one incoming record against a main place index.
  - `MatchingEngine::rank_one_to_many(query, candidates) -> Vec<(usize, MatchResult)>` — same scoring sorted by descending `score`; ties are broken by ascending original index, so the ranking is fully deterministic across calls.
  - The engine remains `Send + Sync`, so consumers can layer their own parallelism (rayon, tokio) without changes to this crate. Blocking (candidate pre-filtering) is a consumer concern; the crate stays a pure scoring library.

### Added (six new national identifier schemes)
- Six additional national identifiers (spec FR-39/40/41/42/43/44, §6.4, §14.5, task T-23). Total schemes supported: **12** (up from 6).
  - **Australia IHI** — 16-digit Individual Healthcare Identifier with Luhn check (ISO/IEC 7812-1). Parser: `parse_au_ihi`. Builder: `Place::builder().au_ihi(...)`. `MatchBreakdown::au_ihi_score`.
  - **Germany KVNR** — 10-character *Krankenversichertennummer* (letter + 9 digits) with Mod-10 check via letter-ordinal expansion. Parser: `parse_de_kvnr`. Builder: `de_kvnr`. `MatchBreakdown::de_kvnr_score`.
  - **Italy *Codice Fiscale*** — 16-character alphanumeric tax identifier with Mod-26 check via odd/even position tables. Parser: `parse_it_cf`. Builder: `it_cf`. `MatchBreakdown::it_cf_score`.
  - **Netherlands BSN** — 9-digit *Burgerservicenummer* with the 11-test (`9d₁ + 8d₂ + … + 2d₈ − d₉ ≡ 0 mod 11`); rejects all-zero. Parser: `parse_nl_bsn`. Builder: `nl_bsn`. `MatchBreakdown::nl_bsn_score`.
  - **Sweden *Placenummer*** — 10- or 12-digit placeal identity number with Luhn check; accepts `-` / `+` separators; canonicalises preserving input length. Parser: `parse_se_placenummer`. Builder: `se_placenummer`. `MatchBreakdown::se_placenummer_score`.
  - **UK Scotland CHI Number** — 10-digit Community Health Index Number with Mod-11 check (same algorithm as NHS Number); scheme-local. Parser: `parse_uk_chi_number`. Builder: `uk_chi_number`. `MatchBreakdown::uk_chi_number_score`.
- Each scheme is **scheme-local**: no two schemes ever cross-match, even when the underlying digits coincide. This is enforced for AU IHI vs IE IHI (different lengths), and for the three UK Mod-11 schemes (NHS Number, NI H&C Number, Scotland CHI Number) which share an algorithm but distinct `Place` fields.

### API surface (six new schemes)
- `Place`, `PlaceBuilder`, `MatchConfig`, and `MatchBreakdown` each gain six new fields. Code constructing any of them via struct-literal syntax MUST add the fields (use `..MatchConfig::default()` to absorb the new weights automatically; the breakdown is built only by the engine so no consumer construction is expected).

### Added (DOB transposition heuristic)
- Probabilistic date-of-birth scoring (spec FR-38, §12.2, task T-22) now returns `0.5` partial credit when one side is a day/month transposition of the other (same year, valid swapped date). Catches the common DD/MM ↔ MM/DD data-entry bug — `1995-01-10` vs `1995-10-01` now scores `0.5` instead of `0.0`.
- `deterministic_match` is unchanged: the demographic-tuple branch still requires exact `NaiveDate` equality.

### Behaviour change
- Records that differ only by a DOB day/month transposition now produce a non-zero `MatchBreakdown::date_of_birth_score`. The overall `is_match` outcome is rarely affected because partial credit on a single component does not clear default thresholds, but consumers reading the breakdown directly will see `Some(0.5)` where they previously saw `Some(0.0)`.

---

Earlier history (pre-0.3.0) has moved to
[`CHANGELOG-archive.md`](./CHANGELOG-archive.md) to keep this file
under 40 KB.
