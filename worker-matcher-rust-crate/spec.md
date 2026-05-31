# Worker matcher — Living Specification

> **Status:** Living document. Canonical SDD specification for the `worker-matcher` Rust crate — single source of truth; consolidates what would otherwise live in `spec.md` + `plan.md` + `tasks.md`. Delivered tasks archived in [`AGENTS/delivered-tasks.md`](AGENTS/delivered-tasks.md) + [`AGENTS/delivered-tasks-2.md`](AGENTS/delivered-tasks-2.md); research-spike outcomes in [`AGENTS/roadmap-research.md`](AGENTS/roadmap-research.md).
>
> **Version:** 0.3.0 · **Maintainer:** Joel Parker Henderson — `joel@joelparkerhenderson.com` · **Crate:** `worker-matcher` (Cargo) · **Edition:** Rust 2024 · **Licence:** MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause · **Repository:** https://github.com/sixarm/worker-matcher-rust-crate
>
> See also: [index.md](./index.md), [AGENTS.md](./AGENTS.md), [README.md](./README.md), [CHANGELOG.md](./CHANGELOG.md).

---

## Table of Contents

1. [Purpose and Vision](#1-purpose-and-vision)
2. [Scope](#2-scope)
3. [Stakeholders and Users](#3-stakeholders-and-users)
4. [Glossary](#4-glossary)
5. [Research Basis](#5-research-basis)
6. [Functional Requirements](#6-functional-requirements)
7. [Non-Functional Requirements](#7-non-functional-requirements)
8. [Domain Model](#8-domain-model)
9. [Architecture](#9-architecture)
10. [Component Specifications](#10-component-specifications)
11. [Public API Specification](#11-public-api-specification)
12. [Algorithm Specifications](#12-algorithm-specifications)
13. [Configuration Specification](#13-configuration-specification)
14. [Normalization Specification](#14-normalization-specification)
15. [Error Model](#15-error-model)
16. [Serialization Contract](#16-serialization-contract)
17. [Quality Attributes](#17-quality-attributes)
18. [Testing Strategy](#18-testing-strategy)
19. [Build, Tooling, and Release](#19-build-tooling-and-release)
20. [Security, Privacy, and Compliance](#20-security-privacy-and-compliance)
21. [Roadmap and Future Work](#21-roadmap-and-future-work)
22. [Open Questions and Risks](#22-open-questions-and-risks)
23. [Tasks and Acceptance Criteria](#23-tasks-and-acceptance-criteria)
24. [Change Control](#24-change-control)
25. [References](#25-references)

---

## 1. Purpose and Vision

A reusable, transparent, auditable Rust library to determine whether two worker demographic records refer to the same worker. Targets HIE scenarios where demographic data and national-style identifiers from disparate source systems must be reconciled. Small, dependency-light, side-effect-free: combines deterministic + probabilistic matching; explainable per-field breakdowns; configurable; handles 42 national identifier schemes (see [`AGENTS/national-person-identifiers.md`](AGENTS/national-person-identifiers.md)), passport books, alphanumeric postcodes, E.164 phone numbers across 39 jurisdictions, and diacritic-rich names; trustworthy for clinical-adjacent workflows (no silent fallbacks, no surprise IO).

**Non-goals.** Persistent storage / databases / indexing; network calls / telemetry; ML or trained classifiers; bulk-pipeline / blocking; cross-scheme identifier translation.

---

## 2. Scope

**In scope.** Pairwise matching of two `Worker` records. Deterministic on any of the 42 national identifiers, the passport-book branch, or the demographic-tuple branch. Probabilistic with weighted per-field similarity (one independent score per identifier scheme). String similarity (Jaro-Winkler, Levenshtein, Combined, Exact). Phonetic (Soundex) for names. Normalisation of names, alphanumeric postcodes, phone, email, address lines, per-scheme identifiers. Address structural comparison (postcode, city, line 1). `serde` JSON-first serialisation. Configurable weights / thresholds / algorithm choice.

**Out of scope (today).** Blocking / candidate generation; persistent master worker indices; population-scale Fellegi-Sunter EM training; external postal-address standardisation (declined per T-14); cross-scheme identity resolution; non-Latin-script-specific phonetic encoders (defer to T-9.1 opt-in).

---

## 3. Stakeholders and Users

Crate maintainer (Joel Parker Henderson — overall ownership). Crate consumers (Rust developers — stable, documented, SemVer-predictable API). Healthcare integrators (reusable matching primitive that drops into HIE pipelines without IO / runtimes / hidden state). Clinical safety reviewers (explainability + auditability of every match decision). Information governance teams (assurance no PII leaves the process or is logged). End users with diacritic names (correctness on `â`, `ŷ`, `é`, `ü`, `ö`, `ł`).

---

## 4. Glossary

**HIE** — Health Information Exchange (cross-organisation worker-data sharing). **PII** — Personally Identifiable Information. **Deterministic match** — binary same/not-same based on exact identifier agreement. **Probabilistic match** — score-based combination of weak signals. **Jaro-Winkler** — prefix-favouring string similarity for names. **Levenshtein** — edit-distance metric. **Soundex** — consonant-to-digit phonetic algorithm. **NFKD** — Unicode Normalization Form, Compatibility Decomposition. **Confidence** — High / Medium / Low band derived from `score`. **42 national identifier schemes** — full catalogue in [`AGENTS/national-person-identifiers.md`](AGENTS/national-person-identifiers.md).

---

## 5. Research Basis

Grounded in Grannis SJ et al. (AMIA, 2014) and Reisman M. (NCVHS, 2020); PDFs in [`help/`](./help/). Findings: real-world error rates ~8% (reach 20%); best-in-class tops 90–98%; hybrid deterministic + probabilistic beats either alone; data standardisation matters more than cleverer scoring; multi-factor more robust than single-identifier reliance. Application: inputs normalised before scoring (§14); weak signals combined via weighted average (§12.3); per-field `MatchBreakdown` transparency; conservative default threshold `0.85` tunable via `strict()` / `lenient()`.

---

## 6. Functional Requirements

Identifiers use **MUST**/**SHOULD**/**MAY** (RFC 2119) semantics. FR numbering is preserved as the canonical reference even where per-FR paragraphs were collapsed into per-group entries (full enumeration history is in the git log).

### 6.1 Worker Model

- **FR-1 / FR-2 / FR-3** Expose a `Worker` struct (§8.1: 42 national-identifier fields + `passport_books: Vec<PassportBook>` + demographics + contacts), constructible via a fluent `WorkerBuilder` with one setter per identifier; `Clone + PartialEq + Debug + Serialize + Deserialize`.
- **FR-4** `Worker::validate()` MUST require at least one of: a name (`given_name` or `family_name`), any of the 42 national identifiers, or a non-empty `passport_books` list. Otherwise it MUST return `MatchingError::MissingField`.

### 6.2 Matching Engine
- **FR-5..FR-10** Expose `MatchingEngine` configured by `MatchConfig`. `match_workers(&p1, &p2)` returns `MatchResult { score, is_match, confidence, breakdown }`. `deterministic_match(&p1, &p2) -> bool` MUST NOT depend on `match_threshold`. `score ∈ [0.0, 1.0]`. `is_match` iff `score >= match_threshold`. Missing fields MUST NOT throw — omitted from the weighted average and reflected as `None` in the breakdown.
- **FR-37 / FR-38** `MatchResult::confidence` is derived from `score` via the §12.5 band table (≥0.90 = `High`, ≥0.75 = `Medium`, else `Low`), independent of `match_threshold`. DOB score is `1.0` exact / `0.5` same-year DD/MM ↔ MM/DD transposition (swapped form valid calendar date) / `0.0` otherwise; transposition applies only to the probabilistic score (`deterministic_match` still requires exact `NaiveDate` equality).
- **FR-45..FR-48** `match_one_to_many(query, candidates)` returns `Vec<MatchResult>` parallel to the slice; `rank_one_to_many` returns `Vec<(usize, MatchResult)>` sorted by descending score with ascending-index tiebreak. Empty candidates → empty `Vec`. Under `strict_mode = true`, `is_match = (score >= match_threshold) && deterministic_match` (score and confidence unchanged). Address sub-score considers every pair from `(p1.address ∪ p1.previous_addresses) × (p2.address ∪ p2.previous_addresses)` and reports the highest; `None` only when at least one side has no address data at all.
- **FR-49 / FR-53** When both workers carry `middle_name`, given-name component blends `0.95 × given_sim + 0.05 × middle_sim` (one-sided middle is dropped from the blend). `Worker` and `Address` carry `#[non_exhaustive]`; external consumers construct via `Worker::builder()` or `Address::new()` + `with_*` fluent setters.

### 6.3 Determinism
- **FR-11** Given the same inputs and config, results MUST be byte-identical across runs (no time, no RNG, no global state).

### 6.4 National Identifier Handling

The library supports **42 national identifier schemes** (one parser per scheme in `src/identifiers.rs`; catalogue in [`AGENTS/national-person-identifiers.md`](AGENTS/national-person-identifiers.md); algorithms in [`AGENTS/normalization.md`](AGENTS/normalization.md) §14.5; check-digit rationale + sentinel-data rejections in [`AGENTS/roadmap-research.md`](AGENTS/roadmap-research.md)). Each scheme has a `Worker` field (`Option<String>`), `WorkerBuilder` setter, `MatchConfig::<scheme>_weight` (default `0.30`), `MatchBreakdown::<scheme>_score` (with `#[serde(default)]`), `deterministic_match` branch, and `Worker::validate` inclusion.

- **FR-13** Same-scheme equality only after the parser produces `Some(canonical)` for both AND the canonical strings agree. Different schemes MUST NEVER cross-match.
- **FR-14** A malformed identifier on either side yields `<scheme>_score = None` (not `0.0`).
- **FR-29** `deterministic_match` returns `true` on any same-scheme canonical-form pair, on a passport-book hit (FR-52), or on demographic-tuple agreement (§12.1).
- **FR-12 / FR-25..FR-28 / FR-32** — Original 6: UK NHS, FR NIR, ES TSI, IE IHI, UK NI H&C, US SSN.
- **FR-39..FR-44** — T-23 (6): AU IHI, DE KVNR, IT CF, NL BSN, SE Workernummer, UK Scotland CHI (scheme-local even when 10 digits agree with UK NHS / UK NI H&C).
- **FR-54..FR-71** — T-27 (18): BE NN, BG EGN, CZ RČ, DK CPR, EE IK, ES DNI, FI HETU, HR OIB, IS KT, LT AK, LV PK, MT ID, NO FNR, PL PESEL, RO CNP, SI EMŠO, SK RČ, UK NINO.
- **FR-72..FR-77** — T-28: 5 ID schemes (GR DSS, LI ID, NL ID, PL NIP, PT NIF) + 9 passport-format validators (CY, CZ, LI, LT, MT, NL, PT, RO, SK; no `Worker` field, data flows via `passport_books`).
- **FR-85..FR-91** — T-17.1 (7): BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID. Brings total to **42**.

### 6.4a Passport Books & FHIR fields

Passport book numbers do not fit the per-scheme `Option<String>` pattern: book numbers carry **scheme-local provenance** (`(country, number)` is meaningful only together), are **multi-country** (dual citizenship), and **time-varying** (change at renewal). FR-50..FR-52 specify the model; FR-78..FR-84 add FHIR-aligned demographic fields. None contribute to `deterministic_match` except FR-52.

- **FR-50 / FR-51 / FR-52** `PassportBook { country, number, issued, expires }`. `PassportBook::new` canonicalises country (trim + uppercase, exactly 2 ASCII letters) and number (whitespace stripped, uppercased), returns `None` on invalid input. Dates are metadata only. `Worker::passport_books: Vec<PassportBook>`. `passport_book_score`: `Some(1.0)` if any `(country, number)` pair is shared, `Some(0.0)` if both sides non-empty but disjoint, `None` if either empty. A non-empty list is a valid identifying field for `validate`. `deterministic_match` fires on any shared `(country, number)` pair after canonicalisation; cross-country values with the same number MUST NEVER cross-match.
- **FR-78 / FR-79 / FR-80** `BloodType` enum (8 ABO+RhD variants serialised as `"A+"`/…/`"O-"`); `BloodType::parse` accepts canonical, lowercase, word, `+VE`/`-VE`, separator-tolerant, and zero-to-O variants. `Worker::blood_type: Option<BloodType>` with `blood_type_score` = `Some(1.0)`/`Some(0.0)`/`None`. Default weight `0.05`. Not in `deterministic_match` or `validate`.
- **FR-81 / FR-82** `Worker::birth_place: Option<Address>` (FHIR `Patient.birthPlace`); `birth_place_score` blends `0.7 × Jaro-Winkler(city) + 0.3 × exact(country)` with single-signal fallback. `Worker::multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, 1-indexed birth order). Both default weight `0.05`. Primary purpose of `multiple_birth`: identical-twin disambiguation.
- **FR-83 / FR-84** `Worker::death_date: Option<NaiveDate>` (reuses DOB transposition heuristic; weight `0.10`); `Worker::death_place: Option<Address>` (parallels `birth_place`, reuses `score_named_place`; weight `0.05`).

### 6.5 Normalization

- **FR-15 / FR-16** Names: lowercased, NFKD-decomposed with combining marks removed, ASCII-punctuation stripped, whitespace collapsed. Postcodes: uppercased, whitespace removed.
- **FR-17 / FR-30** Phone has two paths: legacy national-significant (`Normalizer::normalize_phone`; UK-centric) and E.164 (`Normalizer::normalize_phone_e164` against the 39-country table). Matcher prefers E.164, falls back to legacy. `MatchConfig::phone_default_country` defaults to `Some("GB")`. Cross-country digit collisions MUST NOT score equal under E.164.
- **FR-31** Address lines normalisable via `Normalizer::normalize_address_line`; parseable via `Normalizer::parse_address_line` returning `ParsedAddressLine { house_number, unit, street }`.
- **FR-35 / FR-36** `Normalizer::normalize_email` returns `Some(canonical)` after trim + lowercase + `@` validation, else `None`. Gmail dot/+tag folding is opt-in (`gmail.com` / `googlemail.com` only). `email_score`: `Some(1.0)` equal / `Some(0.0)` unequal-but-parsing / `None`. `local_id` MUST NOT be scored.

### 6.6 Configuration / 6.7 Phonetic / 6.7a Nickname / 6.8 Serialization

- **FR-18..FR-21** `MatchConfig::default()` yields §13.1 weights; `strict()` raises threshold to `0.95` + enables `strict_mode`; `lenient()` lowers threshold to `0.75`. Weights are internally renormalised by the sum of weights of participating fields.
- **FR-22 / FR-23** When `use_phonetic_matching` is `true` and both workers have given AND family names, compute a Soundex-based phonetic score. If `> 0.9`, contribute a `0.05`-weighted bonus (asymmetric — never lowers).
- **FR-33 / FR-34** Expose a public `NicknameTable` (`empty()`, `english()`, `with_class(names)`, `are_equivalent(a, b)`); `MatchConfig::nickname_table` defaults to `empty()` (opt-in). Equivalent name pairs lift component score to `≥ 0.9`; boost never lowers.
- **FR-24** `Worker`, `Address`, `Gender`, `MatchResult`, `MatchBreakdown` MUST round-trip losslessly via `serde_json`.

## 7. Non-Functional Requirements

**NFR-1 Performance** — pairwise match MUST complete in microseconds on commodity hardware. **NFR-2 Memory** — no persistent allocations between calls; bounded per-call allocations proportional to input size. **NFR-3 Concurrency** — public types MUST be `Send + Sync` where fields permit; engine is immutable after construction. **NFR-4 Stability** — public API MUST follow SemVer; pre-1.0 minors MAY break (documented in CHANGELOG). **NFR-5 Determinism** — see FR-11. **NFR-6 No IO** — library code MUST NOT perform file / network / stdin / stdout / stderr IO (only `main.rs` demo may print). **NFR-7 No unsafe** — no `unsafe` blocks. **NFR-8 Linting** — `cargo clippy --all-targets -- -D warnings` MUST pass. **NFR-9 Formatting** — `cargo fmt --check` MUST pass. **NFR-10 Documentation** — all public items MUST have rustdoc; doctests MUST compile. **NFR-11 i18n** — Latin-script diacritics handled via NFKD; the pipeline SHOULD cope with any Unicode combining mark without per-language special-casing. **NFR-12 Reproducibility** — `cargo test` MUST pass on a fresh checkout with no environment variables.

---

## 8. Domain Model

### 8.1 `Worker`

Field naming for national identifiers: `<cc>_<scheme>` (lower-case ISO 3166-1 alpha-2). All fields optional (`Option` / `Vec`); `Worker::validate()` requires at least one identifying field (name, national identifier, or non-empty `passport_books`). `MatchConfig` in `src/matcher.rs` is the authoritative full field list. Field groups:

**Identifying** — 42 `<cc>_<scheme>: Option<String>` per FR-12..FR-91 (catalogue [`AGENTS/national-person-identifiers.md`](AGENTS/national-person-identifiers.md)); `passport_books: Vec<PassportBook>` (§8.6 / §6.4a); `given_name`, `family_name: Option<String>`.

**Demographic (scored, not identifying)** — `middle_name: Option<String>` (blend `0.05 × middle` per FR-49); `date_of_birth`, `death_date: Option<NaiveDate>`; `gender: Option<Gender>`; `blood_type: Option<BloodType>`; `multiple_birth: Option<u8>` (1-indexed birth order; twin disambiguation).

**Location** — `address: Option<Address>`; `previous_addresses: Vec<Address>` (best-of cartesian per §12.4); `birth_place`, `death_place: Option<Address>` (FHIR; city + country only). All share `Address` shape.

**Contact** — `phone: Option<String>` (E.164 + legacy fallback per FR-30); `mobile: Option<String>` (fallback when `phone` is `None`); `email: Option<String>` (canonical form per FR-35/FR-36). `local_id: Option<String>` carried but deliberately NOT scored (OQ-2 cross-org collision risk).

### 8.2 `Gender`

Enum variants: `Male`, `Female`, `Other`, `Unknown`.

### 8.2a `BloodType`

ABO + RhD enum: `A`/`B`/`AB`/`O` × `Positive`/`Negative` (8 variants), serialised as canonical short forms `"A+"`/…/`"O-"`. Stable over a lifetime (modulo bone-marrow transplant edge cases) — disagreement is strong evidence of non-match; agreement is a weak signal (default weight `0.05`). `BloodType::parse(s)` accepts canonical, word-form, separator-tolerant, and zero-to-O legacy-EMR variants.

### 8.3 `Address`

All fields are `Option<String>`: `line1`, `line2`, `city`, `county`, `postcode`, `country`.

### 8.4 `MatchResult`

`MatchResult { score: f64 (in [0.0, 1.0]), is_match: bool (score >= match_threshold), confidence: Confidence (High / Medium / Low; §12.5), breakdown: MatchBreakdown (per-field) }`.

### 8.6 `PassportBook`

`PassportBook { country: String (ISO 3166-1 alpha-2, 2 ASCII letters), number: String (non-empty, uppercased), issued, expires: Option<NaiveDate> }`. `PassportBook::new` canonicalises both fields and rejects invalid input. Dates are metadata only — NOT used in matching. `Debug + Clone + PartialEq + Eq + Serialize + Deserialize`.

### 8.5 `MatchBreakdown`

Each field `Option<f64>` (`None` = not scored; `Some(v)` ∈ `[0.0, 1.0]`). One field per scoring axis: 42 `<cc>_<scheme>_score` + `passport_book_score` + demographic + address + contact scores (`given_name_score`, `family_name_score`, `date_of_birth_score`, `gender_score`, `blood_type_score`, `multiple_birth_score`, `address_score`, `birth_place_score`, `death_date_score`, `death_place_score`, `phone_score`, `email_score`, `phonetic_name_score`). All `#[serde(default)]` so legacy payloads deserialise with `None` for later-added fields.

---

## 9. Architecture

Full per-file detail in [`AGENTS/architecture.md`](AGENTS/architecture.md).

### 9.1 Module Layout

`src/` contains: `lib.rs` (re-exports); `models.rs` (`Worker`, `WorkerBuilder`, `Address`, `Gender`, `BloodType`, `PassportBook`); `identifiers.rs` (42 parsers + 9 passport-format validators); `matcher.rs` (`MatchConfig`, `MatchingEngine`, `MatchResult`, `MatchBreakdown`); `scorer.rs` (similarity primitives); `nicknames.rs` (`NicknameTable`); `normalizer.rs` (`Normalizer`); `error.rs` (`MatchingError`, `Result`); `main.rs` (demo binary).

### 9.2 Dependency Graph

`matcher` → `normalizer`, `scorer`, `models`, `identifiers`, `error`. `identifiers` → `nhs-number`. `models` → `serde`, `chrono`. `scorer` → `strsim`. `normalizer` → `unicode-normalization`, `soundex`. `error` → `thiserror`. No cycles; `lib.rs` only re-exports.

### 9.3 Layering Rules

`models` MUST NOT depend on any other crate module. `identifiers` MUST NOT depend on `matcher`, `normalizer`, or `scorer`. `normalizer` and `scorer` MUST NOT depend on `matcher`. `matcher` is the only orchestration layer. `main.rs` is the only place that performs `println!`.

---

## 10. Component Specifications

- **`Normalizer`** (`normalizer.rs`) — `normalize_name`, `normalize_postcode`, `normalize_phone`, `normalize_phone_e164`, `normalize_email`, `normalize_address_line`, `parse_address_line`, `phonetic_code`. Algorithms in §14 / [`AGENTS/normalization.md`](AGENTS/normalization.md).
- **`identifiers`** (`identifiers.rs`) — one parser per scheme (42 total) + 9 passport-format validators (FR-77). Each parser is `&str → Option<String>`. Catalogue in [`AGENTS/national-person-identifiers.md`](AGENTS/national-person-identifiers.md); algorithms in [`AGENTS/normalization.md`](AGENTS/normalization.md) §14.5. No IO.
- **`Scorer`** (`scorer.rs`) — similarity primitives in `[0.0, 1.0]`: `jaro_winkler_similarity`, `levenshtein_similarity` (`1 − distance / max_len`), `exact_match`, `combined_similarity` (`0.7 × jw + 0.3 × lev`), `optional_field_score(opt1, opt2, algorithm)`. Empty-string: both empty ⇒ 1.0; one empty ⇒ 0.0. `SimilarityAlgorithm` is a `Copy` enum (`JaroWinkler | Levenshtein | Exact | Combined`).
- **`NicknameTable`** (`nicknames.rs`) — equivalence-class lookup: `empty()`, `english()`, `with_class(names)` (normalised via `Normalizer::normalize_name`; <2-distinct classes dropped), `are_equivalent(a, b)`, `is_empty()` / `len()`. Default English dictionary contents are NOT a public contract.
- **`MatchingEngine`** (`matcher.rs`) — immutable wrapper around `MatchConfig`. Public: `new(config)` / `default_config()`; `match_workers` (§12.3); `deterministic_match` (§12.1); `match_one_to_many` / `rank_one_to_many` (§12.6). Private helpers compute component scores + the address sub-score (`compare_addresses`).

---

## 11. Public API Specification

Stable re-exports from `lib.rs`: `identifiers` module (42 parsers + 9 passport-format validators); `MatchingError`, `Result`; `Confidence`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, `MatchingEngine`; `Address`, `BloodType`, `Gender`, `PassportBook`, `Worker`, `WorkerBuilder`; `NicknameTable`; `Normalizer`, `ParsedAddressLine`; `Scorer`, `SimilarityAlgorithm`.

**Stability rules.** `Worker` and `Address` carry `#[non_exhaustive]` (FR-53); external consumers construct via `Worker::builder()` or `Address::new()` + `with_*` fluent setters. Adding fields = minor bump; removing/renaming = major; changing default weights = minor (with CHANGELOG "Behaviour Change" entry); changing the meaning of `is_match` for the same `score` = major.

---

## 12. Algorithm Specifications

This section is a summary. Verbatim per-subsection detail (deterministic-match branch enumeration, component-scoring table, address / birth-place / death-place / DOD sub-scores, confidence bands, batch scoring) is archived in [`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md). Behaviour-defining numbers live there in canonical form; update both surfaces in lockstep.

### 12.1 Deterministic Matching

`deterministic_match` returns `true` iff any of the 42 scheme-local identifier branches fires (one per FR-12..FR-91), the passport-book branch fires (FR-52), or the demographic-tuple branch holds (normalised given + family + exact DOB + matching-or-missing gender). Identifiers are strictly scheme-local — see FR-13.

### 12.2 Component Scoring

Per-field rules (full table in [`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md)): national identifiers + passport books — exact equality of per-scheme canonical form. Names — configured `name_algorithm`; nickname boost to `≥ 0.9`; middle blended `0.95 × given + 0.05 × middle`. DOB / death date — exact `1.0` / same-year DD↔MM transposition `0.5` / else `0.0`. Gender / blood type / multiple birth — exact equality. Address — postcode `0.5` + city `0.3` + line 1 `0.2`, weighted-average; best-of across `previous_addresses`. Birth / death place — `0.7 × JW(city) + 0.3 × exact(country)`. Phone — E.164 with legacy fallback. Email — equality of canonical form. Phonetic — Soundex average; asymmetric bonus only. Missing / unparseable input on either side → `None`.

### 12.3 Probabilistic Scoring

```text
weighted_sum = Σ_field  score_field × weight_field   (over fields with score = Some)
total_weight = Σ_field  weight_field                  (over the same fields)
if phonetic_score is Some(s) and s > 0.9:
    weighted_sum += s × 0.05;  total_weight += 0.05
score = weighted_sum / total_weight   (or 0.0 if total_weight == 0)
is_match = score >= match_threshold
```

Missing fields are dropped from both numerator and denominator. The phonetic bonus is asymmetric (only ever lifts).

### 12.4 Address Sub-Score

Postcode (exact, `0.5`) + city (Jaro-Winkler, `0.3`) + line 1 (structured house-number + street, `0.2`); weighted-average; neutral `0.5` when nothing fires; best-of across `(address ∪ previous_addresses)` cartesian product.

### 12.5 Confidence Bands

`High` for `score >= 0.90`; `Medium` for `0.75 <= score < 0.90`; `Low` otherwise. Independent of `match_threshold`. `Confidence::from_score` is total over `f64` (NaN → `Low`).

### 12.6 Batch Scoring

`match_one_to_many` returns `Vec<MatchResult>` parallel to the input slice; `rank_one_to_many` returns `Vec<(usize, MatchResult)>` sorted by descending score with ascending-index tiebreak. No blocking; engine is `Send + Sync` for consumer-managed parallelism.

## 13. Configuration Specification

### 13.1 Default Configuration

Default thresholds: `match_threshold` = **0.85** (strict 0.95 / lenient 0.75). All 42 national-identifier weights + `passport_book_weight` default to `0.30` (unchanged across presets). Demographic weights: `given_name_weight` = `0.15`; `family_name_weight` and `date_of_birth_weight` = `0.20`; `death_date_weight` = `0.10`; `gender_weight`, `blood_type_weight`, `multiple_birth_weight`, `address_weight`, `birth_place_weight`, `death_place_weight`, `phone_weight`, `email_weight` = `0.05` (all unchanged across presets). Other defaults: `use_phonetic_matching = true`; `name_algorithm = Combined`; `strict_mode = false` (true under strict); `nickname_table = empty()`; `gmail_dot_folding = false`; `phone_default_country = Some("GB")`. Weights are renormalised against participating fields. `MatchConfig` in `src/matcher.rs` is the authoritative source.

### 13.2 `strict_mode` Semantics

When `strict_mode` is `true`, `is_match = (score >= match_threshold) && deterministic_match(p1, p2)`. Probabilistic `score` and `confidence` are unchanged across modes — strict mode tightens only the binary `is_match` decision, rejecting fuzzy matches that lack a deterministic anchor.

---

## 14. Normalization Specification

Verbatim per-subsection algorithms (name / postcode / phone-legacy + phone-E.164 + 39-country table / email / address-line parser / phonetic / per-scheme national-identifier normalisation) are archived in [`AGENTS/normalization.md`](AGENTS/normalization.md). Per-scheme rules also catalogued in [`AGENTS/national-person-identifiers.md`](AGENTS/national-person-identifiers.md). Update both surfaces in lockstep.

Public entry points: `Normalizer::normalize_name` (NFKD → drop combining marks → drop ASCII punctuation → lowercase → collapse whitespace); `normalize_postcode` (strip whitespace + uppercase); `normalize_phone` (legacy, UK-centric); `normalize_phone_e164` (matches `+CC` / `00CC` / `default_country` against 39-country table; strips trunk; validates NSN; returns `+CCNNN…`); `normalize_email` (trim + lowercase + `@` validation; opt-in Gmail dot/+tag folding); `normalize_address_line` / `parse_address_line` (expand abbreviations + name-normalise; parser returns `ParsedAddressLine { house_number, unit, street }`); `phonetic_code` (American Soundex on name-normalised input; T-9.1 adds opt-in alternatives); `identifiers::parse_<cc>_<scheme>` (per-scheme canonical form per FR-12..FR-91).

Design axiom (per §5): most accuracy gains come from data standardisation. Two inputs representing the same value in different textual layouts MUST canonicalise to the same string.

## 15. Error Model

`MatchingError` is a `thiserror`-derived `#[non_exhaustive]` enum; current variants: `MissingField(String)`. `type Result<T> = std::result::Result<T, MatchingError>;`. `MissingField` is returned only by `Worker::validate` when no identifying field is populated. The matching engine itself is infallible — scoring two workers always produces a `MatchResult`. Identifier parsers return `Option<String>` (the parser is the source of truth on validity). Config builders are infallible. Four legacy variants (`InvalidData`, `InvalidNhsNumber`, `InvalidDate`, `ConfigError`) were removed in T-13; `#[non_exhaustive]` keeps future additions SemVer-safe.

---

## 16. Serialization Contract

All public types in §11 except `MatchingEngine` MUST be `Serialize + Deserialize`. JSON is the reference format; `serde_json` is a hard dependency. Optional fields round-trip `null` ⇄ `None`. Dates serialise as ISO-8601 strings via `chrono`'s default `serde` feature. `MatchConfig` carries `#[serde(default)]` on the struct so partial JSON merges over `MatchConfig::default()`. `SimilarityAlgorithm` serialises as the bare variant name. `NicknameTable` serialises as `{ "classes": [["michael", "mike", "mickey"], …] }`; entries are pre-normalised at insertion time so the round-trip is byte-stable.

---

## 17. Quality Attributes

Correctness (behaviour matches §12; verified by §18 tests). Explainability (per-field `MatchBreakdown` on every call). Performance (`< 50 µs` per `match_workers` on 2024-era Mac; verified by `benches/match_pair.rs` — single-pair fuzzy ~4 µs). Maintainability (no single file > 500 lines, `matcher.rs` exempt pending refactor). Portability (pure Rust, no C deps beyond `chrono` / `strsim` defaults; `cargo build` on Linux + macOS). Auditability (all score combinations documented in §12).

---

## 18. Testing Strategy

### 18.1 Test Pyramid

Unit tests in `src/*.rs` `#[cfg(test)]`; integration tests in `tests/integration_tests.rs`; doctests in `///` examples; smoke examples in `examples/basic_usage.rs` + `examples/custom_config.rs`. Full discipline in [`AGENTS/testing.md`](AGENTS/testing.md).

### 18.2 Required Scenarios

Each category MUST have at least one test (unit, integration, or doctest). The verbatim per-scenario list (50+ cases) is in [`AGENTS/testing.md`](AGENTS/testing.md):

Demographic matching (perfect / typographic / phonetic / diacritic / apostrophe / missing-field / unrelated-low-score); address (abbreviated line 1, house-number extraction, unit prefix, directional, mismatching-house-number penalty); phone (country-code / trunk-prefix variants, E.164 within-country, cross-country disambiguation, legacy fallback); national identifiers (per-scheme of the 42 — deterministic-match on identifier alone, layout-variant equivalence, check-digit rejection, scheme-locality, embedded-date validity for CN RRN / MX CURP / ZA ID, structural-invalid rejection for US SSN); passport books (single-pair, multi-country, same-digits-different-country never match, historical pair, both-empty `None`, both-disjoint `0.0`); demographics-only deterministic match + `Worker::validate` empty / solo-identifier / solo-passport; modes & thresholds (strict rejects nicknames; lenient admits more; strict + deterministic clears); transposition heuristic (DD/MM ↔ MM/DD same-year `0.5`; cross-year `0.0`; deterministic still rejects); nicknames + email + `local_id` (English-table lifts `Mike`/`Michael` etc. to ≥ 0.9; boost never lowers; email exact / mismatch / `None`; Gmail dot-folding opt-in; `local_id` not scored); serialisation round-trip + partial-JSON merge + legacy-payload defaulting.

### 18.3 Coverage Goals

Statement coverage SHOULD be `>= 90%` on `src/`. Every public function MUST have at least one direct test or doctest. `cargo test` MUST complete in `< 5 s` on commodity hardware.

### 18.4 Property Tests

Delivered as task T-6. The harness lives in `tests/property_tests.rs` and uses `proptest` with **1000 cases per property**. Properties: `normalize_name` idempotence + output-shape invariants; `score ∈ [0.0, 1.0]` for arbitrary worker pairs; self-match → `is_match` + `Confidence::High`; `match_workers` + `deterministic_match` symmetry; DOB sub-score order-independence; `Confidence::from_score` monotonicity; `MatchConfig::default()` and arbitrary `Worker` JSON round-trips. `proptest` persists shrunk failure seeds in `tests/property_tests.proptest-regressions`.

## 19. Build, Tooling, and Release

### 19.1 Toolchain

Rust edition **2024**. Commands: `cargo build` / `cargo build --release` / `cargo test` (unit + integration + doctests) / `cargo clippy --all-targets -- -D warnings` / `cargo fmt` / `cargo run` (demo) / `cargo run --example basic_usage`. Full release discipline in [`AGENTS/release.md`](AGENTS/release.md).

### 19.2 Release Procedure

Bump `Cargo.toml` per SemVer → update `CHANGELOG.md` → update this spec if behaviour or API changed → `cargo test` + `cargo clippy` + `cargo fmt --check` → `cargo publish --dry-run` then `cargo publish` → tag `v<version>` and push.

### 19.3 Versioning

- Pre-1.0: minor bumps MAY contain breaking changes (per Cargo convention) — document them prominently.
- Post-1.0: strict SemVer.

---

## 20. Security, Privacy, and Compliance

No IO (library reads no files, makes no network calls, opens no sockets). No logging of PII (no logging in library code at all). No global state (no thread-locals, no `static mut`, no lazy_statics carrying worker data). Memory hygiene — input strings are caller-owned; the library borrows; no zeroing because the library does not hold PII beyond a single call. GDPR — the library is a pure function; consumer applications carry GDPR responsibility for records they pass in. Clinical safety — no algorithm is perfect (§5); consumers MUST treat probabilistic matches as recommendations, not decisions. Full guidance in [`AGENTS/security-and-privacy.md`](AGENTS/security-and-privacy.md).

---

## 21. Roadmap and Future Work

Near- and medium-term (0.2.x / 0.3.x) all shipped (T-1 / T-2 / T-3 / T-5 / T-6 / T-10 / T-22 / T-11 / T-24 / T-25). Per-task acceptance criteria in [`AGENTS/delivered-tasks.md`](AGENTS/delivered-tasks.md).

### 21.1 Open initiatives (0.4.x – 1.0)

T-9.1 (opt-in `MatchConfig::phonetic_encoder` enum behind a Cargo feature flag); optional `match_many_to_many` / blocking-key helpers atop the delivered batch API; optional Fellegi-Sunter weight learning (training mode); async batch evaluation with `rayon` or `tokio`; further national identifier schemes (HK, SG, KR, TR, RU, AR, CA-provincial) incremental on consumer demand; 1.0 stabilisation (ratify API surface + freeze).

### 21.2 Declined and 21.3 Research Spike Outcomes

Full rationale + per-spike outcomes in [`AGENTS/roadmap-research.md`](AGENTS/roadmap-research.md). Headline verdicts: T-17/T-17.1 grew identifier coverage to 42 schemes (further incremental on demand); T-9 keeps Soundex default + adds opt-in encoder enum (T-9.1); T-19 ships tactical 39-jurisdiction phone table + declines ITU-T full coverage / `phonenumber` dep / mobile-landline validation; T-14 declines external postal-address standardisation at this layer.

## 22. Open Questions and Risks

Resolved questions (OQ-1..OQ-6) are archived in [`AGENTS/delivered-tasks.md`](AGENTS/delivered-tasks.md) alongside the closing tasks (T-25 / T-11 / T-8 / T-3 / T-4 / T-13). Still open:

- **OQ-7** Should the phonetic bonus participate in `total_weight` only when applied (current behaviour) or always (skews the average down when phonetic is weak)? *Current behaviour is correct;* document explicitly.

### 22.1 Risks

Misuse as clinical oracle (Med/High) → documentation + per-call `MatchBreakdown`. Diacritic-heavy false negatives (Med/Med) → NFKD + T-9.1 opt-in encoder. Spec/code drift (High/Med) → T-7 CI. Soundex over-clustering (Med/Low) → phonetic is bonus-only. `nhs-number` unmaintained (Low/Med) → pin minor + vendored fallback. Cross-scheme identifier confusion (Med/High) → scheme-local matching (FR-13 / §12.1). ES TSI lenient validation (Med/Low) — deliberate.

---

## 23. Tasks and Acceptance Criteria

Single source of truth for outstanding work; absorbs what an SDD workflow would otherwise put in a separate `tasks.md`. Tasks are tagged `T-NN`. Status legend: `[ ]` open, `[~]` in progress, `[x]` done. Delivered tasks archived in [`AGENTS/delivered-tasks.md`](AGENTS/delivered-tasks.md) (T-1..T-16 + §23.1 changelog roll-up) and [`AGENTS/delivered-tasks-2.md`](AGENTS/delivered-tasks-2.md) (T-17..T-32 + project-level acceptance criteria). Only open tasks below.

### 23.1 Open tasks

**T-9.1 — Phonetic encoder enum (implementation follow-up to T-9).**
- [ ] Add `rphonetic` as optional dep behind the `phonetic-rphonetic` Cargo feature flag.
- [ ] Add `PhoneticEncoder` enum (`Soundex` default + `DoubleMetaphone` + `DaitchMokotoff`) and `MatchConfig::phonetic_encoder` field; default preserves current behaviour exactly.
- [ ] Refactor `Normalizer::phonetic_code(name)` → `Normalizer::phonetic_code(name, encoder)` (additive overload; no-encoder form retained for backward compat).
- [ ] Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
- [ ] Define + test multi-code comparison semantics for Daitch-Mokotoff (FR-22a candidate): non-empty code-set intersection → `1.0`; single-name match → `0.5`; disjoint → `0.0`.
- **Acceptance:** Default-config behaviour and existing tests unchanged. New unit tests cover Double Metaphone primary/secondary equality (`Stephen`/`Steven`) and Daitch-Mokotoff Slavic-cluster equality (`Schwarz`/`Shvarts`). Documented "opt-in only" until T-9's corpus methodology is run.

**T-17.1 (residual).**
- [ ] TSV rows in `AGENTS/national-worker-identifiers.tsv` for the 7 FR-85..FR-91 schemes (parsers shipped without their TSV rows).

---

## 24. Change Control

**24.1 Authority.** This file is the specification. Any behaviour-affecting change MUST update this file in the same PR. Section numbering is stable. `CHANGELOG.md` records what changed; this spec records what is.

**24.2 SDD workflow.** All canonical artefacts (spec / plan / tasks) live here — no separate `plan.md` or `tasks.md`. Full discipline in [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md). Sections cluster as: spec §1–§7; plan §8–§20; forward look §21–§22; tasks §23 (live) + [`AGENTS/delivered-tasks.md`](AGENTS/delivered-tasks.md) + [`AGENTS/delivered-tasks-2.md`](AGENTS/delivered-tasks-2.md); provenance in `CHANGELOG.md`.

**24.3 Lifecycle.** Identify section → update spec (RFC 2119) → update / add tests → implement in `src/` → record in `CHANGELOG.md` → open PR referencing the section(s).

**24.4 Disagreements.** Spec wins over code (file a §23 task to align). More-specific section wins over less-specific. Design disagreements → §22, not unilateral action.

## 25. References

1. Grannis SJ et al. *Worker matcher within a Health Information Exchange.* AMIA, 2014. `help/worker-matcher-within-a-health-information-exchange.pdf`.
2. Reisman M. *Patient Identification Techniques.* NCVHS, 2020. `help/healthit-worker-matcher-aggregation-and-linking-2019-08-16.pdf`.
3. Winkler WE. *String Comparator Metrics and Enhanced Decision Rules in the Fellegi-Sunter Model of Record Linkage.* US Census Bureau, 1990.
4. `nhs-number` crate: https://docs.rs/nhs-number
5. Unicode Technical Report #15: *Unicode Normalization Forms.*
6. `strsim` https://docs.rs/strsim · `soundex` https://docs.rs/soundex · `unicode-normalization` https://docs.rs/unicode-normalization
