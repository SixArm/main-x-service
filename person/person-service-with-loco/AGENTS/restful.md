# RESTful API Reference

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

Core domain models are in `src/models/`:

- `Person` — Central person identity record with name, identifiers, addresses, contacts, documents, emergency contacts
- `HumanName` — Name with family, given, prefix, suffix, use type
- `Identifier` — External identifier (MRN, SSN, DL, NPI, PPN, TAX)
- `IdentityDocument` — Identity document (passport, birth certificate, etc.)
- `EmergencyContact` — Emergency contact with name, relationship, telecom
- `Organization` — Healthcare organization
- `MergeRequest` / `MergeResponse` — Person merge operations
- `ReviewQueueItem` — Deduplication review queue
- `Consent` — Person consent management

### Matching

Matching API is in `src/matching/`:

- `PersonMatcher` trait — `match_persons()`, `find_matches()`, `is_match()`
- `ProbabilisticMatcher` — Weighted fuzzy matching with configurable thresholds
- `DeterministicMatcher` — Rule-based exact matching
- `MatchResult` — Score + breakdown per component


### Adapter to the canonical `person-matcher` crate

The service embeds the sibling `person-matcher` crate and re-exports it
from `src/matching/mod.rs` as `matcher_lib`. Pair it with
`adapter::to_matcher_person` to score two service records through the
canonical algorithm:

```rust
use person_service::matching::adapter::to_matcher_person;
use person_service::matching::matcher_lib::{MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_persons(
    &to_matcher_person(&a),
    &to_matcher_person(&b),
);
// result.score: f64 in [0.0, 1.0]
// result.is_match: bool
// result.confidence: High | Medium | Low
// result.breakdown: per-field Option<f64>
```

Field-routing rules are documented inline in
[`src/matching/adapter.rs`](../src/matching/adapter.rs) and pinned by
[`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs).

### Validation

Validation API is in `src/validation/`:

- `validate_person(&Person) -> Vec<ValidationError>` — Comprehensive validation
- `normalize_phone(&str, &str) -> String` — E.164-like normalization
- `standardize_address(&Address) -> Address` — Address standardization

### Privacy

Privacy API is in `src/privacy/`:

- `mask_person(&Person) -> Person` — Mask sensitive fields
- `export_person_data(&Person) -> Value` — GDPR data export
- `has_active_consent(&[Consent], ConsentType) -> bool` — Consent checking


### Prometheus metrics

| Method | Path             | Description                                                                  |
| ------ | ---------------- | ---------------------------------------------------------------------------- |
| GET    | `/metrics.prom`  | Prometheus text-exposition format (`text/plain; version=0.0.4`) for scraping |

Configure your scraper with `metrics_path: /metrics.prom`. The metric
inventory (entity-CRUD counters, HTTP request counter, latency
histograms) is in [`src/metrics.rs`](../src/metrics.rs). The handler
is [`api::rest::handlers::metrics_prom`](../src/api/rest/handlers.rs).

## RESTful API Endpoints

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/health` | Health check |

### Auth

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/whoami` | Echo the verified bearer-token claims (`401` without a valid token) |

Bearer tokens are PASETO `v4.public` (Ed25519) minted by the central
authentication-service and verified **offline** against its published
key set (`/.well-known/paseto-keys`) via the `authentication-verifier`
crate — no shared secret, no introspection call. Configure with
`PERSON_PASETO_KEYS` (key-set JSON), `PERSON_TOKEN_ISSUER`, and
`PERSON_TOKEN_AUDIENCE`. Handlers opt in by taking an `AuthUser`
argument (`src/api/rest/auth.rs`).

### Person CRUD

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/persons` | Create person (with real-time duplicate detection) |
| GET | `/api/persons/{id}` | Get person by ID |
| PUT | `/api/persons/{id}` | Update person |
| DELETE | `/api/persons/{id}` | Soft delete person |

### Search

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/persons/search` | Search persons (full-text, fuzzy, phonetic) |

**Query Parameters:** `q` (query), `limit` (default 10, max 100), `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool)

### Matching & Deduplication

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/persons/match` | Match person against existing records |
| POST | `/api/persons/check-duplicates` | Check for duplicates without creating |
| POST | `/api/persons/merge` | Merge two person records |
| POST | `/api/persons/deduplicate` | Batch deduplication scan |

### Privacy

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/persons/{id}/export` | GDPR data export |
| GET | `/api/persons/{id}/masked` | Masked person view |

### Audit

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/persons/{id}/audit` | Person audit logs |
| GET | `/api/audit/recent` | Recent audit activity |
| GET | `/api/audit/user` | User-specific audit logs |

**Audit Query Parameters:** `limit` (default 50, max 500), `user_id` (for user endpoint)

## FHIR R5 Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/fhir/Person/{id}` | Get FHIR Person |
| POST | `/fhir/Person` | Create FHIR Person |
| PUT | `/fhir/Person/{id}` | Update FHIR Person |
| DELETE | `/fhir/Person/{id}` | Delete FHIR Person |
| GET | `/fhir/Person` | Search FHIR Persons |

**FHIR Search Parameters:** `name`, `family`, `given`, `identifier`, `birthdate`, `gender`, `_count`

## Response Format

All REST endpoints return:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

Error responses:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable message",
    "details": { ... }
  }
}
```

## HTTP Status Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 201 | Created |
| 204 | Deleted (no content) |
| 400 | Bad request / invalid FHIR |
| 404 | Not found |
| 409 | Conflict (duplicate detected on create) |
| 422 | Validation error |
| 500 | Internal server error |

## Source Files

- `src/api/mod.rs` — ApiResponse, ApiError
- `src/api/rest/mod.rs` — REST API setup, router configuration
- `src/api/rest/handlers.rs` — All REST handler implementations
- `src/api/rest/routes.rs` — Route organization
- `src/api/rest/state.rs` — AppState (shared application state)
- `src/api/fhir/mod.rs` — FHIR module, FhirPerson, conversions
- `src/api/fhir/handlers.rs` — FHIR endpoint handlers
- `src/api/fhir/resources.rs` — FHIR resource converters
- `src/api/fhir/bundle.rs` — FHIR bundle handling
- `src/api/fhir/search_parameters.rs` — FHIR search parameter support
- `src/api/grpc/mod.rs` — gRPC server (stub)
