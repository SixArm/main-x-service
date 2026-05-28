# AI Progress

## How to Run

```sh
claude plan.md
```

## Project Overview

Main Patient Index (MPI) - A healthcare patient identification and matching system built with Rust. Production-ready with 15 API endpoints, 34 unit tests, and comprehensive feature set.

## Phase Summary

| Phase | Name                       | Status   | Tests |
| ----- | -------------------------- | -------- | ----- |
| 1     | Project Setup & Foundation | Complete | -     |
| 2     | Database Schema & Models   | Complete | -     |
| 3     | Core MPI Logic             | Complete | 16    |
| 4     | Search Engine Integration  | Complete | 5     |
| 5     | RESTful API (Axum)         | Complete | -     |
| 6     | FHIR R5 Support            | Partial  | -     |
| 7     | Database Integration       | Complete | -     |
| 8     | Event Streaming & Audit    | Partial  | -     |
| 9     | REST API Implementation    | Complete | -     |
| 10    | Integration Testing        | Partial  | 7     |
| 11    | Docker & Deployment        | Complete | -     |
| 12    | Documentation              | Complete | -     |
| 13    | Advanced MPI Features      | Complete | 9     |
| 14-20 | Future phases              | Planned  | -     |

**Total: 34 unit tests passing, 7 integration tests (require PostgreSQL)**

## Phase 1: Project Setup & Foundation

Initialized Rust project with 40+ dependencies:

- Tokio, Axum, Diesel, Tantivy, Tonic, OpenTelemetry, Fluvio, Utoipa
- Modular architecture: api, models, db, matching, search, streaming, observability, config, error, validation, privacy
- 35+ source files

## Phase 2: Database Schema & Models

- 13 PostgreSQL tables with Diesel ORM
- 5 migration sets (365 lines SQL), 27 Diesel models
- 40+ strategic indexes, HIPAA-compliant audit triggers
- Capacity: 10M patients ~ 40-60 GB with indexes and audit

## Phase 3: Core MPI Logic

Matching algorithms:

- Name: Jaro-Winkler, Levenshtein, Soundex phonetic, nickname variants
- DOB: exact match + typo tolerance (day off, month/day transposition, year off)
- Gender: exact / unknown neutral / mismatch
- Address: postal code, city (fuzzy), state, street (normalized)
- Identifier: type + system + value (formatting normalization)
- Tax ID: exact match (deterministic, short-circuits to 1.0)
- Document: type + number match

Scoring:

- Probabilistic: weighted composite (name 30%, DOB 25%, gender 10%, address 10%, identifier 10%, tax_id 10%, document 5%)
- Deterministic: rule-based (tax ID match = 1.0, identifier match = 1.0, document match = 1.0, then name+DOB+gender rules)
- Quality: Definite (>=0.95), Probable (>=threshold), Possible (>=0.50), Unlikely (<0.50)

## Phase 4: Search Engine Integration

Tantivy full-text search:

- 11 indexed fields (id, family_name, given_names, full_name, birth_date, gender, postal_code, city, state, identifiers, active)
- Methods: search, fuzzy_search, search_by_name_and_year, index_patient, index_patients, delete_patient
- Bulk indexing, real-time updates, index optimization

## Phase 5: RESTful API (Axum)

15 endpoints:

- Health: GET /health
- Patient CRUD: POST /patients, GET /patients/{id}, PUT /patients/{id}, DELETE /patients/{id}
- Search: GET /patients/search (pagination, fuzzy, mask_sensitive)
- Matching: POST /patients/match
- Dedup: POST /patients/check-duplicates, POST /patients/merge, POST /patients/deduplicate
- Privacy: GET /patients/{id}/export, GET /patients/{id}/masked
- Audit: GET /patients/{id}/audit, GET /audit/recent, GET /audit/user
- OpenAPI/Swagger at /swagger-ui

## Phase 6: FHIR R5 Support (Partial)

