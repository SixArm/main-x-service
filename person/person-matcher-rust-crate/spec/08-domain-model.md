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

**Planned, not yet implemented** (tracked as **T-33** / **T-34** in §23.2 — no `relationships`/`tags` field, no `RelationshipRef`/`RelationKind` type, and no `relationships_score`/`tags_score` breakdown field exists in `src/` today; do not treat the two bullets below as current behaviour):

- Relationships: `relationships: Vec<RelationshipRef>` (default empty; §8.6a) — typed references to other persons by registry id. A **supporting** signal, NOT an identifying field on its own: two records that reference the **same** related persons (same parent / child / sibling / guardian ids) are more likely the same person. Once T-33 lands: scored by typed-set overlap (§12.2), weighted `relationships_weight` (§13.1).
- Tags: `tags: Vec<String>` (default empty) — short free-text operator labels (e.g. `"vip"`, `"review"`). A **supporting** signal, NOT an identifying field on its own: two records carrying overlapping tag sets are weakly more likely the same person. Once T-34 lands: scored by plain set **Jaccard** over the case-insensitively normalised tags (§12.2), weighted `tags_weight` (§13.1).

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

### 8.6a `RelationshipRef` / `RelationKind` (planned — T-33, not yet implemented)

Design target: `RelationshipRef { relation: RelationKind, person_id: String }` references another person in the consuming registry by **opaque id**; `person_id` is whitespace-trimmed and non-empty. `RelationKind` is an enum mirroring the service `Person`: `ParentOf`, `ChildOf`, `SiblingOf`, `GuardianOf`, `WardOf`. The matcher would **not** resolve the references (it has no registry) — it would only compare the two persons' relationship **sets** (§12.2). Neither type exists in `src/models.rs` today; see §23.2 T-33.

### 8.5 `MatchBreakdown`

Every field is `Option<f64>`: `None` = not scored; `Some(v)` ∈ `[0.0, 1.0]`. One `<scheme>_score` per identifier (42) plus `passport_book_score`, plus demographic / FHIR: `given_name_score`, `family_name_score`, `date_of_birth_score`, `death_date_score`, `gender_score`, `blood_type_score`, `multiple_birth_score`, `birth_place_score`, `death_place_score`, `address_score`, `phone_score`, `email_score`, `phonetic_name_score`. (`relationships_score` / `tags_score` are **planned**, not yet fields on `MatchBreakdown` — T-33 / T-34, §23.2.)

---

