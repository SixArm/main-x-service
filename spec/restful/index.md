# RESTful API conventions

Monorepo-wide specification for the REST API surface shared by the
**Main X Index** service crates. This is the single reference for how
the services expose HTTP: route shape, status codes, OpenAPI, auth,
CORS, the extra API layers, pagination, masking, and metrics.

It is grounded in the actual code. Two generations of service coexist
and they differ in concrete ways (response envelope, extra API layers),
so each section calls out **loco services** (organization, care-pathway,
case) versus the **older Axum services** (person, worker, place, thing,
event, course) where they diverge.

See also the brief shared note
[`../../agents/share/restful.md`](../../agents/share/restful.md) and the
per-entity API references such as
[`../../person/person-service-rust-crate/AGENTS/restful.md`](../../person/person-service-rust-crate/AGENTS/restful.md)
and
[`../../organization/organization-service-rust-crate/AGENTS.md`](../../organization/organization-service-rust-crate/AGENTS.md).

---

## 1. API conventions

### 1.1 Transport and shape

- **JSON** request and response bodies (`Content-Type: application/json`).
- **Resource-oriented** routes under `/api/<plural>` where `<plural>`
  is the entity (`/api/organizations`, `/api/persons`, `/api/cases`, …).
  The event front-end calls under `/api/v1/`; the rest are unversioned.
- A record is addressed by its **public id** (`{pid}`) — a UUID string.
  In the loco services this is the `pid` column, distinct from the
  internal row id; an unparseable pid is treated as not-found (or `400`
  on the audit sub-path — see §2).

### 1.2 Standard endpoint surface