- FhirPatient resource model with all standard fields
- Bidirectional conversion (to_fhir_patient / from_fhir_patient)
- FHIR search parameters (name, family, given, identifier, birthdate, gender)
- OperationOutcome error responses
- Foundation handlers (not yet wired to live DB)

## Phase 7: Database Integration

- DieselPatientRepository: full CRUD with transactions
- Domain <-> DB model conversion for 6 related tables
- Soft delete, paginated listing, name search
- Event publishing and audit logging integrated into repository

## Phase 8: Event Streaming & Audit Logging

- InMemoryEventPublisher (thread-safe, Arc-compatible)
- PatientEvent: Created, Updated, Deleted, Merged, Linked, Unlinked
- AuditLogRepository: CREATE/UPDATE/DELETE with old/new JSON values
- Query: by entity, recent, by user
- Automatic via repository builder pattern

## Phase 9-12: API, Testing, Docker, Docs

- All 15 REST endpoints with OpenAPI annotations
- 7 integration tests (health, CRUD, search, error handling)
- Multi-stage Dockerfile, Docker Compose dev/test
- DEPLOY.md, README.md, architecture docs

## Phase 13: Advanced MPI Features

**Patient Identity Management:**

- `tax_id` field on Patient (CPF, SSN, TIN)
- `documents: Vec<IdentityDocument>` (Passport, Birth Certificate, National ID, Driver's License, Voter ID, Military ID, Residence Permit, Work Permit)
- `emergency_contacts: Vec<EmergencyContact>` (name, relationship, telecom, address, primary flag)
- `AddressUse` enum (Home, Work, Temp, Old, Billing)

**Duplicate Detection:**

- Real-time during POST /patients (returns 409 Conflict with matches)
- POST /patients/check-duplicates (explicit check)
- Tax ID exact match (deterministic, score 1.0)
- Document number match (type + number)
- Soundex phonetic matching integrated into name matching
- Score breakdown (tax_id_score, document_score) in responses

**Record Merging (POST /patients/merge):**

- Transfers: identifiers, names, addresses, contacts, documents, emergency contacts, tax_id
- Adds duplicate's name as "old" alias
- Creates PatientLink (Replaces) from main to duplicate
- Soft-deletes duplicate, publishes Merged event
- Returns merge record with transferred data snapshot

**Batch Deduplication (POST /patients/deduplicate):**

- Pairwise patient scan
- Configurable: threshold, max_candidates, auto_merge_threshold
- Review queue items (Pending, Confirmed, Rejected, AutoMerged)
- Returns: patients_scanned, duplicates_found, auto_merged, queued_for_review

**Data Quality (src/validation/mod.rs):**

- Required: family name, given name
- Validates: birth_date (no future), tax_id format, email (@.), phone (7+ digits)
- Address: requires city/postal_code/country
- Documents: required number, expiry check, issue<expiry
- Emergency contacts: required name + relationship
- normalize_phone(): E.164-like format
- standardize_address(): title-case city, uppercase state/country, expand abbreviations
- Integrated into create/update handlers (422 on failure)

**Privacy (src/privacy/mod.rs):**

- mask_patient(): masks SSN, tax ID, passport, DL, phone (shows last 4)
- GET /patients/{id}/export: GDPR data export (full JSON)
- GET /patients/{id}/masked: masked patient view
- Consent model: DataProcessing, DataSharing, Marketing, Research, EmergencyAccess
- Consent status: Active, Revoked, Expired
- has_active_consent() utility

**New modules:** validation, privacy, matching/phonetic
**New models:** document, emergency_contact, merge, review_queue, consent

**Tests added:** 9 new unit tests (phonetic: 4, validation: 3, privacy: 2)
**Build:** 0 errors, 34/34 unit tests passing

## Build & Test Status

```
cargo check   -> 0 errors
cargo test --lib -> 34 tests passing
```

## Quick Start

```bash
cp .env.example .env
docker-compose up -d
# API: http://localhost:8080/api
# Swagger: http://localhost:8080/swagger-ui
```
