## 8. Domain Model

### 8.1 `Worker`

Field naming for national identifiers: `<cc>_<scheme>` (lower-case ISO 3166-1 alpha-2). All fields optional (`Option` / `Vec`); `Worker::validate()` requires at least one identifying field (name, national identifier, or non-empty `passport_books`). `MatchConfig` in `src/matcher.rs` is the authoritative full field list. Field groups:

**Identifying** — 42 `<cc>_<scheme>: Option<String>` per FR-12..FR-91 (catalogue [`AGENTS/national-person-identifiers.md`](../AGENTS/national-person-identifiers.md)); `passport_books: Vec<PassportBook>` (§8.6 / §6.4a); `given_name`, `family_name: Option<String>`.

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

