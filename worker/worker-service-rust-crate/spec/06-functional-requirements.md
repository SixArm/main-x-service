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

Identifier `system`-URI routing recognises a distinctive token per
national scheme and fills the matching matcher slot. The covered set
is UK NHS / US SSN / BR CPF / FR NIR / ES TSI / IN Aadhaar / JP My
Number / MX CURP / SE Personnummer / DE KVNR / NL BSN / NZ NHI / AU+IE
IHI (length-disambiguated), plus PL PESEL / PL NIP / RO CNP / UK NINO /
UK CHI / UK H&C / IT Codice Fiscale / ES DNI / PT NIF / FI HETU / DK
CPR / HR OIB / NO FNR / BG EGN / SI EMŠO / CN RRN / ZA ID / BE NN.
Tokens are chosen not to collide (e.g. `nino` never overlaps `nir`;
short abbreviations such as `chi`/`hc` require a longer qualifier).
Every covered slot carries its own weight + breakdown score +
deterministic short-circuit in the matcher (its spec §12), so a shared
value drives a match rather than silently falling through. An
unrecognised URI falls back to the `IdentifierType` enum
(`TAX`/`SSN` → US SSN; `ODS`/`MRN`/`DL`/`NPI`/`Other` unmapped).
Routing is pinned by the adapter's own unit tests and by
[`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs)
(`shared_pesel_drives_match_via_pl_pesel_slot`,
`shared_nhs_number_drives_match_via_uk_nhs_number_slot`).

The matcher's `uk_nhs_number` slot is the per-worker equivalent of
the person matcher's `uk_nhs_number` (both crates settled on the
shorter method name once published to crates.io).

**ODS routing decision (entity task T-7, 2026-06-13):** the service's
worker-specific `IdentifierType::ODS` (NHS Organisation Data Service
code) has **no suitable matcher slot and is deliberately left
unmapped**. Grounding:

- Every matcher identifier slot is a *person-level* national scheme
  (42 schemes: `uk_nhs_number`, `us_ssn`, `fr_nir`, …). An ODS code
  identifies an *organisation or site* (RC1/RC2 record classes), not
  the worker; every worker at the same practice shares it.
- Routing it into any person slot would make the deterministic
  exact-match short-circuit declare colleagues to be the same person
  — a catastrophic false positive.
- The matcher's `local_id` slot is deliberately never scored (matcher
  spec resolved OQ-2: organisation-issued values collide), so routing
  ODS there would be a silent no-op.

The fall-through is pinned by two bridge tests in
[`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs):
`ods_organisation_code_falls_through_unmapped` (matching continues on
remaining fields) and
`shared_ods_code_does_not_make_different_workers_match` (a shared ODS
code never short-circuits two different workers to a match). Revisit
only if the matcher crate ever adds an organisation-affiliation
signal (its spec §23). See
[`AGENTS/matching.md`](../AGENTS/matching.md) for the in-service
algorithm and the matcher crate's
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
[`agents/share/privacy.md`](../../../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-worker, recent
system-wide, per-user.

### 6.8 FHIR R5

Bidirectional `Worker` resource conversion under `/fhir/Worker`
(handlers in `src/api/fhir/handlers.rs`; the wire `resourceType` is
`"Worker"`). Search parameters: `name`, `family`, `given`,
`identifier`, `birthdate`, `gender`, `_count`.

**Status:** the FHIR handlers are implemented and **mounted** on the
loco router — `App::routes` registers `fhir_routes()` alongside the
REST and metrics route groups, and `create_router` mirrors the same
`/fhir/Worker` surface for the integration-test harness. The mount is
pinned by `tests/api_integration_test.rs::test_fhir_worker_route_is_mounted`
(un-gated, asserts the route is reachable via a `400` from the
`Path<Uuid>` extractor) and
`::test_fhir_worker_not_found_returns_operation_outcome` (DB-gated,
asserts a FHIR `OperationOutcome`). Tracked in §13 T-9 (done
2026-06-13).

