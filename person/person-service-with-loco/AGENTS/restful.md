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

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).

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

**Boot-time key fetch** — set `PERSON_PASETO_KEYS_URL` to the auth
service's `/.well-known/paseto-keys` URL to fetch the key set once at
boot instead of injecting it via env. Precedence: unset/blank URL ⇒
the `PERSON_PASETO_KEYS` env path exactly as before; URL set and the
fetch succeeds ⇒ the fetched key set **wins** over
`PERSON_PASETO_KEYS`; URL set but the fetch fails (network / HTTP /
parse) ⇒ a warning is logged and the env path is used — the service
**always boots**; auth-service downtime never prevents startup. The
fetch is one-shot (no refresh loop; restart to pick up a rotation).

**Blanket enforcement** — setting `PERSON_REQUIRE_AUTH` to a truthy
value (`1`/`true`/`yes`/`on`, case-insensitive; anything else,
including unset/blank/junk, means **off** — the default) makes every
route require a valid PASETO bearer token, except the public
allow-list: `/api/health`, loco's `/_health` / `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom`.
Unauthorised requests get `401`. The middleware
(`auth::require_auth_middleware`) is layered unconditionally on both
router surfaces (`create_router` and the loco `after_routes` hook);
the flag is read once at router construction, so changing it requires
a restart. Family contract: `agents/share/jwt-enforcement.md`.

#### Authorization (ABAC)

Inside the same guard (so only when `PERSON_REQUIRE_AUTH` is on), a
verified token is authorized by **attribute-based access control**
per `agents/share/authorization-attributes.md`: the request's action
is derived from the HTTP method plus the crate's destructive named
POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the policy over the token's `attrs` claim. Configure with
`PERSON_ABAC_POLICY` (inline JSON) or `PERSON_ABAC_POLICY_FILE`
(path); unset or unparsable ⇒ warn-log + the built-in default policy
(any authenticated subject reads; `access=write` writes;
`access=admin` adds DELETE/merge/deduplicate; `svc=true` does
everything). Read once at router construction — restart to change.
`401` = missing/bad credential; `403` = valid credential, policy
denied (the body names the deciding rule).

### Person CRUD

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/persons` | Create person (with real-time duplicate detection) |
| GET | `/api/persons` | **List** active persons, paginated (`?limit=&offset=&mask_sensitive=`) — database-backed via `PersonRepository::list_active`, deliberately **not** the Tantivy index (see `/persons/search` below and this endpoint's `CHANGELOG.md` entry for why) |
| GET | `/api/persons/{id}` | Get person by ID |
| PUT | `/api/persons/{id}` | Update person |
| DELETE | `/api/persons/{id}` | Soft delete person |

### Search

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/persons/search` | Search persons (full-text, fuzzy, phonetic) |

**Query Parameters:** `q` (query), `limit` (default 10, max 100), `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool)

**Not a list-all mechanism.** `q` has no "match everything" value: an
empty `q` parses to an empty Tantivy query (zero hits), and while the
query grammar's `q=*` token does parse to `AllQuery` in isolation, this
service's Tantivy index is a separate artefact from the database and can
legitimately drift from it (a dev index directory that outlives a
database reset; stale entries from records the database no longer has)
— a live investigation confirmed a small-page `q=*` request can come
back empty even though matching database rows exist. Use `GET
/api/persons` (above) to enumerate the collection; it reads the
database directly and cannot see this class of drift. See
`CHANGELOG.md` for the investigation.

### Matching & Deduplication

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/persons/match` | Match person against existing records |
| POST | `/api/persons/check-duplicates` | Check for duplicates without creating |
| POST | `/api/persons/merge` | Merge two person records |
| POST | `/api/persons/deduplicate` | Batch deduplication scan |
| GET | `/api/persons/review-queue` | Stored review queue (filter `status`, `limit`) |
| POST | `/api/persons/review-queue/{id}/decision` | Decide a pending review item (`confirmed` / `rejected`) |

### Cross-service links

