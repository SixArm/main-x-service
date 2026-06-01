# Person matcher — Living Specification

> Canonical SDD specification for the `person-matcher` Rust crate; the single source of truth. Consolidates what an SDD workflow would split across `spec.md` / `plan.md` / `tasks.md` (see §9–§13 for plan content, §23 plus `AGENTS/delivered-tasks.md` for tasks). When something changes in the codebase, this document changes first.
>
> Version 0.3.0 · Maintainer Joel Parker Henderson (`joel@joelparkerhenderson.com`) · Crate `person-matcher` (Cargo) · Edition Rust 2024 · Licence MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause · Repository https://github.com/sixarm/person-matcher-rust-crate · See also [index.md](./index.md), [AGENTS.md](./AGENTS.md), [README.md](./README.md), [CHANGELOG.md](./CHANGELOG.md).

---

## Table of Contents

§1 Purpose and Vision · §2 Scope · §3 Stakeholders and Users · §4 Glossary · §5 Research Basis · §6 Functional Requirements · §7 Non-Functional Requirements · §8 Domain Model · §9 Architecture · §10 Component Specifications · §11 Public API Specification · §12 Algorithm Specifications · §13 Configuration Specification · §14 Normalization Specification · §15 Error Model · §16 Serialization Contract · §17 Quality Attributes · §18 Testing Strategy · §19 Build, Tooling, and Release · §20 Security, Privacy, and Compliance · §21 Roadmap and Future Work · §22 Open Questions and Risks · §23 Tasks and Acceptance Criteria · §24 Change Control · §25 References

---

## 1. Purpose and Vision

A reusable, transparent, auditable Rust library to determine whether two person demographic records refer to the same person. Targets identity exchange scenarios reconciling data and national-style identifiers from disparate source systems into a single best-guess decision.

**Vision.** Small, dependency-light, side-effect-free; combines deterministic and probabilistic matching; per-field `MatchBreakdown` on every score; configurable without sacrificing safe defaults; handles 42 national-identifier schemes (§6.4 / `AGENTS/national-person-identifiers.md`), a `PassportBook` model (multi-country / multi-book / time-varying), alphanumeric postcodes, international E.164 phones across 39 jurisdictions, diacritic-rich personal names; trustworthy for identity-adjacent workflows (audit trail, no silent fallbacks, no surprise IO).

**Non-Goals.** Persistent storage / databases / indexing; network calls / telemetry / background work; ML models; bulk pipelines beyond the delivered `match_one_to_many` / `rank_one_to_many`; cross-scheme identifier translation (requires a registry the library deliberately does not consult).

---

## 2. Scope

**In Scope.** Pairwise matching of two `Person` records (single-pair and batch via `match_one_to_many` / `rank_one_to_many`). Deterministic matching on any of 42 national identifiers, passport-book agreement, or full demographic tuples. Probabilistic matching with weighted per-field similarity (one independent score per scheme). String similarity (Jaro-Winkler, Levenshtein, Combined, Exact), Soundex phonetics, DOB transposition heuristic, nickname-table boost, middle-name blend. Normalisation of names, postcodes, phones (legacy + E.164 across 39 jurisdictions), email (opt-in Gmail folding), structured address lines, per-scheme identifiers. Address structural comparison; place-of-birth / death-place / death-date scoring. `serde` (JSON-first) round-trip. Configurable via `MatchConfig` (default / strict / lenient presets).

**Out of Scope (Today).** Blocking / candidate generation across large datasets; persistent master person indices; population-scale record linkage (Fellegi-Sunter EM training); external postal-address standardisation (declined per T-14); cross-scheme identity resolution.

---

## 3. Stakeholders and Users

Crate maintainer (Joel Parker Henderson); Rust crate consumers (stable documented public API, predictable SemVer); identity-exchange integrators (reusable matching primitive that drops into HIE pipelines without IO, runtimes, or hidden state); audit reviewers (explainability + auditability); information-governance teams (no PII leaves the process or is logged); end users with diacritic names (Unicode correctness — `â`, `ŷ`, `é`, `ü`, `ö`, `ł`).

---

## 4. Glossary

- **HIE** — Health Information Exchange (cross-organisation person-data sharing).
- **PII** — Personally Identifiable Information.
- **Deterministic match** — binary same / not-same decision from exact identifier agreement.
- **Probabilistic match** — score-based decision combining weak signals.
- **Jaro-Winkler** — prefix-favouring string similarity, well-suited to person names.
- **Levenshtein** — edit-distance (insert / delete / substitute).
- **Soundex** — consonant-to-digit phonetic algorithm.
- **NFKD** — Unicode Normalization Form, Compatibility Decomposition.
- **Confidence** — qualitative score bucket: `High` / `Medium` / `Low`.

Per-scheme identifier definitions are in [AGENTS/national-person-identifiers.md](AGENTS/national-person-identifiers.md).

---

## 5. Research Basis

Grounded in: Grannis SJ et al. *Person matcher within a Health Information Exchange* (AMIA Annu Symp Proc, 2014; PMC4696093); Reisman M. *Patient Identification Techniques* (NCVHS, 2020; PMC7442501). PDFs in [`help/`](./help/).

