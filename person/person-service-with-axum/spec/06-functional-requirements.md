## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete person records.
- Multiple identifiers (typed, system-qualified).
- Identity documents with expiry tracking.
- Multiple addresses, telecom, emergency contacts.
- Automatic event publish on every CRUD. See
  [`agents/share/auditability.md`](../../../agents/share/auditability.md).

### 6.2 Matching

Algorithm reference: [`agents/matching.md`](../agents/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | Weighted sum 0.00–1.00 | Fuzzy input |
| Deterministic | Rule-based; short-circuits on tax-ID, identifier, or document exact match | Hard guarantees |

Default component weights:

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.30 | Jaro-Winkler + Levenshtein + Soundex bonus |
| Birth date | 0.25 | Date proximity |
| Gender | 0.10 | Exact / unknown handling |
| Address | 0.10 | Weighted postal / city / state / street |
| Identifier | 0.10 | Type + system + value match |
| Tax ID | 0.10 | Exact match (deterministic short-circuit to 1.0) |
| Document | 0.05 | Type + number match |

Match quality (configurable thresholds):

| Quality | Score |
|---|---|
| Definite | ≥ 0.95 |
| Probable | ≥ 0.85 |
| Possible | ≥ 0.50 |
| Unlikely | < 0.50 |

#### Interoperability with `person-matcher`

The service embeds the sibling `person-matcher` crate (path dependency
declared in `Cargo.toml`) and re-exports it from
`src/matching/mod.rs` as `matcher_lib`. The matcher crate is the
**canonical reference algorithm** — it carries 40+ national-identifier
parsers (UK NHS, FR NIR, US SSN, BR CPF, IN Aadhaar, …), passport-book
matching, blood-type signals, nickname tables, and three tuned config
presets (`strict` / `default` / `lenient`) that the in-service matcher
does not duplicate.

Bridge: [`src/matching/adapter.rs`](../src/matching/adapter.rs) exposes
`to_matcher_person(&service::Person) -> person_matcher::Person`. The
projection lifts the service's FHIR-shaped record (named `HumanName`,
`Vec<Identifier>` with FHIR system URIs, `Vec<Address>`,
`Vec<ContactPoint>`, `Vec<IdentityDocument>`) into the matcher's flat
builder shape:

- `name.family` → `family_name`; first/second `name.given` → `given_name` / `middle_name`
- `birth_date` → `date_of_birth`; `gender` → `gender`
- First `addresses[]` → `address` (rest → `previous_addresses`); `state` renamed `county`, `postal_code` → `postcode`
- First telecom of each `ContactPointSystem` → `phone` / `mobile` / `email`
- `identifiers[]` routed to country-specific slots by `system` URI (e.g. `https://fhir.nhs.uk/Id/nhs-number` → `uk_nhs_number`); falls back to `IdentifierType` when no URI hint
- `tax_id` defaults to `us_ssn` unless a typed identifier overrides
- `IdentityDocument` of type `Passport` → `passport_books` (one per passport)

Registry-only fields (`id`, `active`, `deceased_datetime`,
`managing_organization`, `links`, `created_at`, …) are dropped — they
have no matcher counterpart. The projection is **lossy by design** so
callers can use the reference algorithm without rewriting their
domain model. See [`agents/matching.md`](../agents/matching.md) for the
in-service algorithm and the matcher crate's
[`spec.md §12`](../../person-matcher-rust-crate/spec/index.md) for the
canonical algorithm.

### 6.3 Search

Powered by Tantivy across 11 indexed fields. Full-text + fuzzy +
phonetic, boolean syntax, pagination (`offset` + `limit`), optional
sensitive-field masking. Index stays synchronised with database writes;
bulk re-index supported.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/persons` when matches exceed
  the configured threshold.
- Explicit `POST /api/persons/check-duplicates`.
- Batch `POST /api/persons/deduplicate` with configurable `threshold`,
  `max_candidates`, `auto_merge_threshold`.
- Review queue with `Pending` / `Confirmed` / `Rejected` / `AutoMerged`.
- **Match evidence** ([match-evidence.md](../../../agents/share/match-evidence.md),
  §10 step 4; T-41). Every scored pair — the `match` endpoint,
  `check-duplicates`, the create-time `409` body, the batch scan and its
  stored review items, bulk-import review rows — carries the matcher's
  `evidence` (weight in bits, fields compared, contradictions, strength,
  per-term listing) beside the service's own renormalised `score`, which
  is unchanged and still ranks and filters. Evidence is computed through
  `matching::adapter::to_matcher_person` and the canonical engine's
  `match_persons_with_frequencies` (default `MatchConfig`: `is_match` at
  `0.85`, proof bar `30` bits). It holds component names and bits, never
  a field value.
- **Auto-merge only on `proven`.** The batch scan flags a pair
  `AutoMerged` only when `score >= auto_merge_threshold` **and**
  `evidence.strength == proven`; a `consistent` pair (name + birth date +
  gender only: twins) or a `contradicted` one (two different national
  identifiers under one scheme) is queued `Pending` whatever its score.
  The service performs no automatic merge itself; the flag is the only
  auto-merge surface, and the gate is `matching::evidence::auto_merge_permitted`.
  The create-time `409` is unchanged.
- **Value frequencies** (match-evidence.md §7). The batch scan counts
  given name, family name and birth date (the matcher's
  `FREQUENCY_COMPONENTS`, keyed by its `frequency_key`) over the records
  it loaded. The interactive paths use a table cached in app state,
  built by a keyset scan of active persons (`db::frequency`, 1 000 per
  page, capped at 200 000: above the cap the table is a sample of the
  first 200 000 by id) and rebuilt on a spawned task when older than ten
  minutes; a request never waits on it, and before the first build it
  uses no frequencies. Below `MIN_FREQUENCY_POPULATION` (1 000) records
  no table is supplied, because a share from a small population is
  noise.
- Merge transfers identifiers, names, addresses, contacts, documents,
  tax ID, emergency contacts; appends the duplicate's primary name as
  a "former" alias on the survivor; adds `Replaces` link; soft-deletes
  the duplicate; records a JSON snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required `family` + first `given` name; future-date guard on birth date;
tax-ID format; email regex; phone digit count; address completeness;
document number required + expiry guard; emergency-contact name +
relationship required. Phone normalised E.164-like; addresses
standardised. Failed validation → `422`.

### 6.6 Privacy

Per-field masking, GDPR Article 15 export at
`GET /api/persons/{id}/export`, masked view at
`GET /api/persons/{id}/masked`, consent model with type + status +
dates, `has_active_consent()` utility. Masking keeps the last four
**characters** of each redacted value visible and replaces preceding
alphanumeric characters with `*` (non-alphanumeric separators pass
through); it counts Unicode scalar values, not bytes, so multibyte
input (accented names, non-Latin identifiers) is masked without
panicking. See
[`agents/share/privacy.md`](../../../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-person, recent
system-wide, per-user. See
[`agents/share/auditability.md`](../../../agents/share/auditability.md).

### 6.8 FHIR R5

Bidirectional Person resource conversion under `/fhir/Person`. Search
parameters: `name`, `family`, `given`, `identifier`, `birthdate`,
`gender`, `_count`. OperationOutcome on error.

