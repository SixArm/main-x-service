## 8. Domain Model

### 8.1 `Worker`

Field naming for national identifiers: `<cc>_<scheme>` (lower-case ISO 3166-1 alpha-2). All fields optional (`Option` / `Vec`); `Worker::validate()` requires at least one identifying field (name, national identifier, or non-empty `passport_books`). `MatchConfig` in `src/matcher.rs` is the authoritative full field list. Field groups:

**Identifying** — 42 `<cc>_<scheme>: Option<String>` per FR-12..FR-91 (catalogue [`agents/national-person-identifiers.md`](../agents/national-person-identifiers.md)); `passport_books: Vec<PassportBook>` (§8.6 / §6.4a); `given_name`, `family_name: Option<String>`.

**Demographic (scored, not identifying)** — `middle_name: Option<String>` (blend `0.05 × middle` per FR-49); `date_of_birth`, `death_date: Option<NaiveDate>`; `gender: Option<Gender>`; `blood_type: Option<BloodType>`; `multiple_birth: Option<u8>` (1-indexed birth order; twin disambiguation).

**Location** — `address: Option<Address>`; `previous_addresses: Vec<Address>` (best-of cartesian per §12.4); `birth_place`, `death_place: Option<Address>` (FHIR; city + country only). All share `Address` shape.

**Contact** — `phone: Option<String>` (E.164 + legacy fallback per FR-30); `mobile: Option<String>` (fallback when `phone` is `None`); `email: Option<String>` (canonical form per FR-35/FR-36). `local_id: Option<String>` carried but deliberately NOT scored (OQ-2 cross-org collision risk).

**Relationships (planned, not yet implemented — T-33)** — the design calls for `relationships: Vec<RelationshipRef>` (default empty; §8.6a): typed references to other workers by registry id, as a **supporting** signal (not identifying on its own): two records that reference the **same** related workers (same line-manager / report ids) would be more likely the same worker, scored by typed-set Jaccard (§12.2) and weighted `relationships_weight` (§13.1). **Not yet on `Worker`** — see the open task at §23 T-33; do not assume this field exists in the current API.

**Tags (planned, not yet implemented — T-34)** — the design calls for `tags: Vec<String>` (default empty): operator-applied free-text labels, normalised case-insensitively, as a **supporting** signal (not identifying on its own): two records carrying the **same** tags would be weakly more likely the same worker, scored by set Jaccard (§12.2) and weighted `tags_weight` (§13.1). **Not yet on `Worker`** — see the open task at §23 T-34; do not assume this field exists in the current API.

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

### 8.6a `RelationshipRef` / `RelationKind` (planned — T-33)

**Not yet implemented** — see §23 T-33. The design: `RelationshipRef { relation: RelationKind, worker_id: String }` would reference another worker in the consuming registry by **opaque id**; `worker_id` whitespace-trimmed and non-empty. `RelationKind` an enum mirroring the service `Worker`: `LineManagerOf`, `ReportsTo` (inverses — A `LineManagerOf` B ⇔ B `ReportsTo` A); extensible (e.g. `MentorOf`, `ColleagueOf` later). The matcher would **not** resolve the references (it has no registry) — it would only compare the two workers' relationship **sets** (§12.2).

### 8.5 `MatchBreakdown`

Each field `Option<f64>` (`None` = not scored; `Some(v)` ∈ `[0.0, 1.0]`). One field per scoring axis: 42 `<cc>_<scheme>_score` + `passport_book_score` + demographic + address + contact scores (`given_name_score`, `family_name_score`, `date_of_birth_score`, `gender_score`, `blood_type_score`, `multiple_birth_score`, `address_score`, `birth_place_score`, `death_date_score`, `death_place_score`, `phone_score`, `email_score`, `phonetic_name_score`). All `#[serde(default)]` so legacy payloads deserialise with `None` for later-added fields. `relationships_score` / `tags_score` are **not yet present** — planned alongside the T-33/T-34 fields above.

---

