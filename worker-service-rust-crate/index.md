# Worker Service — Index

Centralised registry of workforce and professional identities:
operators, contractors, drivers, site staff, field engineers.
Carries credential / licence / professional-identifier fields (NPI,
DEA, board licence, employee number) alongside the structured
demographics. Probabilistic + deterministic matching, real-time and
batch deduplication, HIPAA-grade audit, GDPR Article 15 export, and a
FHIR R5 Practitioner surface.

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
| [`agents/share/*`](../agents/share/) | Project-wide cross-crate references. |

## Quick start

```bash
# REST + gRPC API
cargo run --release

# Tests
cargo test --lib                       # unit (~99)
DATABASE_URL=… cargo test --tests      # integration (needs PostgreSQL)
cargo bench                            # Criterion (matching / search / validation)
```

## URL surface (REST)

| Method | Path | Notes |
|---|---|---|
| GET | `/api/health` | Liveness |
| POST | `/api/workers` | Create — `409` on detected duplicate |
| GET | `/api/workers/{id}` | Read |
| PUT | `/api/workers/{id}` | Update |
| DELETE | `/api/workers/{id}` | Soft delete |
| GET | `/api/workers/search` | Full-text / fuzzy / phonetic |
| POST | `/api/workers/match` | Score against candidates |
| POST | `/api/workers/check-duplicates` | Real-time dup check |
| POST | `/api/workers/merge` | Merge survivor + duplicate |
| POST | `/api/workers/deduplicate` | Batch dedup scan |
| GET | `/api/workers/{id}/masked` | Privacy view |
| GET | `/api/workers/{id}/export` | GDPR Art. 15 export |
| GET | `/api/workers/{id}/audit` | Per-record audit |
| GET | `/api/audit/recent` | System-wide recent audit |
| GET | `/api/audit/user` | Per-user audit |

FHIR R5 Practitioner mounted under `/fhir/Practitioner/*`. See
[`AGENTS/restful.md`](AGENTS/restful.md) for full parameters.

## Worked examples

### Create a worker (professional with NPI)

```bash
curl -X POST http://localhost:8080/api/workers \
  -H 'content-type: application/json' \
  -d '{
    "name": { "family": "Patel", "given": ["Nisha"], "prefix": ["Dr."] },
    "birth_date": "1985-07-04",
    "gender": "female",
    "identifiers": [
      { "identifier_type": "NPI", "system": "http://hl7.org/fhir/sid/us-npi", "value": "1234567893" }
    ],
    "documents": [{
      "document_type": "PROFESSIONAL_LICENSE",
      "number": "CA-MD-12345",
      "issuing_country": "US",
      "issuing_authority": "Medical Board of California",
      "issue_date": "2012-06-15",
      "expiry_date": "2026-06-14",
      "verified": true
    }],
    "managing_organization": "11111111-1111-1111-1111-111111111111"
  }'
```

If the request creates a duplicate above the threshold, you get
`409 Conflict` with the candidate matches and per-component scores
(NPI exact match short-circuits to `score = 1.00`).

### Check for duplicates without creating

```bash
curl -X POST http://localhost:8080/api/workers/check-duplicates \
  -H 'content-type: application/json' \
  -d '{
    "name": { "family": "Patel", "given": ["Nisha"] },
    "identifiers": [
      { "identifier_type": "NPI", "system": "http://hl7.org/fhir/sid/us-npi", "value": "1234567893" }
    ]
  }'
```

### Search

```bash
curl "http://localhost:8080/api/workers/search?q=Patel\
&limit=10&offset=0&fuzzy=true&phonetic=true&mask_sensitive=true"
```

### Match against existing records

```bash
curl -X POST http://localhost:8080/api/workers/match \
  -H 'content-type: application/json' \
  -d '{
    "name": { "family": "Patell", "given": ["Nisha"] },
    "birth_date": "1985-07-04",
    "threshold": 0.7
  }'
```

Returns ranked candidates with `score`, `match_quality`, and a
per-component `breakdown`.

### Merge

```bash
curl -X POST http://localhost:8080/api/workers/merge \
  -H 'content-type: application/json' \
  -d '{
    "main_worker_id": "11111111-1111-1111-1111-111111111111",
    "duplicate_worker_id": "22222222-2222-2222-2222-222222222222",
    "merge_reason": "Confirmed duplicate — same NPI, different employee numbers"
  }'
```