Per [cross-service-linking.md](../../../agents/share/cross-service-linking.md)
§4.1; person is the reference originator of `same_identity` (person ↔
worker) and also originates `works_at` / `member_of` (person →
organization).

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/persons/{id}/links` | Create/upsert an outbound edge (idempotent) |
| GET | `/api/persons/{id}/links` | List this person's outbound edges |
| DELETE | `/api/persons/{id}/links/{link_id}` | Withdraw (soft-delete) an edge |
| GET | `/api/persons/links[?since=]` | Bulk pull of all active edges — the link-graph aggregator's reconciliation source, canonical `EdgeDetail` shape (`{ "edges": [...] }`) |

### Bulk import / export

Person is the family's **reference entity** for
[bulk-import-export.md](../../../agents/share/bulk-import-export.md);
mutating routes are async loco `worker` jobs. `import` is a declared
destructive POST.

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/persons/import` | Submit a bulk import (multipart upload; `202 {job_id}`; supports `dry_run`) — JSONL, CSV, or (feature `parquet`, export-only) rejects Parquet |
| GET | `/api/persons/import/{id}` | Import job status + counts + `errors_url` |
| POST | `/api/persons/export` | Submit a bulk export (JSON filter body; `202 {job_id}`) — JSONL, CSV, or `parquet` (feature-gated) |
| GET | `/api/persons/export/{id}` | Export job status + `download_url` |
| GET | `/api/persons/bulk-jobs` | List recent import/export jobs |

### Privacy

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/persons/{id}/export` | GDPR data export |
| GET | `/api/persons/{id}/masked` | Masked person view |
| POST | `/api/persons/{id}/erase` | GDPR erasure — destroys personal data, keeps the audit chain linkage (irreversible; destructive, `access=admin`) |

### Audit & compliance

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/persons/{id}/audit` | Person audit logs |
| GET | `/api/persons/{id}/audit/disclosures` | HIPAA §164.528 accounting of disclosures for this person |
| GET | `/api/audit/recent` | Recent audit activity |
| GET | `/api/audit/user` | User-specific audit logs |
| GET | `/api/audit/verify` | Recompute the audit hash chain, report any linkage/content break (HIPAA §164.312(c)) |
| GET | `/api/audit/checkpoint` | Take a checkpoint witness of the current audit chain tail |
| POST | `/api/audit/checkpoint/verify` | Check whether the chain still honours a recorded checkpoint (detects wholesale deletion) |
| GET | `/api/records/verify` | Recompute each person record's content hash, report mismatches (complements `/api/audit/verify`) |
| GET | `/api/compliance` | Service identification and build provenance |
| GET | `/api/compliance/sbom` | CycloneDX SBOM + SOUP register (not on the public allow-list — it names exact dependency versions) |

**Audit Query Parameters:** `limit` (default 50, max 500), `user_id` (for user endpoint)

## FHIR R5 Endpoints

Per [fhir.md](../../../agents/share/fhir.md) §3: **`Patient` is the
primary resource** (`high` fidelity, full CRUD + search); `/fhir/Person`
is a thin **read-only alias** for the demographic view (T-11, done
2026-07-07 — this reconciled the crate's earlier unmounted prototype,
which used the non-standard `resourceType: "Person"`).

| Method | Path | Description |
|--------|------|-------------|
| GET | `/fhir/metadata` | `CapabilityStatement` (fhirVersion 5.0.0) |
| POST | `/fhir/Patient` | Create FHIR Patient |
| GET | `/fhir/Patient` | Search FHIR Patients |
| GET | `/fhir/Patient/{id}` | Get FHIR Patient |
| PUT | `/fhir/Patient/{id}` | Update FHIR Patient |
| DELETE | `/fhir/Patient/{id}` | Delete FHIR Patient |
| GET | `/fhir/Person` | Search FHIR Persons (alias; GET only) |
| GET | `/fhir/Person/{id}` | Get FHIR Person (alias; GET only) |

**FHIR Search Parameters:** `name`, `family`, `given`, `identifier`, `birthdate`, `gender`, `_count`

Every non-2xx FHIR response is a `FhirOperationOutcome`; all responses
are `application/fhir+json`. `/fhir/*` sits behind the same blanket
auth+ABAC guard as `/api/*` (not on the public allow-list).

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
- `src/api/rest/links.rs` — Cross-service link handlers (`entity_links`)
- `src/api/fhir/mod.rs` — FHIR module, FhirPerson, conversions
- `src/api/fhir/handlers.rs` — FHIR endpoint handlers (`Patient` primary + `Person` alias)
- `src/api/fhir/resources.rs` — FHIR resource converters
- `src/api/fhir/bundle.rs` — FHIR bundle handling
- `src/api/fhir/search_parameters.rs` — FHIR search parameter support
- `src/api/grpc/mod.rs` — gRPC server (stub)
- `src/bulk/handlers.rs` — Bulk import/export REST handlers
- `src/bulk/worker.rs` — `BulkJobWorker` (loco `worker` job)
- `src/compliance/` — SBOM/SOUP identification, audit-chain verification, checkpoints, record-integrity verification
