# Main X Index Rust crate

@AGENTS/share/overview.md

Subprojects:

- [Main Person Index Rust crate](main-person-index-rust-crate)
- [Main Place Index Rust crate](main-place-index-rust-crate)
- [Main Thing Index Rust crate](main-thing-index-rust-crate/)
- [Main Event Index Rust crate](main-event-index-rust-crate/)
- [Main Patient Index Rust crate](main-patient-index-rust-crate/)
- [Main Worker Index Rust crate](main-worker-index-rust-crate/)

## Features

### Data Management

- Create, read, update, and delete (CRUD) records
- Soft delete support with complete audit trails
- Identifier management; multiple identifiers per record.
- Identity document management; multiple identity documents per record.
- Contact information management; multiple contacts per record.
- Automatic event stream publishing for all CRUD operations

### Matching

- **Probabilistic Matching**: Advanced fuzzy matching algorithms
- **Deterministic Matching**: Rule-based exact matching
- **Configurable Scoring**: Customizable match thresholds and weights
- **Match Components**:
  - String matching (Jaro-Winkler, Levenshtein, Soundex phonetic)
  - Date matching with error tolerance
  - Identifier matching
  - Identification document matching
- **Score Breakdown**: Full per-component score breakdown in API responses

@AGENTS/index.md

### Data Quality & Validation

- Required field enforcement
- Date validation
- ID format validation
- Email format validation
- Phone number digit count validation
- Address validation (requires city, postal code, or country)
- Document validation (required number, expiry check, issue-before-expiry)
- Phone number normalization (E.164-like format)
- Address standardization (title-case city, uppercase state/country, expand abbreviations)
- Validation integrated into create and update handlers (returns 422)
