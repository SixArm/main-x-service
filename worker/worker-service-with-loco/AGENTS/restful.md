# RESTful API Reference

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

Core domain models are in `src/models/`:

- `Worker` — Central worker identity record with name, identifiers, addresses, contacts, documents, emergency contacts
- `HumanName` — Name with family, given, prefix, suffix, use type
- `Identifier` — External identifier (MRN, SSN, DL, NPI, PPN, TAX)
- `IdentityDocument` — Identity document (passport, birth certificate, etc.)
- `EmergencyContact` — Emergency contact with name, relationship, telecom
- `Organization` — Healthcare organization
- `MergeRequest` / `MergeResponse` — Worker merge operations
- `ReviewQueueItem` — Deduplication review queue
- `Consent` — Worker consent management

### Matching

Matching API is in `src/matching/`:

- `WorkerMatcher` trait — `match_workers()`, `find_matches()`, `is_match()`
- `ProbabilisticMatcher` — Weighted fuzzy matching with configurable thresholds
- `DeterministicMatcher` — Rule-based exact matching
- `MatchResult` — Score + breakdown per component


### Adapter to the canonical `worker-matcher` crate

The service embeds the sibling `worker-matcher` crate and re-exports it
from `src/matching/mod.rs` as `matcher_lib`. Pair it with
`adapter::to_matcher_worker` to score two service records through the
canonical algorithm:

```rust
use worker_service::matching::adapter::to_matcher_worker;
use worker_service::matching::matcher_lib::{MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_workers(
    &to_matcher_worker(&a),
    &to_matcher_worker(&b),
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

- `validate_worker(&Worker) -> Vec<ValidationError>` — Comprehensive validation
- `normalize_phone(&str, &str) -> String` — E.164-like normalization
- `standardize_address(&Address) -> Address` — Address standardization

### Privacy

Privacy API is in `src/privacy/`:

- `mask_worker(&Worker) -> Worker` — Mask sensitive fields
- `export_worker_data(&Worker) -> Value` — GDPR data export
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

| Method | Path             | Description  |
| ------ | ---------------- | ------------ |
| GET    | `/api/health` | Health check |

### Auth

| Method | Path             | Description                                                          |
| ------ | ---------------- | -------------------------------------------------------------------- |
| GET    | `/api/whoami` | Echo the verified bearer-token claims (`401` without a valid token) |

Bearer tokens are PASETO `v4.public` (Ed25519) minted by the central
authentication-service and verified **offline** against its published
key set (`/.well-known/paseto-keys`) via the `authentication-verifier`
crate — no shared secret, no introspection call. Configure with
`WORKER_PASETO_KEYS` (key-set JSON), `WORKER_TOKEN_ISSUER`, and
`WORKER_TOKEN_AUDIENCE`. Handlers opt in by taking an `AuthUser`
argument (`src/api/rest/auth.rs`).

**Boot-time key fetch** — set `WORKER_PASETO_KEYS_URL` to the auth
service's `/.well-known/paseto-keys` URL to fetch the key set once at
boot instead of injecting it via env. Precedence: unset/blank URL ⇒
the `WORKER_PASETO_KEYS` env path exactly as before; URL set and the
fetch succeeds ⇒ the fetched key set **wins** over
`WORKER_PASETO_KEYS`; URL set but the fetch fails (network / HTTP /
parse) ⇒ a warning is logged and the env path is used — the service
**always boots**; auth-service downtime never prevents startup. The
fetch is one-shot (no refresh loop; restart to pick up a rotation).

**Blanket enforcement (default off).** Setting `WORKER_REQUIRE_AUTH`
to a truthy value (`1`/`true`/`yes`/`on`, case-insensitive; anything
else — including unset, blank, `0`, or junk — means off) requires a
valid PASETO bearer token on **every** route of both router surfaces
(the standalone Axum `create_router` and the loco router) and returns
`401` otherwise. The flag and the verifier are captured **once, at
router construction** — changing the flag requires a process restart.
The public allow-list (`PUBLIC_PATHS` / `PUBLIC_PATH_PREFIXES` in
`src/api/rest/auth.rs`) stays token-free even when enforcement is on:

- `/_health`, `/_ping` (loco health probes)
- `/api/health` (this crate's health endpoint)
- `/api-docs/openapi.json` (OpenAPI document)
- `/swagger-ui*` (Swagger UI + assets)
- `/metrics.prom` (Prometheus scrape)

The `/fhir` surface is deliberately **not** on the allow-list — it
serves worker PII. Family-wide contract:
`agents/share/jwt-enforcement.md`.

#### Authorization (ABAC)

Inside the same guard (so only when `WORKER_REQUIRE_AUTH` is on), a
verified token is authorized by **attribute-based access control**
per `agents/share/authorization-attributes.md`: the request's action
is derived from the HTTP method plus the crate's destructive named
POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the policy over the token's `attrs` claim. Configure with
`WORKER_ABAC_POLICY` (inline JSON) or `WORKER_ABAC_POLICY_FILE`
(path); unset or unparsable ⇒ warn-log + the built-in default policy
(any authenticated subject reads; `access=write` writes;
`access=admin` adds DELETE/merge/deduplicate; `svc=true` does
everything). Read once at router construction — restart to change.
`401` = missing/bad credential; `403` = valid credential, policy
denied (the body names the deciding rule).

### Worker CRUD

| Method | Path                   | Description                                        |
| ------ | ---------------------- | -------------------------------------------------- |
| POST   | `/api/workers`      | Create worker (with real-time duplicate detection) |
| GET    | `/api/workers/{id}` | Get worker by ID                                   |
| PUT    | `/api/workers/{id}` | Update worker                                      |
| DELETE | `/api/workers/{id}` | Soft delete worker                                 |

### Search

| Method | Path                     | Description                                 |
| ------ | ------------------------ | ------------------------------------------- |
| GET    | `/api/workers/search` | Search workers (full-text, fuzzy, phonetic) |

**Query Parameters:** `q` (query), `limit` (default 10, max 100), `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool)