**Findings:** real-world error rates average ~8% (can reach 20%); even best-in-class techniques top out at 90–98% accuracy; hybrid deterministic + probabilistic strategies outperform either alone; data standardisation before matching is essential (most gains come from normalisation, not cleverer scoring); single-identifier reliance is brittle — multi-factor matching is more robust.

**Application:** inputs are normalised before scoring (§14); multiple weak signals combine via weighted average (§12); match results are transparent — every component score is in `MatchBreakdown`; defaults are conservative (threshold `0.85`) and can be tightened (`strict()`) or relaxed (`lenient()`).

---

## 6. Functional Requirements

Identifiers use **MUST**/**SHOULD**/**MAY** (RFC 2119) semantics.

### 6.1 Person Model
- **FR-1 / FR-2 / FR-3** `Person` exposes the fields in §8.1 (42 national-identifier fields + `passport_books: Vec<PassportBook>`) and is constructible via a fluent `PersonBuilder` with one setter per identifier. `Person` is `Clone + PartialEq + Eq + Debug + Serialize + Deserialize`.
- **FR-4** `Person::validate()` MUST require at least one identifying field — a name (`given_name` or `family_name`), any of the 42 national identifiers, or a non-empty `passport_books`. Otherwise return `MatchingError::MissingField`.

### 6.2 Matching Engine
- **FR-5..FR-10** `MatchingEngine` (configured by `MatchConfig`) exposes `match_persons(&p1, &p2) -> MatchResult { score, is_match, confidence, breakdown }` and `deterministic_match(&p1, &p2) -> bool` (independent of `match_threshold`). `score ∈ [0.0, 1.0]`; `is_match` iff `score >= match_threshold` (modulo FR-47). Missing fields are omitted from the weighted average and reflected as `None` — never throw.
- **FR-45 / FR-46** `match_one_to_many(query, candidates) -> Vec<MatchResult>` (parallel-to-slice; empty → empty); `rank_one_to_many` (sorted by `score` descending; ascending-original-index tiebreak).
- **FR-47** Strict mode: `is_match = (score >= match_threshold) && deterministic_match(p1, p2)`; `score` and `confidence` unchanged.
- **FR-48** Address sub-score is best-of across `(p1.address ∪ p1.previous_addresses) × (p2.address ∪ p2.previous_addresses)`; `None` only when one side has no address data.
- **FR-49** With `middle_name` on both sides, given-name component blends `0.95 × given_sim + 0.05 × middle_sim` (same `name_algorithm` and nickname boost). One-sided middle skips the blend.
- **FR-50 / FR-51 / FR-52** `PassportBook { country, number, issued, expires }`; `PassportBook::new` canonicalises both fields (`None` on invalid input). `Person::passport_books`; `MatchBreakdown::passport_book_score` = `Some(1.0)` for any shared `(country, number)` pair, `Some(0.0)` when both non-empty but disjoint, `None` when either empty. `deterministic_match` fires on any shared pair; cross-country same-`number` MUST NEVER cross-match. `Person::validate()` accepts a non-empty `passport_books` as sole identifying data.
- **FR-53** `Person` and `Address` MUST carry `#[non_exhaustive]`; external construction via `Person::builder()` / `Address::new().with_*(...)`.
- **FR-37** `MatchResult::confidence` derived from `score` (≥0.90 `High`, ≥0.75 `Medium`, else `Low`); independent of `match_threshold`.
- **FR-38** DOB component score: `1.0` exact, `0.5` for same-year day/month transposition that produces a valid calendar date, `0.0` else. Probabilistic-only; `deterministic_match` requires exact `NaiveDate` equality.

### 6.3 Determinism
- **FR-11** Same inputs + config → byte-identical results (no time, no RNG, no global state).

### 6.4 National Identifier Handling

The library supports **42 national identifier schemes**, each parsed by a function in `identifiers` and scored as an independent component.

**Passport books (§6.4a).** Passport book numbers carry three properties the per-scheme `Option<String>` pattern cannot model: scheme-local provenance, multi-country (dual citizenship), and time-varying (renewal). `PassportBook` (§8.6) and `Person::passport_books` (§8.1) capture this; the matcher consumes books as a set of `(country, number)` keys; dates are audit-only (FR-50..FR-52).

**Cross-cutting identifier requirements:** **FR-13** same-scheme identifiers compare equal iff their parsers produce equal canonical forms; different-scheme identifiers MUST NEVER cross-match. **FR-14** a malformed identifier yields `<scheme>_score = None` (not `0.0`). **FR-29** `deterministic_match` returns `true` on any same-scheme equal canonical-form pair, on the passport-book branch (FR-52), or on the demographic-tuple branch (§12).

**Per-scheme parser requirements** (FR-12, FR-25..FR-28, FR-32, FR-39..FR-44, FR-54..FR-76, FR-85..FR-91 — 42 schemes). Each MUST expose `parse_<cc>_<scheme>(&str) -> Option<String>`. Per-scheme algorithms, parser names, and per-FR cross-reference in [AGENTS/national-person-identifiers.md](AGENTS/national-person-identifiers.md) and `src/identifiers.rs` rustdoc; behaviour pinned by per-parser unit + per-scheme integration tests.

- **FR-77 Passport-number format validators** in `identifiers` for CY / CZ / LI / LT / MT / NL / PT / RO / SK (rules in `AGENTS/national-person-identifiers.md`). No `Person` field — passport data flows through `Person::passport_books` per FR-50/51/52.

