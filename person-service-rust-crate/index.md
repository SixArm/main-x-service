# Person Service — Index

Centralised registry of person identities. Healthcare-aware (carries
the same NHS#/SSN/identity-document fields as the patient index), with
probabilistic + deterministic matching, real-time and batch
deduplication, HIPAA-grade audit, GDPR Article 15 export, and a
FHIR R5 Person surface.

This page is a **navigation aid with worked examples**. For canonical
behaviour, read [`spec.md`](spec.md).

## Documentation map

| File | Role |
|------|------|
| [`spec.md`](spec.md) | **Single source of truth.** What the system does, how it is built, NFRs, tasks (§13), open questions (§16). |
| [`README.md`](README.md) / [`CLAUDE.md`](CLAUDE.md) | User-facing intro — must stay consistent with the spec. |
| [`AGENTS.md`](AGENTS.md) | Agent-facing entry point — `AGENTS/*` directory + shared docs. |
| [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md) | The SDD discipline this crate practises. |
| [`AGENTS/models.md`](AGENTS/models.md) | Field-by-field domain model reference. |
| [`AGENTS/matching.md`](AGENTS/matching.md) | Match weights, components, deterministic rules, Soundex. |
| [`AGENTS/restful.md`](AGENTS/restful.md) | Endpoint catalogue + library API. |
| [`AGENTS/testing.md`](AGENTS/testing.md) | Unit / integration / benchmark layout. |
| [`agents/share/*`](../agents/share/) | Project-wide cross-crate references (architecture, web stack, compliance, …). |

## Quick start

```bash
# REST + gRPC API
cargo run --release

# Web UI (Loco / Tera / HTMX / Alpine / Lily)
cargo run --bin web                    # → http://0.0.0.0:5150
PORT=5180 cargo run --bin web

# Tests
cargo test --lib                       # unit (~100)
DATABASE_URL=… cargo test --tests      # integration (needs PostgreSQL)
cargo bench                            # Criterion (matching / search / validation)
```

## URL surface (REST)

| Method | Path | Notes |
|---|---|---|
| GET | `/api/health` | Liveness |
| POST | `/api/persons` | Create — `409` on detected duplicate |
| GET | `/api/persons/{id}` | Read |
| PUT | `/api/persons/{id}` | Update |
| DELETE | `/api/persons/{id}` | Soft delete |
| GET | `/api/persons/search` | Full-text / fuzzy / phonetic |
| POST | `/api/persons/match` | Score against candidates |
| POST | `/api/persons/check-duplicates` | Real-time dup check |
| POST | `/api/persons/merge` | Merge survivor + duplicate |
| POST | `/api/persons/deduplicate` | Batch dedup scan |
| GET | `/api/persons/{id}/masked` | Privacy view |
| GET | `/api/persons/{id}/export` | GDPR Art. 15 export |
| GET | `/api/persons/{id}/audit` | Per-record audit |
| GET | `/api/audit/recent` | System-wide recent audit |
| GET | `/api/audit/user` | Per-user audit |

FHIR R5 mounted under `/fhir/Person/*`. See
[`AGENTS/restful.md`](AGENTS/restful.md) for full parameters.

## Worked examples

### Create a person (with real-time duplicate detection)

```bash
curl -X POST http://localhost:8080/api/persons \
  -H 'content-type: application/json' \
  -d '{
    "name": { "family": "Smith", "given": ["John"] },
    "birth_date": "1980-01-15",
    "gender": "male",
    "tax_id": "123-45-6789",
    "documents": [{
      "document_type": "PASSPORT",
      "number": "X12345678",
      "issuing_country": "US"
    }],
    "emergency_contacts": [{
      "name": "Jane Smith",
      "relationship": "spouse",
      "telecom": [{ "system": "phone", "value": "+15550199" }],
      "is_primary": true
    }]
  }'
```

If the request creates a duplicate above the threshold, you get
`409 Conflict` with the candidate matches and per-component scores.

### Check for duplicates without creating

```bash
curl -X POST http://localhost:8080/api/persons/check-duplicates \
  -H 'content-type: application/json' \
  -d '{
    "name": { "family": "Smith", "given": ["John"] },
    "birth_date": "1980-01-15",
    "gender": "male"
  }'
```

### Search

```bash
curl "http://localhost:8080/api/persons/search?q=Smith\
&limit=10&offset=0&fuzzy=true&phonetic=true&mask_sensitive=true"
```

| Parameter | Meaning |
|---|---|
| `q` | Free-text against indexed fields |
| `limit` / `offset` | Pagination (limit ≤ 100) |
| `fuzzy` | Enable Tantivy fuzzy matching |
| `phonetic` | Enable Soundex-augmented matching |
| `mask_sensitive` | Apply per-field masking to results |

### Match against existing records

```bash
curl -X POST http://localhost:8080/api/persons/match \
  -H 'content-type: application/json' \
  -d '{
    "name": { "family": "Smyth", "given": ["Jon"] },
    "birth_date": "1980-01-15",
    "threshold": 0.7
  }'
```

Returns ranked candidates with `score`, `match_quality`
(Definite / Probable / Possible / Unlikely), and a per-component
`breakdown`.

### Merge two records

```bash
curl -X POST http://localhost:8080/api/persons/merge \
  -H 'content-type: application/json' \
  -d '{
    "main_person_id": "11111111-1111-1111-1111-111111111111",
    "duplicate_person_id": "22222222-2222-2222-2222-222222222222",
    "merge_reason": "Confirmed duplicate"
  }'
```

The duplicate is soft-deleted, its identifiers / addresses /
contacts / documents transfer to the survivor, its primary name
appends as a "former" alias, a `Replaces` link is written, a JSON
snapshot is captured, and a `Merged` event is published.

### Batch deduplication

```bash
curl -X POST http://localhost:8080/api/persons/deduplicate \
  -H 'content-type: application/json' \
  -d '{
    "threshold": 0.70,
    "auto_merge_threshold": 0.95,
    "max_candidates": 50
  }'
```

Returns `persons_scanned`, `duplicates_found`, `auto_merged`,
`queued_for_review`, and a list of `ReviewQueueItem`s for human
review.

### GDPR Article 15 export

```bash
curl "http://localhost:8080/api/persons/{id}/export"
```

Returns a JSON document with the full person record, identifiers,
addresses, contacts, documents, emergency contacts, consents, and
audit history.

### Masked person view

```bash
curl "http://localhost:8080/api/persons/{id}/masked"
```

Returns the person with per-field masking applied (last-four for
SSN/tax IDs, redacted phone, truncated email, etc.).

### FHIR R5 Person

```bash
# Create
curl -X POST http://localhost:8080/fhir/Person \
  -H 'content-type: application/fhir+json' \
  -d '{ "resourceType": "Person", "name": [{ "family": "Smith", "given": ["John"] }], "gender": "male", "birthDate": "1980-01-15" }'

# Read
curl -H 'accept: application/fhir+json' http://localhost:8080/fhir/Person/{id}

# Search
curl "http://localhost:8080/fhir/Person?family=Smith&birthdate=1980-01-15&_count=20"
```

## Library API examples

### Match two persons

```rust
use person_service::matching::{ProbabilisticMatcher, PersonMatcher};
use person_service::models::*;

let a = Person::new(HumanName::new("Smith", ["John"]), Gender::Male);
let b = Person::new(HumanName::new("Smyth", ["Jon"]),  Gender::Male);

let matcher = ProbabilisticMatcher::with_defaults();
let result  = matcher.match_persons(&a, &b);

println!("score={:.3} quality={:?}", result.score, result.quality);
for (k, v) in &result.breakdown {
    println!("  {k}: {v:.3}");
}
```

### Validate and normalise

```rust
use person_service::validation::{validate_person, normalize_phone, standardize_address};

let errs = validate_person(&person);
assert!(errs.is_empty(), "validation failed: {errs:?}");

let phone = normalize_phone("(555) 010-9999", "US");      // → "+15550109999"
let addr  = standardize_address(&Address {
    line1: Some("123 main st.".into()),
    city: Some("oakland".into()),
    state: Some("ca".into()),
    country: Some("us".into()),
    ..Default::default()
});                                                       // → title-case city, uppercase state, "Street"
```

### Privacy mask + GDPR export

```rust
use person_service::privacy::{mask_person, export_person_data};

let masked = mask_person(&person);          // last-4 for SSN, redacted phone, …
let export = export_person_data(&person);   // full JSON for portability
```

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | _required_ |
| `DATABASE_MIN_CONNECTIONS` / `DATABASE_MAX_CONNECTIONS` | Pool sizes | `2` / `10` |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `PORT` | Web UI port (`cargo run --bin web`) | `5150` |
| `SEARCH_INDEX_PATH` | Tantivy index directory | `./search_index` |
| `MATCHING_THRESHOLD` | Default match cutoff | `0.7` |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://localhost:4317` |
| `OTLP_SERVICE_NAME` | OTel `service.name` | `person-service` |
| `RUST_LOG` | `tracing-subscriber` filter | `info,person_service=info` |

## Project layout

```
src/
├── lib.rs              # Library root
├── api/                # REST, FHIR R5, gRPC API layers
├── models/             # Domain models (Person, Identifier, Document, …)
├── matching/           # Algorithms (name, DOB, gender, address, phonetic, scoring)
├── search/             # Tantivy index + query
├── db/                 # SeaORM models + repositories + audit
├── streaming/          # Event publishing (InMemory + Fluvio stub)
├── validation/         # Validation + normalisation
├── privacy/            # Masking + GDPR export + consent
├── config/             # Env loading + Config struct
├── observability/      # OpenTelemetry setup
├── web/                # Loco app + Tera views + Axum web router
├── bin/web.rs          # cargo run --bin web
└── error.rs

assets/views/           # Tera templates (HTMX + Alpine + Lily)
assets/static/          # lily.css, htmx.min.js, alpine.min.js
config/                 # development.yaml, test.yaml, production.yaml
migrations/             # SeaORM up.sql / down.sql pairs
tests/                  # Integration tests
benches/                # Criterion benchmarks
AGENTS/                 # Reference documentation
```

## Key types

| Type | Module | Description |
|---|---|---|
| `Person` | `models::person` | Core person identity record |
| `HumanName` | `models::person` | Structured name (family, given, prefix, suffix) |
| `Gender` | `models::mod` | Male / Female / Other / Unknown |
| `Identifier` | `models::identifier` | External IDs (MRN, SSN, DL, NPI, PPN, TAX, Other) |
| `IdentityDocument` | `models::document` | Passport / national ID / driver's licence / … |
| `EmergencyContact` | `models::emergency_contact` | Name + relationship + telecom + address |
| `Address` | `models::mod` | Physical address |
| `ContactPoint` | `models::mod` | Phone / email / fax / SMS / pager / URL |
| `Consent` | `models::consent` | GDPR consent record |
| `MergeRequest` / `MergeResponse` / `MergeRecord` | `models::merge` | Merge contract + persisted record |
| `ReviewQueueItem` | `models::review_queue` | Pending / Confirmed / Rejected / AutoMerged |
| `MatchResult` / `MatchScoreBreakdown` | `matching::mod` | Score + per-component detail |

## Key functions

| Function | Module | Description |
|---|---|---|
| `match_persons` | `matching::mod` | Match two persons with weighted scoring |
| `find_matches` | `matching::mod` | Match a person against a candidate list |
| `match_name` | `matching::algorithms` | Jaro-Winkler + Levenshtein name comparison |
| `match_dob` | `matching::algorithms` | Date proximity with tolerance |
| `match_address` | `matching::algorithms` | Weighted address comparison |
| `match_tax_id` | `matching::algorithms` | Exact tax-ID match (short-circuit) |
| `match_document` | `matching::algorithms` | Document type + number match |
| `soundex` | `matching::phonetic` | 4-character phonetic code |
| `validate_person` | `validation` | Required-field + format checks |
| `normalize_phone` | `validation` | E.164-like normalisation |
| `standardize_address` | `validation` | Title-case city, uppercase region, expand abbreviations |
| `mask_person` | `privacy` | Per-field masking |
| `export_person_data` | `privacy` | GDPR Article 15 export |
| `has_active_consent` | `privacy` | Consent check utility |

## Status & roadmap

- **Status** — see [`spec.md §14`](spec.md#14-implementation-status).
- **Tasks** — see [`spec.md §13`](spec.md#13-tasks) for the queue of
  in-flight work with acceptance criteria.
- **Roadmap** — see [`spec.md §15`](spec.md#15-roadmap).
- **Open questions** — see [`spec.md §16`](spec.md#16-open-questions).

## Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, soft delete, encryption-at-rest, access controls |
| GDPR Art. 15 | `/api/persons/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | `Person` resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |

## License

Dual-licensed: MIT OR Apache-2.0.
