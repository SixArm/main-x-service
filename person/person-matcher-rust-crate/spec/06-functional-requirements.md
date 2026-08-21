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

**Per-scheme parser requirements** (FR-12, FR-25..FR-28, FR-32, FR-39..FR-44, FR-54..FR-76, FR-85..FR-91 — 42 schemes). Each MUST expose `parse_<cc>_<scheme>(&str) -> Option<String>`. Per-scheme algorithms, parser names, and per-FR cross-reference in [agents/national-person-identifiers.md](../agents/national-person-identifiers.md) and `src/identifiers.rs` rustdoc; behaviour pinned by per-parser unit + per-scheme integration tests.

- **FR-77 Passport-number format validators** in `identifiers` for CY / CZ / LI / LT / MT / NL / PT / RO / SK (rules in `agents/national-person-identifiers.md`). No `Person` field — passport data flows through `Person::passport_books` per FR-50/51/52.

**Blood-type, FHIR, place fields** (all `#[serde(default)]`; excluded from `deterministic_match` and `Person::validate`):

- **FR-78 / FR-79 / FR-80** Public `BloodType` enum (8 ABO+RhD variants `APositive`..`ONegative`; serialised as `"A+"`..`"O-"`). `BloodType::parse(s)` is lenient (canonical / lowercase / word / `+VE`/`-VE` / separator-tolerant / zero-to-O). `Person::blood_type: Option<BloodType>`; score `Some(1.0)`/`Some(0.0)`/`None`; weight `0.05`.
- **FR-81 / FR-84** `Person::birth_place: Option<Address>` (FHIR `Patient.birthPlace`) and `Person::death_place: Option<Address>` are scored via `score_named_place` (`0.7 × city Jaro-Winkler + 0.3 × country exact`; single signal when only one populated; `None` when no comparable subset); weight `0.05` each.
- **FR-82** `Person::multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, 1-indexed); score `Some(1.0)`/`Some(0.0)`/`None`; weight `0.05`. Primary use: disambiguate identical twins.
- **FR-83** `Person::death_date: Option<NaiveDate>` (FHIR `Patient.deceasedDateTime`); reuses DOB transposition via `score_dob_pair`; weight `0.10`.

### 6.5 Normalization
Full algorithms in §14 / `agents/normalization.md`.
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