**Blood-type, FHIR, place fields** (all `#[serde(default)]`; excluded from `deterministic_match` and `Person::validate`):

- **FR-78 / FR-79 / FR-80** Public `BloodType` enum (8 ABO+RhD variants `APositive`..`ONegative`; serialised as `"A+"`..`"O-"`). `BloodType::parse(s)` is lenient (canonical / lowercase / word / `+VE`/`-VE` / separator-tolerant / zero-to-O). `Person::blood_type: Option<BloodType>`; score `Some(1.0)`/`Some(0.0)`/`None`; weight `0.05`.
- **FR-81 / FR-84** `Person::birth_place: Option<Address>` (FHIR `Patient.birthPlace`) and `Person::death_place: Option<Address>` are scored via `score_named_place` (`0.7 × city Jaro-Winkler + 0.3 × country exact`; single signal when only one populated; `None` when no comparable subset); weight `0.05` each.
- **FR-82** `Person::multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, 1-indexed); score `Some(1.0)`/`Some(0.0)`/`None`; weight `0.05`. Primary use: disambiguate identical twins.
- **FR-83** `Person::death_date: Option<NaiveDate>` (FHIR `Patient.deceasedDateTime`); reuses DOB transposition via `score_dob_pair`; weight `0.10`.

### 6.5 Normalization
Full algorithms in §14 / `AGENTS/normalization.md`.
- **FR-15** Names: lowercase + NFKD + drop combining marks + drop ASCII punctuation + collapse whitespace.
- **FR-16** Postcodes: uppercase + drop whitespace.
- **FR-31** Address lines: `Normalizer::normalize_address_line` (abbreviation expansion + name normalisation) and `parse_address_line` returning `ParsedAddressLine { house_number, unit, street }`.
- **FR-17 / FR-30** Phones: legacy `Normalizer::normalize_phone` (strip `0044` / `44` / leading trunk `0`) plus international `normalize_phone_e164` (`Some("+CCNNN…")` or `None`). `MatchingEngine::match_persons` MUST prefer E.164 with legacy fallback. Default country `MatchConfig::phone_default_country = Some("GB")`. Same NSN digits from different countries MUST NOT collide under E.164.
- **FR-35 / FR-36** Email: `Normalizer::normalize_email(email, gmail_dot_folding) -> Option<String>` (trim + lowercase + structural validation; opt-in Gmail dot-folding / `+tag` stripping for `gmail.com` / `googlemail.com`). `MatchBreakdown::email_score`: `Some(1.0)` for equal canonical forms, `Some(0.0)` for unequal-when-both-parse, `None` otherwise. `local_id` MUST NOT be scored (cross-organisation collision risk).

### 6.6 Configuration
- **FR-18 / FR-19 / FR-20** `MatchConfig::default()` yields weights in §13.1; `strict()` raises threshold to **0.95** and enables `strict_mode`; `lenient()` lowers threshold to **0.75**.
- **FR-21** Weights MAY sum to anything; the engine MUST renormalise by the sum of weights of participating fields.

### 6.7 Phonetic / Nickname Matching
- **FR-22 / FR-23** When `use_phonetic_matching = true` and both persons have given AND family names, compute a Soundex score; when it exceeds **0.9**, add a `0.05`-weighted bonus. Bonus only lifts.
- **FR-33 / FR-34** Public `NicknameTable` exposes `empty()`, `english()`, `with_class(names)`, `are_equivalent(a, b)`. `MatchConfig::nickname_table` defaults to `empty()` (opt-in). When `are_equivalent` is `true` for either given- or family-name pair, that component score MUST be at least **0.9**; boost never lowers.

### 6.8 Serialization
- **FR-24** `Person`, `Address`, `Gender`, `MatchResult`, `MatchBreakdown` MUST round-trip losslessly via `serde_json`.

---

## 7. Non-Functional Requirements

- **NFR-1** Performance — single pairwise match MUST complete in microseconds on commodity hardware.
- **NFR-2** Memory — no persistent allocations between calls; bounded per-call allocations proportional to input size.
- **NFR-3** Concurrency — all public types MUST be `Send + Sync` where their fields permit; engine is immutable post-construction.
- **NFR-4** Stability — public API MUST follow SemVer; pre-1.0 minors MAY break (document in CHANGELOG).
- **NFR-5** Determinism — see FR-11.
- **NFR-6** No IO — no file / network / stdio from library code (only `main.rs` demo prints).
- **NFR-7** No `unsafe` blocks.
- **NFR-8 / NFR-9** `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` MUST pass.
- **NFR-10** All public items MUST have rustdoc; doctests MUST compile.
- **NFR-11** i18n — Latin-script diacritics handled via NFKD; the same pipeline copes with any Unicode combining mark without per-language special-casing.
- **NFR-12** `cargo test` MUST pass on a fresh checkout with no environment variables.

---

## 8. Domain Model

### 8.1 `Person`

Identifier-field naming convention: `<cc>_<scheme>` where `<cc>` is the ISO 3166-1 alpha-2 country code (lower-cased).

**Identifier fields** (each `Option<String>`, parsed at match time via `identifiers::parse_<cc>_<scheme>`; at least one identifying field required per FR-4): 42 fields enumerated in §6.4.

**Demographic / FHIR / contact fields** (all optional unless noted; FR cross-refs in §6):

- Names: `given_name` (FR-4 identifying field), `middle_name` (FR-49 blend), `family_name` (identifying field) — all `Option<String>`.
- Dates: `date_of_birth: Option<NaiveDate>`; `death_date: Option<NaiveDate>` (FHIR `Patient.deceasedDateTime`; FR-83).
- `gender: Option<Gender>`; `blood_type: Option<BloodType>` (FR-78..FR-80); `multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, FR-82).
- Places: `address: Option<Address>`; `birth_place: Option<Address>` (FR-81); `death_place: Option<Address>` (FR-84); `previous_addresses: Vec<Address>` (default empty; FR-48).
- `passport_books: Vec<PassportBook>` (identifying field; FR-50..FR-52).
- Contacts: `phone: Option<String>`; `mobile: Option<String>` (fallback); `email: Option<String>` (FR-35/FR-36); `local_id: Option<String>` (NOT scored; OQ-2).

