# RESTful API Reference

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

Core domain models are in `src/models/`:

- `Patient` — Central patient identity record with name, identifiers, addresses, contacts, documents, emergency contacts
- `HumanName` — Name with family, given, prefix, suffix, use type
- `Identifier` — External identifier (MRN, SSN, DL, NPI, PPN, TAX)
- `IdentityDocument` — Identity document (passport, birth certificate, etc.)
- `EmergencyContact` — Emergency contact with name, relationship, telecom
- `Organization` — Healthcare organization
- `MergeRequest` / `MergeResponse` — Patient merge operations
- `ReviewQueueItem` — Deduplication review queue
- `Consent` — Patient consent management

### Matching

Matching API is in `src/matching/`:

- `PatientMatcher` trait — `match_patients()`, `find_matches()`, `is_match()`
- `ProbabilisticMatcher` — Weighted fuzzy matching with configurable thresholds
- `DeterministicMatcher` — Rule-based exact matching
- `MatchResult` — Score + breakdown per component

### Validation

Validation API is in `src/validation/`:

- `validate_patient(&Patient) -> Vec<ValidationError>` — Comprehensive validation
- `normalize_phone(&str, &str) -> String` — E.164-like normalization
- `standardize_address(&Address) -> Address` — Address standardization

### Privacy

Privacy API is in `src/privacy/`:

- `mask_patient(&Patient) -> Patient` — Mask sensitive fields
- `export_patient_data(&Patient) -> Value` — GDPR data export
- `has_active_consent(&[Consent], ConsentType) -> bool` — Consent checking

## RESTful API Endpoints

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/health` | Health check |

### Patient CRUD

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/patients` | Create patient (with real-time duplicate detection) |
| GET | `/api/patients/{id}` | Get patient by ID |
| PUT | `/api/patients/{id}` | Update patient |
| DELETE | `/api/patients/{id}` | Soft delete patient |

### Search

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/patients/search` | Search patients (full-text, fuzzy, phonetic) |

**Query Parameters:** `q` (query), `limit` (default 10, max 100), `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool)

### Matching & Deduplication

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/patients/match` | Match patient against existing records |
| POST | `/api/patients/check-duplicates` | Check for duplicates without creating |
| POST | `/api/patients/merge` | Merge two patient records |
| POST | `/api/patients/deduplicate` | Batch deduplication scan |

### Privacy

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/patients/{id}/export` | GDPR data export |
| GET | `/api/patients/{id}/masked` | Masked patient view |

### Audit

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/patients/{id}/audit` | Patient audit logs |
| GET | `/api/audit/recent` | Recent audit activity |
| GET | `/api/audit/user` | User-specific audit logs |

**Audit Query Parameters:** `limit` (default 50, max 500), `user_id` (for user endpoint)

## FHIR R5 Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/fhir/Patient/{id}` | Get FHIR Patient |
| POST | `/fhir/Patient` | Create FHIR Patient |
| PUT | `/fhir/Patient/{id}` | Update FHIR Patient |
| DELETE | `/fhir/Patient/{id}` | Delete FHIR Patient |
| GET | `/fhir/Patient` | Search FHIR Patients |

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
- `src/api/fhir/mod.rs` — FHIR module, FhirPatient, conversions
- `src/api/fhir/handlers.rs` — FHIR endpoint handlers
- `src/api/fhir/resources.rs` — FHIR resource converters
- `src/api/fhir/bundle.rs` — FHIR bundle handling
- `src/api/fhir/search_parameters.rs` — FHIR search parameter support
- `src/api/grpc/mod.rs` — gRPC server (stub)