Every service offers the same conceptual surface, mounted under the
entity prefix. The table uses `organizations` as the example; substitute
the entity plural.

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/organizations` | Create (body: the entity payload) |
| `GET` | `/api/organizations` | List active records (capped) |
| `GET` | `/api/organizations/{pid}` | Fetch one stored record |
| `PUT` | `/api/organizations/{pid}` | Replace the payload (full replace, not patch) |
| `DELETE` | `/api/organizations/{pid}` | Soft-delete (row retained) |
| `GET` | `/api/organizations/search?q=` | Name / title search |
| `POST` | `/api/organizations/match` | Rank a `{query, candidates}` set (stateless, no persistence) |
| `POST` | `/api/organizations/check-duplicates` | Score a query against stored records |
| `POST` | `/api/organizations/merge` | Merge a duplicate into a survivor |
| `GET` | `/api/organizations/merges/recent` | Recent merge-history records |
| `GET` | `/api/organizations/audit/recent` | Recent audit entries (all records) |
| `GET` | `/api/organizations/{pid}/audit` | Audit trail for one record |
| `GET` | `/api/organizations/events/recent` | Recent in-memory event-stream entries |
| `GET` | `/api/organizations/whoami` | Verified bearer-token claims (`401` without one) |

The older Axum services additionally expose `POST /api/<plural>/deduplicate`
(a batch index scan) and the privacy reads
`GET /api/<plural>/{pid}/masked` and `GET /api/<plural>/{pid}/export`
(see §7). They also mount a health route at `/api/health`.

Literal sub-paths (`/search`, `/merge`, `/whoami`, `/merges/recent`, …)
are registered **before** the `/{pid}` captures so the dynamic segment
does not shadow them. This ordering is a hard convention — see the
`routes()` function in
[`organizations.rs`](../../organization/organization-service-rust-crate/src/controllers/organizations.rs).

### 1.3 Response envelope: raw loco JSON vs `ApiResponse`

This is the main wire difference between the two generations.

- **Loco services** (organization, care-pathway, case) return **raw
  loco JSON**: the handler's value is the body. Create/update/list/search
  return a lightweight `{pid, name}` reference (`OrgRef`); `GET /{pid}`
  returns the full stored payload; errors surface as loco's own JSON
  error shape with the right status code.
- **Older Axum services** (person, worker, place, thing, event, course)
  wrap every response in an `ApiResponse` envelope:

  ```json
  { "success": true, "data": { /* ... */ }, "error": null }
  ```

  Errors invert it:

  ```json
  { "success": false, "data": null,
    "error": { "code": "ERROR_CODE", "message": "…", "details": { } } }
  ```

  Defined in `src/api/mod.rs` (`ApiResponse`, `ApiError`).

New work follows the loco convention (raw JSON). Clients that talk to
both generations must handle both shapes.

---

## 2. HTTP status codes

The services use a deliberately small, conventional set. The table is
repo-accurate; behaviour noted per generation where it differs.

| Code | Meaning | When |
|---|---|---|
| `200` | OK | Successful read / create / update / match / merge (loco create returns `200`, not `201`) |
| `201` | Created | Create success in the older Axum services |
| `204` | No Content | Delete success in the older Axum services (loco delete returns `200` with empty JSON) |
| `400` | Bad Request | Malformed request: a blank/missing `q` on `/search` (an empty search would match everything); an invalid-UUID `pid` on `/{pid}/audit`; malformed FHIR in the older services |
| `404` | Not Found | Unknown `pid` on get/update/delete/merge; in the loco services a soft-deleted or malformed pid also maps to not-found (`http_err` remaps SeaORM `EntityNotFound`, which loco would otherwise surface as `500`) |
| `409` | Conflict | Duplicate detected during create (real-time duplicate detection); body carries the candidate matches |
| `422` | Unprocessable Entity | Validation failure — e.g. a blank `name`/`title`, or `main_pid == duplicate_pid` on merge. **All validation problems are reported together**, not just the first. `422` is reserved for semantic validation; `400` is for malformed requests (see [`../validation/index.md`](../validation/index.md)) |
| `429` | Too Many Requests | Rate limit on the authentication-service magic-link endpoints (see [`../authentication/index.md`](../authentication/index.md)) |
| `500` | Internal Server Error | Unexpected server / database error |
| `401` | Unauthorized | Missing or invalid bearer token on a protected route — see §4 |

Note the loco-vs-older split on create/delete: loco returns `200`
throughout (create `200`+`OrgRef`, delete `200`+empty JSON); the older
services use `201` for create and `204` for delete.

---

## 3. OpenAPI / Swagger

Both generations ship an OpenAPI 3 document and a Swagger UI, but the
loco services author the spec **by hand** rather than deriving it:

- The loco service DTO **is** the dependency-light matcher type (e.g.
  `organization_matcher::Organization`), which deliberately does not
  depend on `utoipa`. So the OpenAPI document is a hand-written
  `serde_json::Value` in
  [`src/openapi.rs`](../../organization/organization-service-rust-crate/src/openapi.rs)
  (`spec()`), which also keeps the doc accurate to the snake_case wire
  format.
- Served by
  [`src/controllers/docs.rs`](../../organization/organization-service-rust-crate/src/controllers/docs.rs):
  - `GET /api-docs/openapi.json` — the raw OpenAPI 3.0.3 JSON.
  - `GET /swagger-ui` — a static HTML page that loads Swagger UI from
    the `swagger-ui-dist@5` **CDN** (keeps the crate dependency-light)
    and points it at the spec endpoint.
- Both doc paths stay **public** even under blanket auth enforcement
  (see `is_public_path` in §4).
- The hand-written spec is **pinned by unit tests** in `openapi.rs`:
  they assert it is well-formed (`openapi == "3.0.3"`), that the
  load-bearing endpoints/schemas are present (create, merge,
  `MergeRequest`), and that `whoami` carries the `bearer` security
  requirement — so edits to the `json!` literal cannot silently drop
  them.

The older Axum services document the surface with **Utoipa**-derived
OpenAPI and serve Swagger UI at `/swagger-ui`.

---

## 4. Authentication on the API

Bearer **RS256 JWT**, issued by the central
[authentication-service](../../authentication/authentication-service-rust-crate)
and verified **offline** by each peer via the
[authentication-verifier](../../authentication/authentication-verifier-rust-crate)
crate against the service's JWKS — no shared secret, no introspection
hop. See [`../authentication/index.md`](../authentication/index.md) for
the token flow. The per-service extractor lives in
[`src/auth.rs`](../../organization/organization-service-rust-crate/src/auth.rs).

Two extractors, two postures:

| Extractor | Posture | Use |
|---|---|---|
| `MaybeAuthUser` | Never rejects | Yields `Some(claims)` when a valid token is present, else `None`. Handlers use `.actor()` (the caller `sub`) to stamp the audit / merge `actor` without requiring auth |
| `AuthUser` | Rejects with `401` | A missing/invalid token rejects before the handler runs. `GET /api/<plural>/whoami` takes `AuthUser`, so reaching its body proves end-to-end verification; it echoes the verified claims |

**Blanket enforcement.** A process-wide flag (`<ENTITY>_REQUIRE_AUTH`,
e.g. `ORGANIZATION_REQUIRE_AUTH`) turns on an Axum middleware layer
(`enforce`, wired in `src/app.rs`) that requires a valid bearer token on
**every route except** the public health/ping and OpenAPI/Swagger paths
(`is_public_path`: `/_health`, `/_ping`, `/api-docs/openapi.json`,
`/swagger-ui*`). It is **off by default** (`1`/`true`/`yes`/`on`,
case-insensitive, are the only truthy values); unset/blank/junk leaves
today's opt-in-per-handler behaviour. Activation is an operations
decision once the SSO token flow is live.

JWKS source is environment-driven: `<ENTITY>_JWKS` (the JWKS JSON;
absent ⇒ empty key set ⇒ every token rejected, but the service still
boots), `<ENTITY>_JWT_ISSUER` (default `authentication-service`),
`<ENTITY>_JWT_AUDIENCE` (default `main-x-service`). Boot-time JWKS fetch
over HTTP is a deferred follow-up; today the JWKS is injected via env.

---

## 5. CORS, error handling, content negotiation

- **CORS** is configurable for browser front-ends (each entity has a
  sibling SvelteKit SPA that calls the REST API cross-origin). The
  loco services configure CORS via loco middleware/config; the older
  services via `tower-http` `CorsLayer`.
- **Content negotiation** is minimal and deliberate: requests and
  responses are JSON (`application/json`); the OpenAPI page is the one
  HTML response (`/swagger-ui`). There is no XML/CSV negotiation.
- **Error handling** is centralized. Loco services map domain errors to
  HTTP via small helpers — `validate()` returns `422`
  (`Error::CustomError(UNPROCESSABLE_ENTITY, …)`), `http_err()` remaps
  SeaORM `EntityNotFound` to `404` (loco's default would be `500`), and
  `bad_request()` returns `400`. The older services convert their
  `Error` enum (11 variants, `thiserror`-derived) into the `ApiResponse`
  error envelope with the appropriate status code.
- Best-effort side effects (audit write, event publish) **never fail the
  request**: they log a `WARN` on error and the response still succeeds.

---

## 6. Additional API layers (older services)

The older Axum services carry two extra API layers beyond REST. The
loco services do **not** (they are REST/JSON only).

### 6.1 FHIR R5

- Mounted under `/fhir/<Resource>` (e.g. `/fhir/Person`).
- Five endpoints per resource: `GET /fhir/Person/{id}`,
  `POST /fhir/Person`, `PUT /fhir/Person/{id}`,
  `DELETE /fhir/Person/{id}`, `GET /fhir/Person` (search).
- FHIR search parameters: `name`, `family`, `given`, `identifier`,
  `birthdate`, `gender`, `_count`.
- Resource converters and bundle handling live in `src/api/fhir/`.
- **Status:** implemented in person (and the other older services as a
  partial Person/entity resource); **not** present in the loco services.

### 6.2 gRPC (Tonic)

- Reserved high-throughput interface mirroring REST/FHIR.
- **Status: stub.** The `.proto` service is not defined and `serve()` is
  a no-op; the module (`src/api/grpc/mod.rs`) only sketches the intended
  Tonic wiring. Callers should use the REST API until it is implemented.

---

## 7. Pagination, search params, masking on reads

- **List caps.** `GET /api/<plural>` is capped (loco: 100 active rows,
  newest first, soft-deleted excluded) as a guard against unbounded
  responses. `/search` is capped too (loco: 50; `ILIKE '%q%'` over
  active rows). `check-duplicates` does an in-memory full scan up to a
  named cap (`CHECK_DUPLICATES_SCAN_CAP = 1000`) and logs a `WARN` when
  it hits the cap so truncation is observable; lifting it requires
  blocking / candidate pre-selection.
- **Pagination.** The older services' search takes `limit`
  (default 10, max 100) + `offset`. See
  [`../search/index.md`](../search/index.md).
- **Search params** (older services' full-text search): `q`, `limit`,
  `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool).
  Loco services currently expose only `q` (Postgres `ILIKE`; Tantivy
  full-text is deferred).