### 8.2 `Gender`

Enum variants: `Male`, `Female`, `Other`, `Unknown`.

### 8.2a `BloodType`

ABO + RhD enum (8 variants serialised as canonical short forms `"A+"`/`"A-"`/`"B+"`/`"B-"`/`"AB+"`/`"AB-"`/`"O+"`/`"O-"`). Stable for life so disagreement is reliable evidence of non-match; agreement is a weak positive signal (≈38% of US persons are O+), so weighted at `0.05` by default. `BloodType::parse(s)` is the lenient ingestion entry point.

### 8.3 `Address`

All fields are `Option<String>`: `line1`, `line2`, `city`, `county`, `postcode`, `country`.

### 8.4 `MatchResult`

`MatchResult { score: f64 ∈ [0.0, 1.0], is_match: bool, confidence: Confidence (per §12), breakdown: MatchBreakdown }`.

### 8.6 `PassportBook`

`PassportBook { country: String, number: String, issued: Option<NaiveDate>, expires: Option<NaiveDate> }`. `country` is ISO 3166-1 alpha-2 uppercased (2 ASCII letters); `number` is whitespace-stripped, uppercased, non-empty; dates are metadata only (NOT matched). `PassportBook::new(country, number) -> Option<PassportBook>` canonicalises and rejects invalid input. Derives `Debug + Clone + PartialEq + Eq + Serialize + Deserialize`; re-exported from the crate root.

### 8.5 `MatchBreakdown`

Every field is `Option<f64>`: `None` = not scored; `Some(v)` ∈ `[0.0, 1.0]`. One `<scheme>_score` per identifier (42) plus `passport_book_score`, plus demographic / FHIR: `given_name_score`, `family_name_score`, `date_of_birth_score`, `death_date_score`, `gender_score`, `blood_type_score`, `multiple_birth_score`, `birth_place_score`, `death_place_score`, `address_score`, `phone_score`, `email_score`, `phonetic_name_score`.

---

## 9. Architecture

### 9.1 Module Layout

`src/lib.rs` (public API re-exports), `src/models.rs` (Person, PersonBuilder, Address, Gender, BloodType, PassportBook), `src/identifiers.rs` (42 per-scheme parsers + 9 passport-format validators), `src/matcher.rs` (MatchConfig, MatchingEngine, MatchResult, MatchBreakdown, Confidence), `src/scorer.rs` (similarity primitives), `src/nicknames.rs` (NicknameTable), `src/normalizer.rs` (name / postcode / phone / address / phonetic / email normalisation), `src/error.rs` (MatchingError, Result), `src/main.rs` (demo binary; not library API). See `AGENTS/architecture.md` for diagrams.

### 9.2 Dependency Graph

`matcher → normalizer / scorer / models / identifiers / error`; `identifiers → united-kingdom-national-health-service-number`; `models → serde / chrono`; `scorer → strsim`; `normalizer → unicode-normalization / soundex`; `error → thiserror`. No cycles. `lib.rs` only re-exports.

### 9.3 Layering Rules

`models` MUST NOT depend on any other crate module. `identifiers` MUST NOT depend on `matcher` / `normalizer` / `scorer` — it is a leaf beneath `matcher`. `normalizer` and `scorer` MUST NOT depend on `matcher`. `matcher` is the only orchestration layer. `main.rs` is the only place that performs `println!`.

---

## 10. Component Specifications

