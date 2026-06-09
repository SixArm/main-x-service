# Worker matcher Rust Crate

A comprehensive Rust library for matching worker records for identity exchanges.

> **Documentation index:** [`index.md`](./index.md) is the top-level map of every doc in this repo (spec, AGENTS guides, CHANGELOG, examples). Start there if you're new.

## Overview

This crate implements both **deterministic** and **probabilistic** worker matcher algorithms based on research from:

- [Person matcher within a Health Information Exchange](https://pmc.ncbi.nlm.nih.gov/articles/PMC4696093/)
- [Patient Identification Techniques – Approaches, Implications, and Findings](https://pmc.ncbi.nlm.nih.gov/articles/PMC7442501/)

## Features

- ✅ **Deterministic Matching**: Exact matches on NHS numbers and key demographics
- ✅ **Probabilistic Matching**: Fuzzy matching with configurable scoring thresholds and a `Confidence` band (High/Medium/Low) derived from the score for triage UIs
- ✅ **Batch API**: `match_one_to_many` scores one query against many candidates (output parallel to input); `rank_one_to_many` returns the same scores sorted by descending score with a deterministic tiebreak — the building block for screening against a master worker index
- ✅ **String Similarity Algorithms**: Jaro-Winkler and Levenshtein distance
- ✅ **NHS-Format Identifier Support**: Validation and normalization via the `nhs-number` crate
- ✅ **Multinational National Identifiers** (**42 schemes**): UK NHS Number, France NIR, España TSI, Éire IHI, UK Northern Ireland H&C Number, United States SSN, Australia IHI, Germany KVNR, Italy *Codice Fiscale*, Netherlands BSN, Sweden *Personnummer*, UK Scotland CHI Number, Belgium National Number, Bulgaria EGN, Czech *Rodné číslo*, Denmark CPR, Estonia *Isikukood*, Spain DNI/NIE, Finland HETU, Croatia OIB, Iceland *Kennitala*, Lithuania *Asmens kodas*, Latvia *Personas kods*, Malta National ID, Norway *Fødselsnummer*, Poland PESEL, Romania CNP, Slovenia EMŠO, Slovakia *Rodné číslo*, UK NINO, Greece DSS, Liechtenstein National ID, Netherlands National ID, Poland NIP, Portugal NIF, Brazil CPF, China Resident Identity Card, India Aadhaar, Japan My Number, Mexico CURP, New Zealand NHI, South Africa ID — each scheme-local with its own parser, weight, and breakdown score. Plus **9 per-country passport-format validators** (CY, CZ, LI, LT, MT, NL, PT, RO, SK) that feed the multi-country `PassportBook` model.
- ✅ **Passport Books**: `Vec<PassportBook>` on `Worker` carries one entry per book with explicit country provenance — supports dual / multi-citizenship, accumulates historical book numbers as passports are renewed, and treats any shared `(country, number)` pair as a deterministic match (cross-country with same digits never matches)
- ✅ **Phonetic Matching**: Soundex-like algorithm for names (handles "Stephen" vs "Steven")
- ✅ **Blood Type Matching**: `BloodType` enum (8 ABO+RhD variants) with a lenient parser accepting canonical (`A+`), word (`A positive`), and zero-to-O (`0+`) variants. Blood type is stable for life, so disagreement is a strong negative signal even though agreement alone is weak.
- ✅ **Place of Birth Matching**: `Worker::birth_place` reuses the existing `Address` type (FHIR `Patient.birthPlace` parity); dedicated city + country sub-score (`0.7 × Jaro-Winkler(city) + 0.3 × exact(country)` blend) is diacritic-tolerant and ignores street / postcode fields that aren't meaningful for a place of birth.
- ✅ **Multiple-Birth (Twin) Disambiguation**: `Worker::multiple_birth` carries FHIR `Patient.multipleBirth` (1-indexed birth order) — the canonical fix for identical-twin records that otherwise share name, DOB, address, and demographic data and would otherwise be wrongly merged.
- ✅ **Date of Death and Place of Death**: `Worker::death_date` (FHIR `Patient.deceasedDateTime`) and `Worker::death_place` for deceased-worker records. Death date uses the same DD/MM ↔ MM/DD transposition heuristic as date of birth; place of death shares the `0.7 × city + 0.3 × country` blend with place of birth via a shared `score_named_place` helper.
- ✅ **Nickname Matching**: Opt-in `NicknameTable` lifts the given-name score for known nicknames (Michael ↔ Mike, Elizabeth ↔ Liz, Robert ↔ Bob, …); built-in English dictionary plus user-extensible classes
- ✅ **Diacritic Handling**: Unicode normalisation so accented names match their unaccented form (Siân → Sian, José → Jose)
- ✅ **Address Normalization**: Postcode and street address comparison
- ✅ **Sophisticated Address Parsing**: `Normalizer::parse_address_line` extracts house number, unit (Flat/Apt/Suite/…), and street; `Normalizer::expand_street_abbreviations` unifies `St`/`Street`, `Rd`/`Road`, `N`/`North`, etc. so abbreviated and full forms canonicalise identically
- ✅ **Email Matching**: `Normalizer::normalize_email` canonicalises lowercase + whitespace; optional Gmail dot-folding (`j.smith@gmail.com` ≡ `jsmith@gmail.com`) and `+tag` stripping behind a config flag
- ✅ **Phone Number Normalization**: International / trunk-prefix stripping (`+44`, `0044`, leading `0`)
- ✅ **International Phone Numbers (E.164)**: `Normalizer::normalize_phone_e164` converts inputs to `+CCNNN…` form across 39 supported countries — every jurisdiction the crate parses a national identifier for (UK, FR, ES, IE, DE, IT, NL, US/CA, AU, JP, BR, BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK, …); the matcher prefers the E.164 form so a French and a UK number with overlapping digits don't collide; Lithuania's non-`0` (`8`) national trunk prefix is handled correctly
- ✅ **Configurable Weights**: Customize importance of each field
- ✅ **Serialization Support**: JSON import/export via serde for all data types and for `MatchConfig` itself — load tuning parameters from a file without recompiling

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
worker-matcher = "0.1.0"
```

## Usage

### Basic Example

```rust
use worker_matcher::{Worker, MatchingEngine, MatchConfig};
use jiff::civil::date;

fn main() {
    // Create two worker records
    let worker1 = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .date_of_birth(date(1980, 5, 15))
        .nhs_number("1234567890")
        .build();

    let worker2 = Worker::builder()
        .given_name("Jon")  // Typo
        .family_name("Smith")
        .date_of_birth(date(1980, 5, 15))
        .nhs_number("1234567890")
        .build();

    // Create matching engine with default config
    let engine = MatchingEngine::default_config();

    // Match workers
    let result = engine.match_workers(&worker1, &worker2);

    println!("Match score: {:.2}", result.score);
    println!("Is match: {}", result.is_match);
    println!("Confidence: {:?}", result.confidence); // High / Medium / Low
}
```

### Configurable Matching

```rust
use worker_matcher::{MatchConfig, MatchingEngine};

// Strict matching (exact matches required)
let strict_engine = MatchingEngine::new(MatchConfig::strict());

// Lenient matching (more forgiving for typos)
let lenient_engine = MatchingEngine::new(MatchConfig::lenient());

// Custom configuration
let custom_config = MatchConfig {
    match_threshold: 0.90,
    nhs_number_weight: 0.40,  // Increase NHS number importance
    given_name_weight: 0.15,
    family_name_weight: 0.20,
    date_of_birth_weight: 0.15,
    use_phonetic_matching: true,
    ..Default::default()
};

let engine = MatchingEngine::new(custom_config);
```

### Deterministic Matching

```rust
// Check for exact matches only
let is_deterministic_match = engine.deterministic_match(&worker1, &worker2);

if is_deterministic_match {
    println!("Exact match on NHS number or all key demographics");
}
```

### Detailed Match Breakdown

```rust
let result = engine.match_workers(&worker1, &worker2);

println!("Overall score: {:.2}", result.score);
println!("NHS number score: {:?}", result.breakdown.nhs_number_score);
println!("Given name score: {:?}", result.breakdown.given_name_score);
println!("Family name score: {:?}", result.breakdown.family_name_score);
println!("Date of birth score: {:?}", result.breakdown.date_of_birth_score);
println!("Address score: {:?}", result.breakdown.address_score);
println!("Phonetic name score: {:?}", result.breakdown.phonetic_name_score);
```

## Worker Data Model

The `Worker` struct supports:

- **NHS Number**: NHS-format 10-digit national personal identifier with Modulus-11 check digit
- **Name Fields**: First, middle, and Family names
- **Date of Birth**: Birth date for age verification
- **Gender**: Male, Female, Other, Unknown
- **Address**: Multi-line address with postcode
- **Contact**: Phone, mobile, email
- **Local ID**: Site / source-system-specific identifier

## Matching Algorithm

The matching engine uses a weighted scoring system:

| Field         | Default Weight | Purpose                             |
| ------------- | -------------- | ----------------------------------- |
| NHS Number    | 30%            | Strongest identifier when available |
| Family Name   | 20%            | Critical demographic                |
| Date of Birth | 20%            | Age verification                    |
| Given Name    | 15%            | Important but subject to nicknames  |
| Address       | 5%             | Supporting evidence                 |
| Gender        | 5%             | Supporting evidence                 |
| Phone         | 5%             | Supporting evidence                 |

**Phonetic Matching** provides bonus points when names sound similar (e.g., "Stephen" vs "Steven").

## Research Basis

### Key Findings Applied

1. **No 100% Accuracy**: Research shows even the best algorithms achieve 90-98% accuracy. This crate aims for transparency with confidence scores.

2. **Standardization Critical**: All inputs are normalized:
   - Names: lowercase, remove diacritics, trim spaces
   - Postcodes: uppercase, remove spaces
   - Phone numbers: remove formatting, handle country codes
   - NHS numbers: digits only

3. **Multi-Factor Approach**: Following research recommendations, matching uses multiple demographic fields rather than relying on a single identifier.

4. **Weighted Probabilistic Matching**: Combines multiple weak identifiers into a strong match signal, following best practices from health information exchanges.

## Testing

Run the test suite:

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_fuzzy_name_match

# Property tests (1000 proptest cases per property)
cargo test --test property_tests
```

### Benchmarks

Criterion benchmarks live in `benches/match_pair.rs` and exercise the hot paths a downstream MPI integrator will care about:

```bash
# Run all benches (HTML reports → target/criterion/)
cargo bench

# Smoke run (fast, lower statistical power)
cargo bench -- --quick

# A single bench by name
cargo bench --bench match_pair -- match_pair/fuzzy_near_match
```

Indicative numbers on a 2024 Apple Silicon machine: single-pair fuzzy match ~4 µs, deterministic identifier hit ~160 ns, batch ranking ~3 µs per candidate — well under the spec §17 budget of `< 50 µs` per pair.

### Test Coverage

- ✅ Perfect matches (100% score)
- ✅ Fuzzy name matching (typos, alternate spellings)
- ✅ Names with diacritics
- ✅ Phonetic name matching
- ✅ Phone number normalization
- ✅ Address comparison
- ✅ NHS number validation
- ✅ Deterministic matching
- ✅ Strict vs lenient modes
- ✅ Missing field handling
- ✅ Serialization/deserialization

## Example: Running the Demo

```bash
cargo run
```

This runs example scenarios including:

1. Perfect match
2. Fuzzy name match (Stephen vs Steven)
3. Names with diacritics (Siân vs Sian)
4. Address matching
5. Complete mismatch
6. Strict vs lenient comparison

## Performance Considerations

- **Time Complexity**: O(1) for deterministic matching, O(n) for string similarity
- **Memory**: Minimal allocation, uses borrowed references where possible
- **Concurrency**: Thread-safe, all operations are immutable

## Limitations

1. **No Machine Learning**: This is a rule-based system, not ML/AI
2. **Single Identifier Scheme**: Optimised for NHS-format check-digit identifiers; other national identifier schemes are not currently validated
3. **No Persistent Storage**: In-memory matching only
4. **No Batch Processing**: Processes pairs of workers

## International Phone Numbers

The crate exposes two phone normalisers:

- `Normalizer::normalize_phone(phone) -> String` — legacy UK-centric national-significant form. Infallible.
- `Normalizer::normalize_phone_e164(phone, default_country) -> Option<String>` — international E.164 form (`+CCNNN…`). Returns `None` if the input cannot be confidently parsed against the supported country table.

`MatchingEngine::match_workers` uses the E.164 form first and falls back to the legacy form when either input cannot be parsed. The default country is configured via `MatchConfig::phone_default_country` (defaults to `Some("GB")`):

```rust
use worker_matcher::{MatchConfig, MatchingEngine, Normalizer, Worker};

// Direct call:
assert_eq!(
    Normalizer::normalize_phone_e164("+44 7700 900123", Some("GB")),
    Some("+447700900123".to_string()),
);
assert_eq!(
    Normalizer::normalize_phone_e164("01 23 45 67 89", Some("FR")),
    Some("+33123456789".to_string()),
);

// Via the matcher, with a non-UK default:
let cfg = MatchConfig {
    phone_default_country: Some("FR".into()),
    ..MatchConfig::default()
};
let engine = MatchingEngine::new(cfg);
let p1 = Worker::builder()
    .given_name("Jean").family_name("Dupont")
    .phone("01 23 45 67 89").build();
let p2 = Worker::builder()
    .given_name("Jean").family_name("Dupont")
    .phone("+33 1 23 45 67 89").build();
assert_eq!(engine.match_workers(&p1, &p2).breakdown.phone_score, Some(1.0));
```

Supported countries: UK, France, Spain, Ireland, UK Northern Ireland (via GB dial code), Germany, Italy, Netherlands, Belgium, Portugal, Switzerland, Austria, Sweden, Norway, Denmark, Finland, Poland, Australia, New Zealand, US, Canada, Japan, China, India, Brazil, Mexico, South Africa. See `spec.md` §14.3.2 for the full table.

## Passport Books

Passport book numbers don't fit the per-scheme `Option<String>` national-identifier pattern: a worker may hold passports from several countries, each book has its own number, and book numbers change with each renewal. The crate models this directly with a `Vec<PassportBook>` on `Worker`:

```rust
use jiff::civil::date;
use worker_matcher::{MatchingEngine, PassportBook, Worker};

let alice = Worker::builder()
    .given_name("Alice")
    .family_name("Anderson")
    // Current UK passport
    .add_passport_book(
        PassportBook::new("GB", "123456789").unwrap()
            .with_issued(date(2024, 6, 1))
            .with_expires(date(2034, 6, 1)),
    )
    // Dual citizen: also carries a US passport
    .add_passport_book(PassportBook::new("US", "AB1234567").unwrap())
    // Historical UK book, kept for cross-system matching
    .add_passport_book(PassportBook::new("GB", "ORIGINAL000").unwrap())
    .build();

// Other system has only the historical UK book recorded.
let same_alice = Worker::builder()
    .given_name("Alice")
    .family_name("Anderson")
    .add_passport_book(PassportBook::new("GB", "original000").unwrap())
    .build();

let engine = MatchingEngine::default_config();
assert!(engine.deterministic_match(&alice, &same_alice));
```

Matching semantics:

- The `country` is part of the comparison key — a UK `AB123456` is a different identifier from a US `AB123456`, and they never cross-match.
- Any shared `(country, number)` pair across the two workers' lists is sufficient for a deterministic match. A multi-country worker matches another record that carries any one of their books.
- Historical and current books mix freely in the same `Vec`. A renewal that produces a new book number doesn't invalidate the old one for matching purposes — keep both.
- `issued` / `expires` dates are metadata for downstream display and audit; they are NOT used in matching.

## Batch Scoring

For master-worker-index workflows, screen one query against many candidates:

```rust
use worker_matcher::{MatchingEngine, Worker};

let engine = MatchingEngine::default_config();
let query = Worker::builder().given_name("Ada").family_name("Lovelace").build();
let candidates: Vec<Worker> = vec![
    Worker::builder().given_name("Grace").family_name("Hopper").build(),
    Worker::builder().given_name("Ada").family_name("Lovelace").build(),   // best match
    Worker::builder().given_name("Alan").family_name("Turing").build(),
];

// Parallel-to-input results — keep the original index by zipping:
let results = engine.match_one_to_many(&query, &candidates);
for (i, r) in results.iter().enumerate() {
    println!("candidate[{i}]: score={:.2}, match={}", r.score, r.is_match);
}

// Ranked results — best first, tied scores ordered by original index:
let ranked = engine.rank_one_to_many(&query, &candidates);
let (idx, top) = &ranked[0];
println!("best match is candidate[{idx}] with score {:.2}", top.score);
```

The engine is `Send + Sync`, so wrap calls in `rayon::par_iter` or any other parallelism primitive without changes to this crate. Candidate-blocking (Soundex prefix, postcode outward code, date-of-birth year, …) is intentionally not baked into the API — pre-filter the candidate slice in your application layer.

## Loading Config from JSON

`MatchConfig`, `SimilarityAlgorithm`, and `NicknameTable` all derive `serde::Serialize` and `serde::Deserialize`, so tuning parameters can live in a config file:

```rust
use worker_matcher::{MatchConfig, MatchingEngine};

let json = r#"{
    "match_threshold": 0.80,
    "phone_default_country": "US",
    "gmail_dot_folding": true
}"#;
// `#[serde(default)]` on MatchConfig: omitted fields inherit from MatchConfig::default().
let cfg: MatchConfig = serde_json::from_str(json).unwrap();
let engine = MatchingEngine::new(cfg);
# let _ = engine;
```

## Email Matching

Email addresses are normalised (trim + lowercase, structural validation) and compared for exact equality. The matcher writes `Some(1.0)`/`Some(0.0)` to `MatchBreakdown::email_score`, or `None` when either side is missing or malformed:

```rust
use worker_matcher::{MatchConfig, MatchingEngine, Worker};

let a = Worker::builder().given_name("Alice").family_name("Anderson")
    .email("  Alice@Example.ORG  ").build();
let b = Worker::builder().given_name("Alice").family_name("Anderson")
    .email("alice@example.org").build();
let r = MatchingEngine::default_config().match_workers(&a, &b);
assert_eq!(r.breakdown.email_score, Some(1.0));
```

Gmail's documented routing rules treat `j.smith@gmail.com`, `jsmith@gmail.com`, and `jsmith+work@gmail.com` as the same mailbox. Opt in via `MatchConfig::gmail_dot_folding`:

```rust
# use worker_matcher::{MatchConfig, MatchingEngine, Worker};
let cfg = MatchConfig { gmail_dot_folding: true, ..MatchConfig::default() };
let engine = MatchingEngine::new(cfg);

let a = Worker::builder().given_name("X").family_name("Y")
    .email("j.smith@gmail.com").build();
let b = Worker::builder().given_name("X").family_name("Y")
    .email("jsmith+work@gmail.com").build();
assert_eq!(engine.match_workers(&a, &b).breakdown.email_score, Some(1.0));
```

Note that `local_id` is **not** scored: different organisations may issue colliding values (different workers' MRNs from sites A and B can be byte-equal), so positional matching would produce false positives.

## Nickname Matching

Nicknames are an opt-in feature. Enable the built-in English dictionary on `MatchConfig::nickname_table` and the matcher will lift the per-name score to `≥ 0.9` for any pair the table considers equivalent:

```rust
use worker_matcher::{MatchConfig, MatchingEngine, NicknameTable, Worker};

let cfg = MatchConfig {
    nickname_table: NicknameTable::english(),
    ..MatchConfig::default()
};
let engine = MatchingEngine::new(cfg);

let a = Worker::builder().given_name("Michael").family_name("Jones").build();
let b = Worker::builder().given_name("Mike").family_name("Jones").build();
let r = engine.match_workers(&a, &b);
assert!(r.breakdown.given_name_score.unwrap() >= 0.9);

// Extend the dictionary with your own classes:
let cfg = MatchConfig {
    nickname_table: NicknameTable::english().with_class(["Reginald", "Reggie"]),
    ..MatchConfig::default()
};
```

The default table is empty (`NicknameTable::empty()`) so existing callers see no behaviour change. The exact contents of `NicknameTable::english()` are NOT part of the stable contract — entries may be added in minor releases. Pin a custom table via `with_class` if you need deterministic behaviour across upgrades.

## Address Parsing

`Normalizer::parse_address_line` decomposes a single-line postal address into its components:

```rust
use worker_matcher::{Normalizer, ParsedAddressLine};

let p: ParsedAddressLine = Normalizer::parse_address_line("Flat 2A, 10 Downing Street");
assert_eq!(p.unit.as_deref(),         Some("flat 2a"));
assert_eq!(p.house_number.as_deref(), Some("10"));
assert_eq!(p.street,                  "downing street");

// `normalize_address_line` expands abbreviations and applies the name pipeline:
assert_eq!(
    Normalizer::normalize_address_line("123 High St"),
    Normalizer::normalize_address_line("123 High Street"),
);
assert_eq!(
    Normalizer::normalize_address_line("45 N Park Ave"),
    "45 north park avenue",
);
```

The matcher uses this internally so `"123 High St"` and `"123 High Street"` no longer suffer a Jaro-Winkler penalty for the abbreviation. Mismatching house numbers (e.g. `"10 Downing St"` vs `"20 Downing St"`) penalise the address sub-score even when the street name is identical. See `spec.md` §12.4.1 / §14.4a for the algorithm.

## Future Enhancements

- [ ] Support for other national identifiers (SSN, etc.)
- [ ] Batch matching API for large datasets
- [ ] Machine learning integration
- [ ] Performance benchmarks
- [ ] More sophisticated address parsing
- [ ] Broader phone-number country coverage and mobile-vs-landline validation

## License

MIT or BSD or Apache-2.0 or GPL-2.0 or GPL-3.0 or contact us for more.

## Contributing

Contributions welcome! Please ensure:

- All tests pass (`cargo test`)
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)

## References

1. Grannis SJ, et al. "Person matcher within a Health Information Exchange." AMIA Annu Symp Proc. 2014.
2. Reisman M. "Patient Identification Techniques – Approaches, Implications, and Findings." NCVHS. 2020.

## Contact

For questions, issues, or contributions, contact Joel Henderson at <joel@joelparkerhenderson.com>, or open an issue on the project repository.
