## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete worker records.
- Multiple professional identifiers per worker.
- Credential documents with expiry tracking.
- Multiple addresses, telecom contacts, emergency contacts.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](../AGENTS/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | Weighted sum 0.00–1.00 | Fuzzy input |
| Deterministic | Rule-based; short-circuits on identifier (NPI, DEA, employee #), tax-ID, or document exact match | Hard guarantees |

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

#### Interoperability with `worker-matcher`

The service embeds the sibling `worker-matcher` crate (declared in
`Cargo.toml`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The matcher crate is the **canonical reference
algorithm** — it carries 40+ national-identifier parsers (UK NHS,
FR NIR, US SSN, BR CPF, IN Aadhaar, …), passport-book matching,
blood-type signals, nickname tables, and three tuned config presets
(`strict` / `default` / `lenient`) that the in-service matcher does
not duplicate.

Bridge: [`src/matching/adapter.rs`](../src/matching/adapter.rs) exposes
`to_matcher_worker(&service::Worker) -> worker_matcher::Worker`. The
projection lifts the service's FHIR-shaped record into the matcher's
flat builder shape using the same routing rules as the person bridge
(name flattening, telecom sampling by `ContactPointSystem`, address
field renames, identifier routing by `system` URI with type-based
fallbacks, passport documents → `passport_books`). Service-only
fields (`id`, `active`, `worker_type`, `deceased_datetime`,
`managing_organization`, `links`, `created_at`, …) are dropped.

The matcher's `uk_nhs_number` slot is the per-worker equivalent of
the person matcher's `uk_nhs_number` (both crates settled on the
shorter method name once published to crates.io). The service's
worker-specific
`IdentifierType::ODS` (NHS Organisation Data Service code) has no
country-slot counterpart and falls through unmapped; surface it on
the matcher side only if a future matcher release adds an ODS
parser. See [`AGENTS/matching.md`](../AGENTS/matching.md) for the
in-service algorithm and the matcher crate's
[`spec.md §12`](../../worker-matcher-rust-crate/spec/index.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across 11 indexed fields (name, identifiers — including NPI
/ DEA — DOB year, addresses). Full-text + fuzzy + phonetic, boolean
syntax, pagination (`offset` + `limit`), optional sensitive-field
masking. Index stays synchronised with database writes.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/workers`.
- Explicit `POST /api/workers/check-duplicates`.
- Batch `POST /api/workers/deduplicate` with configurable thresholds.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers (credentials!), names, addresses,
  contacts, documents, tax-ID, emergency contacts; appends the
  duplicate's primary name as a "former" alias on the survivor;
  adds a `Replaces` link; soft-deletes the duplicate; records a JSON
  snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required-field enforcement (family + given name), future-date guard
on birth date, tax-ID format, email regex, phone digit count, address
completeness, document number required + expiry guard,
emergency-contact name + relationship required. Phone normalised
E.164-like; addresses standardised. Failed validation → `422`.

### 6.6 Privacy

Per-field masking, GDPR Article 15 export at
`GET /api/workers/{id}/export`, masked view at
`GET /api/workers/{id}/masked`, consent model with type + status +
dates, `has_active_consent()` utility. Sensitive fields specific to
workforce data (SSN, tax ID, DEA, home address) are masked by default
in the masked view. See
[`agents/share/privacy.md`](../../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-worker, recent
system-wide, per-user.

### 6.8 FHIR R5

Bidirectional Practitioner resource conversion under
`/fhir/Practitioner`. Search parameters: `name`, `family`, `given`,
`identifier`, `birthdate`, `gender`, `_count`.