- **§10.1 `Normalizer`** (`normalizer.rs`) — static utility struct with `pub fn (input: &str) -> ...` methods: `normalize_name`, `normalize_postcode`, `normalize_phone`, `normalize_phone_e164`, `normalize_email`, `expand_street_abbreviations`, `normalize_address_line`, `parse_address_line`, `phonetic_code`. Algorithms in §14 / [AGENTS/normalization.md](AGENTS/normalization.md).
- **§10.1a `identifiers`** (`identifiers.rs`) — free-function module: 42 per-scheme parsers + 9 passport-format validators, each `pub fn (&str) -> Option<String>`. See [AGENTS/national-person-identifiers.md](AGENTS/national-person-identifiers.md). No IO.
- **§10.2 `Scorer`** (`scorer.rs`) — similarity primitives in `[0.0, 1.0]`: `jaro_winkler_similarity` (wraps `strsim::jaro_winkler`), `levenshtein_similarity` (`1 − distance / max_len`), `exact_match`, `combined_similarity` (`0.7 × jw + 0.3 × lev`), `optional_field_score`. Empty-input convention: both empty ⇒ 1.0, one empty ⇒ 0.0. `SimilarityAlgorithm` is a `Copy` enum: `JaroWinkler | Levenshtein | Exact | Combined`.
- **§10.2a `NicknameTable`** (`nicknames.rs`) — equivalence-class lookup. Public API: `empty()`, `english()` (built-in nicknames; exact contents NOT part of the public contract), `with_class(names)` (normalises via `normalize_name`; classes with fewer than two distinct normalised entries are silently dropped), `are_equivalent(a, b)`, `is_empty()`, `len()`.
- **§10.3 `MatchingEngine`** (`matcher.rs`) — holds an immutable `MatchConfig`. Public methods: `new(config)` / `default_config()`; `match_persons(&p1, &p2) -> MatchResult`; `deterministic_match(&p1, &p2) -> bool`; `match_one_to_many(&query, &[Person]) -> Vec<MatchResult>`; `rank_one_to_many(&query, &[Person]) -> Vec<(usize, MatchResult)>`.

---

## 11. Public API Specification

Stable re-exports from `lib.rs`: `pub mod identifiers;` (42 personal-identifier parsers + 9 passport-format validators); `pub use error::{MatchingError, Result};`; `pub use matcher::{Confidence, MatchConfig, MatchResult, MatchBreakdown, MatchingEngine};`; `pub use models::{Address, BloodType, Gender, PassportBook, Person, PersonBuilder};`; `pub use nicknames::NicknameTable;`; `pub use normalizer::{Normalizer, ParsedAddressLine};`; `pub use scorer::{Scorer, SimilarityAlgorithm};`.

Stability rules: `Person` / `Address` carry `#[non_exhaustive]` (FR-53), constructed via builder or `Address::new().with_*(...)`. Adding fields → minor bump; removing / renaming → major bump; changing default weights → minor bump (CHANGELOG "Behaviour Change"); changing the meaning of `is_match` for the same `score` → major bump.

---

## 12. Algorithm Specifications

Full per-component score tables and pseudocode are in [AGENTS/matching-algorithm.md](AGENTS/matching-algorithm.md) under "Detailed Algorithm Specifications". Summary:

- **§12.1 Deterministic** — fires on same-scheme identifier agreement (any of 42 schemes), passport-book agreement, or full demographic-tuple agreement (normalised given + family + DOB + compatible gender).
- **§12.2 Component scoring** — per-field scores in `[0.0, 1.0]` or `None`. Identifiers: exact canonical equality. Names: `name_algorithm` (JW / Lev / Exact / Combined) with nickname boost to `≥ 0.9` and `0.95 × given + 0.05 × middle` blend. DOB: exact (`1.0`) or same-year day/month transposition (`0.5`). Gender / blood type / multiple birth: exact equality. Birth/death place: shared `score_named_place` (`0.7 × city + 0.3 × country`). Death date: reuses DOB transposition. Address: §12.4. Phone: E.164 preferred + legacy fallback. Email: canonical equality. Phonetic: Soundex.
- **§12.3 Probabilistic** — `score = Σ(score × weight) / Σ(weight)` over participating fields. Phonetic bonus is asymmetric (`+ s × 0.05` when `s > 0.9`); only lifts.
- **§12.4 Address sub-score** — weighted average over postcode (0.5), city (0.3), line 1 (0.2). Line 1 is a `(house_number, street)` blend: `0.6 × street_sim + 0.4 × house_score` when both have a house number, street similarity alone otherwise. Empty-address fallback `0.5`. Best-of across `(current ∪ previous_addresses)` on both sides (FR-48).
- **§12.4a/b/c Place / date-of-death sub-scores** — see AGENTS for city / country blend rules.
- **§12.5 Confidence bands** — `score ≥ 0.90 → High`, `≥ 0.75 → Medium`, else `Low`; independent of `match_threshold`.
- **§12.6 Batch** — `match_one_to_many` (parallel-to-slice); `rank_one_to_many` (sorted descending; deterministic index tie-break). Engine is `Send + Sync`; consumers layer parallelism.

All behaviour-defining numbers are pinned by AGENTS/matching-algorithm.md and the test suite.

## 13. Configuration Specification

### 13.1 Default Configuration

Threshold and `strict_mode` vary by preset; everything else is identical across presets.

- `match_threshold`: default **0.85**, strict **0.95**, lenient **0.75**.
- `strict_mode`: default `false`, strict **`true`**, lenient `false`.

Weights (all renormalised against participating fields):

- `0.30` — 42 per-scheme identifier weights (`<cc>_<scheme>_weight`, names = `Person` identifier-field names + `_weight`); `passport_book_weight`.
- `0.20` — `family_name_weight`, `date_of_birth_weight`.
- `0.15` — `given_name_weight`.
- `0.10` — `death_date_weight`.
- `0.05` — `gender_weight`, `blood_type_weight`, `multiple_birth_weight`, `address_weight`, `birth_place_weight`, `death_place_weight`, `phone_weight`, `email_weight`.

Other defaults: `use_phonetic_matching = true`; `name_algorithm = Combined`; `nickname_table = NicknameTable::empty()`; `gmail_dot_folding = false`; `phone_default_country = Some("GB")`.

