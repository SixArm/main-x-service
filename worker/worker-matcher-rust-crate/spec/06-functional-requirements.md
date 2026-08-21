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

The library supports **42 national identifier schemes** (one parser per scheme in `src/identifiers.rs`; catalogue in [`agents/national-person-identifiers.md`](../agents/national-person-identifiers.md); algorithms in [`agents/normalization.md`](../agents/normalization.md) §14.5; check-digit rationale + sentinel-data rejections in [`agents/roadmap-research.md`](../agents/roadmap-research.md)). Each scheme has a `Worker` field (`Option<String>`), `WorkerBuilder` setter, `MatchConfig::<scheme>_weight` (default `0.30`), `MatchBreakdown::<scheme>_score` (with `#[serde(default)]`), `deterministic_match` branch, and `Worker::validate` inclusion.

- **FR-13** Same-scheme equality only after the parser produces `Some(canonical)` for both AND the canonical strings agree. Different schemes MUST NEVER cross-match.
- **FR-14** A malformed identifier on either side yields `<scheme>_score = None` (not `0.0`).
- **FR-29** `deterministic_match` returns `true` on any same-scheme canonical-form pair, on a passport-book hit (FR-52), or on demographic-tuple agreement (§12.1).
- **FR-12 / FR-25..FR-28 / FR-32** — Original 6: UK NHS, FR NIR, ES TSI, IE IHI, UK NI H&C, US SSN.
- **FR-39..FR-44** — T-23 (6): AU IHI, DE KVNR, IT CF, NL BSN, SE Personnummer, UK Scotland CHI (scheme-local even when 10 digits agree with UK NHS / UK NI H&C).
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

