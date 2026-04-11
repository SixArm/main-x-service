# RESTful API Reference

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

Core domain models are in `src/models/`:

- `Event` — Central event identity record with name, identifiers, addresses, contacts, documents, emergency contacts
- `HumanName` — Name with family, given, prefix, suffix, use type
- `Identifier` — External identifier (MRN, SSN, DL, NPI, PPN, TAX)
- `IdentityDocument` — Identity document (passport, birth certificate, etc.)
- `EmergencyContact` — Emergency contact with name, relationship, telecom
- `Organization` — Healthcare organization
- `MergeRequest` / `MergeResponse` — Event merge operations
- `ReviewQueueItem` — Deduplication review queue
- `Consent` — Event consent management

### Matching

Matching API is in `src/matching/`:

- `EventMatcher` trait — `match_events()`, `find_matches()`, `is_match()`
- `ProbabilisticMatcher` — Weighted fuzzy matching with configurable thresholds
- `DeterministicMatcher` — Rule-based exact matching
- `MatchResult` — Score + breakdown per component

### Validation

Validation API is in `src/validation/`:

- `validate_event(&Event) -> Vec<ValidationError>` — Comprehensive validation
- `normalize_phone(&str, &str) -> String` — E.164-like normalization
- `standardize_address(&Address) -> Address` — Address standardization

### Privacy

Privacy API is in `src/privacy/`:

- `mask_event(&Event) -> Event` — Mask sensitive fields
- `export_event_data(&Event) -> Value` — GDPR data export
- `has_active_consent(&[Consent], ConsentType) -> bool` — Consent checking

## RESTful API Endpoints

### Health

| Method | Path          | Description  |
| ------ | ------------- | ------------ |
| GET    | `/api/health` | Health check |

### Event CRUD

| Method | Path               | Description                                       |
| ------ | ------------------ | ------------------------------------------------- |
| POST   | `/api/events`      | Create event (with real-time duplicate detection) |
| GET    | `/api/events/{id}` | Get event by ID                                   |
| PUT    | `/api/events/{id}` | Update event                                      |
| DELETE | `/api/events/{id}` | Soft delete event                                 |

### Search

| Method | Path                 | Description                                |
| ------ | -------------------- | ------------------------------------------ |
| GET    | `/api/events/search` | Search events (full-text, fuzzy, phonetic) |

**Query Parameters:** `q` (query), `limit` (default 10, max 100), `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool)

### Matching & Deduplication

| Method | Path                           | Description                           |
| ------ | ------------------------------ | ------------------------------------- |
| POST   | `/api/events/match`            | Match event against existing records  |
| POST   | `/api/events/check-duplicates` | Check for duplicates without creating |
| POST   | `/api/events/merge`            | Merge two event records               |
| POST   | `/api/events/deduplicate`      | Batch deduplication scan              |

### Privacy

| Method | Path                      | Description       |
| ------ | ------------------------- | ----------------- |
| GET    | `/api/events/{id}/export` | GDPR data export  |
| GET    | `/api/events/{id}/masked` | Masked event view |

### Audit

| Method | Path                     | Description              |
| ------ | ------------------------ | ------------------------ |
| GET    | `/api/events/{id}/audit` | Event audit logs         |
| GET    | `/api/audit/recent`      | Recent audit activity    |
| GET    | `/api/audit/user`        | User-specific audit logs |

**Audit Query Parameters:** `limit` (default 50, max 500), `user_id` (for user endpoint)

## FHIR R5 Endpoints

| Method | Path               | Description        |
| ------ | ------------------ | ------------------ |
| GET    | `/fhir/Event/{id}` | Get FHIR Event     |
| POST   | `/fhir/Event`      | Create FHIR Event  |
| PUT    | `/fhir/Event/{id}` | Update FHIR Event  |
| DELETE | `/fhir/Event/{id}` | Delete FHIR Event  |
| GET    | `/fhir/Event`      | Search FHIR Events |

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

| Code | Meaning                                 |
| ---- | --------------------------------------- |
| 200  | Success                                 |
| 201  | Created                                 |
| 204  | Deleted (no content)                    |
| 400  | Bad request / invalid FHIR              |
| 404  | Not found                               |
| 409  | Conflict (duplicate detected on create) |
| 422  | Validation error                        |
| 500  | Internal server error                   |

## Source Files

- `src/api/mod.rs` — ApiResponse, ApiError
- `src/api/rest/mod.rs` — REST API setup, router configuration
- `src/api/rest/handlers.rs` — All REST handler implementations
- `src/api/rest/routes.rs` — Route organization
- `src/api/rest/state.rs` — AppState (shared application state)
- `src/api/fhir/mod.rs` — FHIR module, FhirEvent, conversions
- `src/api/fhir/handlers.rs` — FHIR endpoint handlers
- `src/api/fhir/resources.rs` — FHIR resource converters
- `src/api/fhir/bundle.rs` — FHIR bundle handling
- `src/api/fhir/search_parameters.rs` — FHIR search parameter support
- `src/api/grpc/mod.rs` — gRPC server (stub)