### 13.2 `strict_mode` Semantics

Strict mode computes identical `score` / `confidence` / `MatchBreakdown` but tightens the binary `is_match` decision: `is_match = (score >= match_threshold) && deterministic_match(p1, p2)`. A fuzzy match clearing the threshold but lacking a deterministic anchor is rejected (FR-47).

---

## 14. Normalization Specification

Full algorithms are in [AGENTS/normalization.md](AGENTS/normalization.md) under "Detailed Normalisation Specifications". Summary:

- **Names** (`normalize_name`) — NFKD + drop combining marks + drop ASCII punctuation + lowercase + collapse whitespace. (`José` → `jose`.)
- **Postcodes** (`normalize_postcode`) — drop whitespace, uppercase (`CF10 1AA` → `CF101AA`).
- **Phones legacy** (`normalize_phone`) — UK-centric: keep digits, strip `0044` / `44` / leading `0`; infallible fallback.
- **Phones E.164** (`normalize_phone_e164`) — match `+CC` / `00CC` / `default_country` against the 39-jurisdiction `COUNTRY_PHONE_TABLE`, strip national trunk prefix, validate NSN length; return `+CCNNN…` or `None`.
- **Email** (`normalize_email`) — trim + lowercase + structural validation; opt-in Gmail dot-/`+tag`-folding for `gmail.com` / `googlemail.com`.
- **Address lines** (`expand_street_abbreviations`, `normalize_address_line`, `parse_address_line`) — token-level abbreviation expansion + name normalisation; `parse_address_line` returns `ParsedAddressLine { house_number, unit, street }`.
- **Phonetic** (`phonetic_code`) — name normalisation then American Soundex.
- **National identifiers** (`identifiers::parse_<cc>_<scheme>`) — 42 per-scheme parsers; see `AGENTS/national-person-identifiers.md`.

Invariants: normalisers SHOULD be idempotent; identifier parsers are scheme-local (parsers sharing an algorithm MUST NOT cross-match); phone matching prefers E.164 with legacy fallback (FR-30); the 39-jurisdiction phone table covers every identifier-scheme jurisdiction (T-19); `local_id` is deliberately NOT normalised and NOT scored.

## 15. Error Model

`MatchingError` is a `thiserror`-derived enum with `#[non_exhaustive]` (future variants do not break SemVer). One variant: `MissingField(String)`. `type Result<T> = std::result::Result<T, MatchingError>;`. `MissingField` is returned by `Person::validate` when no name / identifier / `passport_books` entry is populated. The matching engine is infallible; identifier parsers return `Option<String>` rather than `Result` (the parser is the source of truth); `MatchConfig::default` / `strict` / `lenient` are infallible. Earlier `InvalidData` / `InvalidUnitedKingdomNationalHealthServiceNumber` / `InvalidDate` / `ConfigError` variants were removed in T-13 (OQ-6).

---

## 16. Serialization Contract

All public types in §11 except `MatchingEngine` MUST be `Serialize + Deserialize`. JSON is the reference format (`serde_json` hard dep). Optional fields round-trip `null` ⇄ `None`; dates serialise as ISO-8601 via `chrono`'s `serde` feature. `MatchConfig` carries `#[serde(default)]` so partial JSON deserialises with remaining fields from `MatchConfig::default()`. `SimilarityAlgorithm` serialises as the bare variant name (`"JaroWinkler"` / `"Levenshtein"` / `"Exact"` / `"Combined"`). `NicknameTable` serialises as `{ "classes": [["michael", "mike", "mickey"], …] }` (entries pre-normalised at insertion → byte-stable round-trip).

---

## 17. Quality Attributes

- **Correctness** — behaviour matches §12; verified by §18 unit + integration tests.
- **Explainability** — every score carries a per-field `MatchBreakdown`.
- **Performance** — `< 50 µs` per `match_persons` on a 2024-era Mac; verified by `benches/match_pair.rs` (criterion, T-5; single-pair fuzzy match ≈ 4 µs).
- **Maintainability** — no single file > 500 lines (`matcher.rs` exempt pending refactor).
- **Portability** — pure Rust, no C deps beyond `chrono` / `strsim` defaults.
- **Auditability** — all score combinations documented in §12.

---

## 18. Testing Strategy

### 18.1 Test Pyramid

Unit tests (`src/*.rs` `#[cfg(test)]` modules), integration tests (`tests/integration_tests.rs`), doctests (`///` examples in `lib.rs` and elsewhere), and runnable examples (`examples/basic_usage.rs`, `examples/custom_config.rs`).

### 18.2 Required Scenarios