- **Masking option on reads.** The older services support a
  `mask_sensitive=true` query option and dedicated
  `GET /api/<plural>/{pid}/masked` and `GET /api/<plural>/{pid}/export`
  (GDPR) endpoints. Per-field masking / GDPR export is **deferred** in
  the loco services. See [`../privacy/index.md`](../privacy/index.md).

Audit and event reads (`/audit/recent`, `/{pid}/audit`,
`/events/recent`) are described in [`../auditability/index.md`](../auditability/index.md);
the event stream is an in-memory, per-process ring buffer
(`EventView {kind, pid, name, seq}`) and is not durable.

---

## 8. Metrics endpoint

Where present, Prometheus metrics are exposed as:

| Method | Path | Format |
|---|---|---|
| `GET` | `/metrics.prom` | Prometheus text exposition (`text/plain; version=0.0.4`) |

Configure the scraper with `metrics_path: /metrics.prom`. The metric
inventory (entity-CRUD counters, an HTTP request counter, latency
histograms) lives in each older service's `src/metrics.rs`, with the
handler in `src/api/rest/handlers.rs`.

- **Status:** implemented in the older Axum services (e.g. person).
  The loco services rely on loco/OpenTelemetry observability and do not
  yet expose `/metrics.prom`. See
  [`../observability/index.md`](../observability/index.md) and
  [`../../agents/share/observability.md`](../../agents/share/observability.md).