### Matching & Deduplication

| Method | Path                               | Description                           |
| ------ | ---------------------------------- | ------------------------------------- |
| POST   | `/api/workers/match`            | Match worker against existing records |
| POST   | `/api/workers/check-duplicates` | Check for duplicates without creating |
| POST   | `/api/workers/merge`            | Merge two worker records              |
| POST   | `/api/workers/deduplicate`      | Batch deduplication scan              |
| GET | `/api/workers/review-queue` | Stored review queue (filter `status`, `limit`) |
| POST | `/api/workers/review-queue/{id}/decision` | Decide a pending review item (`confirmed` / `rejected`) |

### Privacy

| Method | Path                          | Description        |
| ------ | ----------------------------- | ------------------ |
| GET    | `/api/workers/{id}/export` | GDPR data export   |
| GET    | `/api/workers/{id}/masked` | Masked worker view |

### Audit

| Method | Path                         | Description              |
| ------ | ---------------------------- | ------------------------ |
| GET    | `/api/workers/{id}/audit` | Worker audit logs        |
| GET    | `/api/audit/recent`       | Recent audit activity    |
| GET    | `/api/audit/user`         | User-specific audit logs |

**Audit Query Parameters:** `limit` (default 50, max 500), `user_id` (for user endpoint)

## FHIR R5 Endpoints

> **Status:** the handlers below are implemented in
> `src/api/fhir/handlers.rs` (wire `resourceType: "Practitioner"`, the
> standard FHIR R5 type per `agents/share/fhir.md` §3) and are
> **mounted** on the loco router — `App::routes` registers
> `workers_routes()`, `fhir_routes()`, and `metrics_routes()`, and the
> standalone `create_router` mirrors the same `/fhir/Practitioner`
> surface for the integration-test harness. Pinned by
> `tests/api_integration_test.rs::test_fhir_worker_route_is_mounted`
> (un-gated, asserts the route is reachable) and
> `::test_fhir_worker_not_found_returns_operation_outcome` (DB-gated,
> asserts a FHIR `OperationOutcome`). Closes spec §13 T-9 / entity T-1.

| Method | Path                      | Description               |
| ------ | ------------------------- | ------------------------- |
| GET    | `/fhir/Practitioner/{id}` | Get FHIR Practitioner     |
| POST   | `/fhir/Practitioner`      | Create FHIR Practitioner  |
| PUT    | `/fhir/Practitioner/{id}` | Update FHIR Practitioner  |
| DELETE | `/fhir/Practitioner/{id}` | Delete FHIR Practitioner  |
| GET    | `/fhir/Practitioner`      | Search FHIR Practitioners |
| GET    | `/fhir/metadata`          | CapabilityStatement       |

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
- `src/api/fhir/mod.rs` — FHIR module, FhirWorker, conversions
- `src/api/fhir/handlers.rs` — FHIR endpoint handlers
- `src/api/fhir/resources.rs` — FHIR resource converters
- `src/api/fhir/bundle.rs` — FHIR bundle handling
- `src/api/fhir/search_parameters.rs` — FHIR search parameter support
- `src/api/grpc/mod.rs` — gRPC server (stub)