Scenario classes (each ≥ one test): matching baselines (perfect / unrelated / missing-field); name variants (typographic / phonetic / diacritic / apostrophe); address handling (abbreviation, house-number, unit prefix, directional, mismatched-house penalty); phone normalisation (`+CC` / `00CC` / `07…`, E.164 within-country, cross-country disambiguation, legacy fallback); deterministic identifier-only match (one test per scheme: 42 + passport-book); scheme locality (UK United Kingdom National Health Service Number ≠ UK NI H&C ≠ UK CHI; AU IHI ≠ IE IHI; NL ID ≠ NL BSN; PL NIP ≠ PL PESEL; UK NINO never cross-matches); per-parser validation (canonical / case-whitespace / check-digit / length / character / variant); configuration modes (strict rejects nickname-only; lenient admits more partial; demographics-alone deterministic); DOB transposition (`0.5` swap; deterministic rejects; cross-year doesn't fire); `Person::validate` (rejects empty; accepts solo identifier / non-empty `passport_books`); serde round-trips for `Person` and `MatchResult` (all 42 identifiers, `#[serde(default)]` legacy defaulting); nicknames lift to ≥ 0.9 one-way (default empty table); email canonical equality (opt-in Gmail folding, `local_id` not scored).

### 18.3 Coverage Goals

Statement coverage SHOULD be `≥ 90%` on `src/`. Every public function MUST have at least one direct test or doctest. `cargo test` MUST complete in `< 5 s` on commodity hardware.

### 18.4 Property Tests

Delivered as T-6. `tests/property_tests.rs` uses `proptest` with **1000 cases per property**. Properties: `normalize_name` idempotence + shape (no uppercase / no leading or trailing whitespace); `score ∈ [0.0, 1.0]`; self-match `is_match == true` and `Confidence::High`; `match_persons` / `deterministic_match` symmetry; `MatchConfig::default()` and `Person` JSON round-trip; DOB sub-score order-independence; `Confidence::from_score` monotonicity. Shrunk failure seeds in `tests/property_tests.proptest-regressions`.

---

## 19. Build, Tooling, and Release

### 19.1 Toolchain

Rust edition **2024**. Standard `cargo build` / `cargo build --release` / `cargo test` (unit + integration + doctests) / `cargo clippy --all-targets -- -D warnings` / `cargo fmt`. Demo: `cargo run` (`src/main.rs`). Examples: `cargo run --example basic_usage`, `cargo run --example custom_config`.

### 19.2 Release Procedure

(1) Update `Cargo.toml` per SemVer; (2) `CHANGELOG.md` dated section; (3) update this spec if behaviour / API changed; (4) `cargo test` / `cargo clippy` / `cargo fmt --check`; (5) `cargo publish --dry-run` then `cargo publish`; (6) tag `v<version>` and push.

### 19.3 Versioning

Pre-1.0: minor bumps MAY contain breaking changes (per Cargo convention) — document prominently. Post-1.0: strict SemVer.

---

## 20. Security, Privacy, and Compliance

No IO (no file / network / socket). No logging of PII (no logging in library code at all). No global state (no thread-locals, no `static mut`, no lazy_statics holding person data). Memory hygiene: input strings are caller-owned; the library borrows them and holds no PII beyond a single call (no zeroing required). GDPR: the library is a pure function; consumers carry GDPR responsibility for the records they pass in. Safety: per §5, no algorithm is perfect — consumers MUST treat probabilistic matches as recommendations, not decisions.

---

## 21. Roadmap and Future Work

**§21.1 Near-term (0.2.x)** — all delivered (T-1 / T-2 / T-3 / T-5 / T-6); see `AGENTS/delivered-tasks.md`.

**§21.2 Medium-term (0.3.x)** — open: T-9.1 (locale-aware phonetic encoder enum, opt-in `MatchConfig::phonetic_encoder` behind `phonetic-rphonetic` feature flag; follow-up to T-9 spike). Delivered: T-10 / T-22 / T-11 / T-24 / T-25.

**§21.3 Longer-term (0.4.x – 1.0)** — optional `match_many_to_many` / blocking-key helpers atop the batch API; optional Fellegi-Sunter training; async batch evaluation; further national-identifier schemes beyond 42 (HK / SG / KR / TR / RU / AR / CA-provincial), incremental per consumer demand; 1.0 API freeze. Declined (`AGENTS/roadmap-research.md`): external postal-address standardisation (T-14); full ~250-territory ITU-T phone expansion + `phonenumber` dep (T-19); per-country mobile/landline phone validation.

### 21.4 Research Spike Outcomes

Full write-ups for the four closed spikes are in [`AGENTS/roadmap-research.md`](AGENTS/roadmap-research.md). Summary: **T-17** recommended the 7-jurisdiction batch (BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID); shipped as T-17.1, total 42. **T-9** keep Soundex default; add opt-in `phonetic_encoder` (`Soundex` / `DoubleMetaphone` / `DaitchMokotoff`) behind `phonetic-rphonetic`; tracked as T-9.1. **T-19** tactical expansion `COUNTRY_PHONE_TABLE` 26 → 39; declined full ~250 ITU-T expansion, `phonenumber` dep, mobile/landline prefix validation. **T-14** declined external postal-address standardisation at this layer.

---

## 22. Open Questions and Risks

Open questions are tracked here until resolved.

- **OQ-1..OQ-6 — Resolved.** Middle-name scoring (T-25 / FR-49); email + `local_id` (T-11 / FR-35/36); `#[non_exhaustive]` on `Person` / `Address` (T-8 / FR-53); address sub-score weighted average (T-3 / §12); strict-mode enforcement (T-4 / FR-47); `MatchingError` cleanup leaving only `MissingField` (T-13).
- **OQ-7 — Open.** Should the phonetic bonus participate in `total_weight` only when applied (current) or always? Current behaviour is judged correct; the OQ tracks the intent to document it explicitly.

### 22.1 Risks

- Misuse as a decision oracle (Med/High) — documentation; require `MatchBreakdown` on every call.
- Diacritic-heavy name false negatives (Med/Med) — NFKD pipeline; T-9.1 phonetic encoder follow-up.
- Spec / code drift (High/Med) — T-7 CI check.
- Soundex collisions cluster too aggressively (Med/Low) — phonetic is bonus-only.
- `united-kingdom-national-health-service-number` dep becomes unmaintained (Low/Med) — pin minor version; vendored fallback documented.
- Cross-scheme identifier confusion (Med/High) — FR-13 forbids cross-scheme equality; consumers must record provenance at ingest.
- ES TSI lenient validation admits malformed regional values (Med/Low) — deliberate; consumers may layer a community-specific check.

---

## 23. Tasks and Acceptance Criteria

Tasks tagged `T-NN`; status `[ ]` open, `[~]` in progress, `[x]` done. Delivered tasks with full acceptance criteria are archived in [`AGENTS/delivered-tasks.md`](AGENTS/delivered-tasks.md) (summary) and [`AGENTS/delivered-tasks-detail.md`](AGENTS/delivered-tasks-detail.md). This section keeps only currently-open tasks.

### 23.1 Done (carried over from CHANGELOG)

Full list in [`AGENTS/delivered-tasks.md`](AGENTS/delivered-tasks.md); covers the core engine (T-1..T-8 / T-13 / T-15), 42 identifier schemes + 9 passport-format validators (T-16 / T-21 / T-23 / T-27 / T-28 / T-17.1), 39-jurisdiction phone E.164 (T-18 / T-19), address parsing + `previous_addresses` (T-20 / T-24), nickname / middle-name / DOB-transposition / email scoring (T-10 / T-25 / T-22 / T-11), passport books / blood type / multi-birth / birth+death (T-26 / T-29 / T-30 / T-31 / T-32), benchmarks / property tests / drift CI / doc harmonisation (T-5 / T-6 / T-7 / T-12), and the T-9 / T-14 / T-17 / T-19 research spike outcomes.

### 23.2 Open tasks

**T-9.1 — Phonetic encoder enum (implementation follow-up to T-9).**
- [ ] Add `rphonetic` as an optional dep behind the `phonetic-rphonetic` Cargo feature flag.
- [ ] Add `PhoneticEncoder` enum (`Soundex` default + `DoubleMetaphone` + `DaitchMokotoff`) and `MatchConfig::phonetic_encoder` field; default preserves current behaviour.
- [ ] Refactor `Normalizer::phonetic_code(name)` → `phonetic_code(name, encoder)` (additive overload).
- [ ] Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
- [ ] Test multi-code semantics for Daitch-Mokotoff: non-empty code-set intersection → `1.0`, single-name match → `0.5`, disjoint → `0.0`.
- **Acceptance:** default-config behaviour and existing tests unchanged; new unit tests cover Double Metaphone (`"Stephen"`/`"Steven"`) and Daitch-Mokotoff (`"Schwarz"`/`"Shvarts"`); documented as opt-in only until T-9's corpus methodology is run.

### 23.3 Acceptance Criteria — Project-level

"1.0-ready" when all §21.1 tasks complete; spec and code agree (T-7 enforced); `Person` / `Address` `#[non_exhaustive]` (T-8); public API unchanged for two consecutive minor releases; coverage `≥ 90%` and `cargo test` in `< 5 s`.

---

## 24. Change Control

**Authority.** This file is **the** specification. Behavioural changes MUST update this file in the same PR as the code. Spec-only PRs are acceptable for documenting existing behaviour or recording a decision; editorial fixes (typos, formatting) MAY be batched. Section numbering is stable — prefer appending. `CHANGELOG.md` records *what changed*; this spec records *what is*.

**SDD workflow.** SDD artefacts live in this one document: **Specification** → §1 / §2 / §3 / §6 / §7; **Plan** → §8–§20; **Forward look** → §21 / §22; **Tasks** → §23 plus `AGENTS/delivered-tasks.md` / `AGENTS/delivered-tasks-detail.md`; **Provenance** → `CHANGELOG.md`. No separate `plan.md` / `tasks.md`.

**Lifecycle of a change.** (1) Identify affected sections; if the spec is silent, draft an addition first. (2) Update the spec with normative text (RFC 2119 MUST / SHOULD / MAY). (3) Update or add tests. (4) Implement in `src/`. (5) Record in `CHANGELOG.md` under "Unreleased". (6) Open a PR referencing the affected sections.

**Resolving disagreements.** If the spec disagrees with the code, the spec wins (file a §23 task; never silently rewrite the spec). If two sections disagree, the more specific wins (file an editorial fix). If a contributor disagrees with a design point, propose a change to §22 rather than acting unilaterally.

---

## 25. References

1. Grannis SJ et al. *Person matcher within a Health Information Exchange.* AMIA Annu Symp Proc, 2014. (`help/`)
2. Reisman M. *Patient Identification Techniques.* NCVHS, 2020. (`help/`)
3. Winkler WE. *String Comparator Metrics and Enhanced Decision Rules in the Fellegi-Sunter Model of Record Linkage.* US Census Bureau, 1990.
4. Unicode Technical Report #15 — *Unicode Normalization Forms.*
5. Crates: `united-kingdom-national-health-service-number` (Mod-11 check digit; aliases the upstream `nhs-number` crate), `strsim`, `soundex`, `unicode-normalization`.