Credentials transfer from the duplicate to the survivor, the
duplicate's primary name appends as a "former" alias, and a
`Replaces` link is written.

### Batch deduplication

```bash
curl -X POST http://localhost:8080/api/workers/deduplicate \
  -H 'content-type: application/json' \
  -d '{
    "threshold": 0.70,
    "auto_merge_threshold": 0.95,
    "max_candidates": 50
  }'
```

### GDPR Article 15 export

```bash
curl "http://localhost:8080/api/workers/{id}/export"
```

### Masked worker view

```bash
curl "http://localhost:8080/api/workers/{id}/masked"
```

Workforce-specific sensitive fields (SSN, tax ID, DEA, home address)
are masked by default in the masked view.

### FHIR R5 Practitioner

```bash
# Create
curl -X POST http://localhost:8080/fhir/Practitioner \
  -H 'content-type: application/fhir+json' \
  -d '{
    "resourceType": "Practitioner",
    "identifier": [{ "system": "http://hl7.org/fhir/sid/us-npi", "value": "1234567893" }],
    "name": [{ "family": "Patel", "given": ["Nisha"], "prefix": ["Dr."] }],
    "gender": "female",
    "birthDate": "1985-07-04"
  }'

# Read
curl -H 'accept: application/fhir+json' http://localhost:8080/fhir/Practitioner/{id}

# Search
curl "http://localhost:8080/fhir/Practitioner?family=Patel&identifier=http://hl7.org/fhir/sid/us-npi|1234567893"
```

## Library API examples

### Match two workers

```rust
use worker_service::matching::{ProbabilisticMatcher, WorkerMatcher};
use worker_service::models::*;

let a = Worker::new(HumanName::new("Patel", ["Nisha"]),  Gender::Female);
let b = Worker::new(HumanName::new("Patell", ["Nisha"]), Gender::Female);

let matcher = ProbabilisticMatcher::with_defaults();
let result  = matcher.match_workers(&a, &b);

println!("score={:.3} quality={:?}", result.score, result.quality);
for (k, v) in &result.breakdown {
    println!("  {k}: {v:.3}");
}
```

### Match via the canonical `worker-matcher` bridge

Use the sibling `worker-matcher` crate as the reference algorithm. The
service re-exports it as `matcher_lib`, and `adapter::to_matcher_worker`
projects the service's domain model into the matcher's input shape
(including national-identifier routing by FHIR `system` URI, address
field renaming, and identifier-scheme dispatch).

```rust,no_run
use worker_service::matching::adapter::to_matcher_worker;
use worker_service::matching::matcher_lib::{Confidence, MatchConfig, MatchingEngine};
use worker_service::models::*;

let dob = chrono::NaiveDate::from_ymd_opt(1970, 4, 1).unwrap();
let name_a = HumanName {
    use_type: None,
    family: "Patel".into(),
    given: vec!["Asha".into()],
    prefix: vec![],
    suffix: vec![],
};
let name_b = HumanName {
    use_type: None,
    family: "Patel".into(),
    given: vec!["Ashaa".into()], // typo
    prefix: vec![],
    suffix: vec![],
};

let mut a = Worker::new(name_a, Gender::Female);
a.birth_date = Some(dob);
let mut b = Worker::new(name_b, Gender::Female);
b.birth_date = Some(dob);

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));

assert!(result.is_match, "near-duplicate should classify as match");
assert_eq!(result.confidence, Confidence::High);
println!("score   = {:.3}", result.score);
println!("conf    = {:?}", result.confidence);
// `result.breakdown` carries per-field Option<f64> for an auditable trail.
```

End-to-end pinning lives in
[`tests/duplicate_detection.rs`](tests/duplicate_detection.rs); the
adapter source is [`src/matching/adapter.rs`](src/matching/adapter.rs).


### Validate and normalise

```rust
use worker_service::validation::{validate_worker, normalize_phone, standardize_address};

let errs = validate_worker(&worker);
assert!(errs.is_empty(), "validation failed: {errs:?}");

let phone = normalize_phone("(555) 010-9999", "US");  // → "+15550109999"
```

### Privacy mask + GDPR export