---

## 9. Implemented vs deferred (summary)

| Capability | Loco services (org / care-pathway / case) | Older Axum services (person / …) |
|---|---|---|
| CRUD + soft-delete | Implemented | Implemented |
| `ApiResponse` envelope | No (raw loco JSON) | Yes |
| Search | `ILIKE` (`q` only) | Tantivy full-text (`q`,`fuzzy`,`phonetic`,paging) |
| Match / check-duplicates / merge | Implemented | Implemented |
| Batch `deduplicate` | Deferred | Implemented |
| OpenAPI / Swagger | Hand-written, test-pinned | Utoipa-derived |
| JWT (MaybeAuthUser / AuthUser / whoami) | Implemented | — |
| Blanket `/api/*` enforcement | Flag, off by default | — |
| FHIR R5 | No | Implemented (partial) |
| gRPC (Tonic) | No | Stub |
| Privacy masking / GDPR export | Deferred | Implemented |
| `/metrics.prom` | Implemented | Implemented |

---

## Cross-references

- [`../../agents/share/restful.md`](../../agents/share/restful.md) — brief shared RESTful note
- [`../authentication/index.md`](../authentication/index.md) — token issuance + verification (planned sibling topic)
- [`../auditability/index.md`](../auditability/index.md) — audit log + event stream (planned sibling topic)
- [`../privacy/index.md`](../privacy/index.md) — masking + GDPR export (planned sibling topic)
- [`../observability/index.md`](../observability/index.md) — tracing + metrics (planned sibling topic)
- [`../validation/index.md`](../validation/index.md) — validation rules / `422` (planned sibling topic)
- [`../search/index.md`](../search/index.md) — search params + pagination (planned sibling topic)
- [`../index.md`](../index.md) — monorepo spec index
- Per-entity API references, e.g.
  [`../../person/person-service-rust-crate/AGENTS/restful.md`](../../person/person-service-rust-crate/AGENTS/restful.md)
  and
  [`../../organization/organization-service-rust-crate/AGENTS.md`](../../organization/organization-service-rust-crate/AGENTS.md)