```rust
use worker_service::privacy::{mask_worker, export_worker_data, has_active_consent};
use worker_service::models::consent::ConsentType;

let masked = mask_worker(&worker);
let export = export_worker_data(&worker);

let ok = has_active_consent(&worker_consents, ConsentType::DataSharing);
```

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | _required_ |
| `DATABASE_MIN_CONNECTIONS` / `DATABASE_MAX_CONNECTIONS` | Pool sizes | `2` / `10` |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `SEARCH_INDEX_PATH` | Tantivy index directory | `./search_index` |
| `MATCHING_THRESHOLD` | Default match cutoff | `0.7` |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://localhost:4317` |
| `OTLP_SERVICE_NAME` | OTel `service.name` | `worker-service` |
| `RUST_LOG` | `tracing-subscriber` filter | `info,worker_service=info` |

## Project layout

```
src/
├── lib.rs              # Library root
├── api/                # REST, FHIR R5 Practitioner, gRPC API layers
├── models/             # Worker, HumanName, Identifier, Document, EmergencyContact, …
├── matching/           # algorithms (name, DOB, gender, address, identifier, tax-ID, document, phonetic)
├── search/             # Tantivy index + query
├── db/                 # SeaORM models + repositories + audit
├── streaming/          # Event publishing (InMemory + Fluvio stub)
├── validation/         # Validation + normalisation
├── privacy/            # Masking + GDPR export + consent
├── config/             # Env loading + Config struct
├── observability/      # OpenTelemetry setup
└── error.rs

config/                 # development.yaml, test.yaml, production.yaml
migrations/             # SeaORM up.sql / down.sql pairs
tests/                  # Integration tests
benches/                # Criterion benchmarks
AGENTS/                 # Reference documentation
```

## Key types

| Type | Module | Description |
|---|---|---|
| `Worker` | `models::worker` | Core worker identity record |
| `HumanName` | `models::worker` | Structured name |
| `Gender` | `models::mod` | Male / Female / Other / Unknown |
| `Identifier` | `models::identifier` | External IDs (MRN, SSN, DL, NPI, PPN, TAX, Other) |
| `IdentityDocument` | `models::document` | Credentials / licences / passports |
| `EmergencyContact` | `models::emergency_contact` | Name + relationship + telecom + address |
| `Address` / `ContactPoint` | `models::mod` | Shared shapes |
| `Consent` | `models::consent` | GDPR consent record |
| `MergeRequest` / `MergeResponse` / `MergeRecord` | `models::merge` | Merge contract + persisted record |
| `ReviewQueueItem` | `models::review_queue` | Pending / Confirmed / Rejected / AutoMerged |
| `MatchResult` / `MatchScoreBreakdown` | `matching::mod` | Score + per-component detail |

## Key functions

| Function | Module | Description |
|---|---|---|
| `match_workers` | `matching::mod` | Match two workers with weighted scoring |
| `find_matches` | `matching::mod` | Match a worker against a candidate list |
| `match_name` | `matching::algorithms` | Jaro-Winkler + Levenshtein name comparison |
| `match_dob` | `matching::algorithms` | Date proximity with tolerance |
| `match_address` | `matching::algorithms` | Weighted address comparison |
| `match_tax_id` | `matching::algorithms` | Exact tax-ID match (short-circuit) |
| `match_document` | `matching::algorithms` | Document type + number match |
| `soundex` | `matching::phonetic` | 4-char phonetic |
| `validate_worker` | `validation` | Required + format checks |
| `normalize_phone` | `validation` | E.164-like normalisation |
| `standardize_address` | `validation` | Title-case city, uppercase region, expand abbreviations |
| `mask_worker` | `privacy` | Per-field masking |
| `export_worker_data` | `privacy` | GDPR Article 15 export |
| `has_active_consent` | `privacy` | Consent check utility |

## Status & roadmap

- **Status** — see [`spec.md §14`](spec.md#14-implementation-status).
- **Tasks** — see [`spec.md §13`](spec.md#13-tasks), including the
  credential-expiry warning workflow (T-7) and role / assignment
  history timeline (T-8).
- **Roadmap** — see [`spec.md §15`](spec.md#15-roadmap).
- **Open questions** — see [`spec.md §16`](spec.md#16-open-questions).

## Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, soft delete, encryption-at-rest, access controls |
| GDPR Art. 15 | `/api/workers/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Practitioner resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |

## License

Dual-licensed: MIT OR Apache-2.0.
