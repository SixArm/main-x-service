# RESTful API conventions

Monorepo-wide specification for the REST API surface shared by the
**Main X Index** service crates. This is the single reference for how
the services expose HTTP: route shape, status codes, OpenAPI, auth,
CORS, the extra API layers, pagination, masking, and metrics.

It is grounded in the actual code. Two generations of service coexist
and they differ in concrete ways (response envelope, extra API layers),
so each section calls out **loco services** (organization, care-pathway,
case, portfolio) versus the **older Axum services** (person, worker,
place, thing, event, course) where they diverge.

See also the brief shared note
[`../../agents/share/restful.md`](../../agents/share/restful.md) and the
per-entity API references such as
[`../../person/person-service-with-loco/agents/restful.md`](../../person/person-service-with-loco/agents/restful.md)
and
[`../../organization/organization-service-with-loco/AGENTS.md`](../../organization/organization-service-with-loco/AGENTS.md).

---

## 1. API conventions

### 1.1 Transport and shape

- **JSON** request and response bodies (`Content-Type: application/json`).
- **Resource-oriented** routes under `/api/<plural>` where `<plural>`
  is the entity (`/api/organizations`, `/api/persons`, `/api/cases`, …).
  The event front-end calls under `/api/`; the rest are unversioned.
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
(a batch index scan) and mount a health route at `/api/health`. The
privacy reads `GET /api/<plural>/{pid}/masked` and
`GET /api/<plural>/{pid}/export` are **not** older-services-only any
more: organization, care-pathway, case, and portfolio all expose them
too (§7.3). `POST /api/<plural>/deduplicate` itself is the one
older-services-only item left in this list among the loco crates —
organization has it, care-pathway/case/portfolio don't yet (§9,
[`../merge/index.md`](../merge/index.md) §9).

Literal sub-paths (`/search`, `/merge`, `/whoami`, `/merges/recent`, …)
are registered **before** the `/{pid}` captures so the dynamic segment
does not shadow them. This ordering is a hard convention — see the
`routes()` function in
[`organizations.rs`](../../organization/organization-service-with-loco/src/controllers/organizations.rs).

### 1.3 Response envelope: raw loco JSON vs `ApiResponse`

This is the main wire difference between the two generations.

- **Loco services** (organization, care-pathway, case, portfolio) return **raw
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
  [`src/openapi.rs`](../../organization/organization-service-with-loco/src/openapi.rs)
  (`spec()`), which also keeps the doc accurate to the snake_case wire
  format.
- Served by
  [`src/controllers/docs.rs`](../../organization/organization-service-with-loco/src/controllers/docs.rs):
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

The **human session** is a server-side Postgres **cookie session**
(opaque id in an httpOnly cookie). **Cross-service** API calls carry a
short-lived **PASETO v4.public** bearer token (Ed25519-signed), issued by
the central
[authentication-service](../../authentication/authentication-service-with-loco)
and verified **offline** by each peer via the
[authentication-verifier](../../authentication/authentication-verifier-rust-crate)
crate against the service's published Ed25519 key at
`/.well-known/paseto-keys` — no shared secret, no introspection hop. This
replaces the prior RS256 JWT + JWKS model 1:1. See
[`../authentication/index.md`](../authentication/index.md) for the token
flow and [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
for the source-of-truth design. The per-service extractor lives in
[`src/auth.rs`](../../organization/organization-service-with-loco/src/auth.rs).

Two extractors, two postures:

| Extractor | Posture | Use |
|---|---|---|
| `MaybeAuthUser` | Never rejects | Yields `Some(claims)` when a valid token is present, else `None`. Handlers use `.actor()` (the caller `sub`) to stamp the audit / merge `actor` without requiring auth |
| `AuthUser` | Rejects with `401` | A missing/invalid token rejects before the handler runs. `GET /api/<plural>/whoami` takes `AuthUser`, so reaching its body proves end-to-end verification; it echoes the verified claims |

**Blanket enforcement.** A process-wide flag (`<ENTITY>_REQUIRE_AUTH`,
e.g. `ORGANIZATION_REQUIRE_AUTH`) turns on a middleware layer
(`enforce`, wired in `src/app.rs`) that requires a valid PASETO bearer
token (service-to-service) or a valid session (BFF/browser) on
**every route except** the public health/ping and OpenAPI/Swagger paths
(`is_public_path`: `/_health`, `/_ping`, `/api-docs/openapi.json`,
`/swagger-ui*`). It is **off by default** (`1`/`true`/`yes`/`on`,
case-insensitive, are the only truthy values); unset/blank/junk leaves
today's opt-in-per-handler behaviour. This is **implemented and shipped
on all ten entity registries** (not organization alone — see
[`../authentication/index.md`](../authentication/index.md) §7); the SSO
token flow it depends on has been live since 2026-07-04, so activation
is purely a per-deployment operations decision now, not something
waiting on unshipped code.

The verification key source is environment-driven: `<ENTITY>_PASETO_KEYS`
(the published Ed25519 public key set; absent ⇒ empty key set ⇒ every
token rejected, but the service still boots), `<ENTITY>_TOKEN_ISSUER`
(default `authentication-service`), `<ENTITY>_TOKEN_AUDIENCE` (default
`main-x-service`). **Boot-time key fetch over HTTP is implemented**:
when `<ENTITY>_PASETO_KEYS_URL` is set, the service fetches the key set
once at boot via `Verifier::from_paseto_keys_url` (the fetched set wins
over the env key set; a fetch failure falls back to the env path with a
warning, so the service always boots) and, in organization at least,
polls the URL in the background to pick up key rotation. This
supersedes the prior `<ENTITY>_JWKS` / `_JWT_*` RS256 vars, which are
removed, not merely deprecated (see
[`agents/share/security.md`](../../agents/share/security.md) §7).

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

## 6. Additional API layers

FHIR R5 has landed in three loco services, so this section is no longer
"older services only" for that layer; gRPC is still older-services-only
and still a stub.

### 6.1 FHIR R5

- Mounted under `/fhir/<Resource>` (e.g. `/fhir/Person`, `/fhir/Organization`).
- Five endpoints per resource: `GET /fhir/<Resource>/{id}`,
  `POST /fhir/<Resource>`, `PUT /fhir/<Resource>/{id}`,
  `DELETE /fhir/<Resource>/{id}`, `GET /fhir/<Resource>` (search), plus
  `GET /fhir/metadata` (`CapabilityStatement`).
- FHIR search parameters vary by resource — see
  [`agents/share/fhir.md`](../../agents/share/fhir.md) §6.
- **Status:** implemented in the older Axum services (person, worker,
  place, thing, event — course is excluded, mapped instead to a
  non-standard `Basic` resource per fhir.md §3) **and** in three loco
  services — **organization** (`Organization`, the reference
  implementation), **care-pathway** (`PlanDefinition`), and **case**
  (`Task`, carrying the `subject_of`-edge governance from
  [cross-service-linking.md](../../agents/share/cross-service-linking.md)
  §10). This corrects an earlier version of this section that said FHIR
  was "not present in the loco services" — it is present in three of
  the four; **portfolio** is the one loco service still out of FHIR
  scope (no FHIR resource meaningfully models a plan/portfolio, per
  fhir.md §3). Resource converters and bundle handling live in
  `src/api/fhir/` (older services) or `src/fhir/` (loco services).

### 6.2 gRPC (Tonic)

- Reserved high-throughput interface mirroring REST/FHIR. Not present
  in any loco service (organization/care-pathway/case/portfolio) — see
  [`agents/share/overview.md`](../../agents/share/overview.md)'s
  capability matrix, gRPC-stub row.
- **Status: stub**, in the older services that declare it (person,
  worker, event — place and thing declare a `tonic` dependency in
  anticipation of a not-yet-built server but have no `src/api/grpc`
  module either). The `.proto` service is not defined and `serve()` is
  a no-op; the module (`src/api/grpc/mod.rs`, where present) only
  sketches the intended Tonic wiring. Callers should use the REST API
  until it is implemented.

---

## 7. Pagination, search params, masking on reads

### 7.1 Pagination — the family-wide header contract

This section previously described pagination as ad hoc and
inconsistent per service (a fixed loco list cap of 100, a fixed loco
search cap of 50, an older-service search default of 10/max 100, no
shared contract). **That description is superseded.**
[`agents/share/restful.md`](../../agents/share/restful.md)'s Pagination
section is the current, family-wide, present-tense contract for every
collection-read endpoint (`GET /<plural>`, `/search`, and any other
paginated list), and this document defers to it rather than restating
it in full:

- `?limit=` and `?offset=` query params on every collection read;
  totals/limit/offset are reported in **response headers**
  (`X-Total-Count`, `X-Limit`, `X-Offset`), not a body envelope — so a
  bare-JSON-array endpoint (every loco endpoint, §1.3) keeps returning a
  bare array.
- **Omitting both params preserves each endpoint's pre-existing
  behaviour** — the old ad hoc numbers below are exactly what an
  unparameterised request still gets today; they were never wrong, they
  were just not the whole story.
- **`limit` is clamped**, not rejected, to a per-endpoint `MAX_LIMIT`
  (500 for list/search surfaces) — a caller asking for more gets the
  max and an honest `X-Limit`.
- **`offset` is bounded**; a request beyond the bound is `400` (SEC-G7)
  rather than materialising an unbounded number of rows to discard.
- Zero/unparseable values fall back to the default rather than erroring.

The pre-existing per-endpoint defaults this contract preserves:

- **List caps.** `GET /api/<plural>` is capped (loco: 100 active rows,
  newest first, soft-deleted excluded) as a guard against unbounded
  responses absent `limit`/`offset`. `check-duplicates` does an
  in-memory full scan up to a named cap (`CHECK_DUPLICATES_SCAN_CAP =
  1000`) on the crates that still scan; on organization, care-pathway,
  case, and portfolio, `check-duplicates` candidates are search-blocked
  via Tantivy instead (§7.2), so the cap there bounds the index result
  count, not a row-by-row DB scan.
- **`/search`** (loco): historically a fixed cap of 50 with no client
  `limit`/`offset` — see §7.2; that has changed along with the Tantivy
  migration.
- **The older services' search** default `limit` is 10, max 100 (now
  one instance of the family-wide `MAX_LIMIT` clamp rather than a
  bespoke rule). See [`../search/index.md`](../search/index.md).

### 7.2 Search params

Every entity registry's `/search` now takes `q`, `limit`, `offset`,
`fuzzy` (bool), `phonetic` (bool) — Tantivy full-text is **implemented
on all ten registries**, not just the older services (see
[`../search/index.md`](../search/index.md) for the full, corrected
picture; this document previously said the loco services "currently
expose only `q` (Postgres `ILIKE`; Tantivy full-text is deferred)",
which is no longer the case — organization removed its `ILIKE` method
entirely once Tantivy landed). `mask_sensitive` (bool) remains specific
to the older services' richer search response shape (§7.3); portfolio
additionally accepts `kind` as a filter.

### 7.3 Masking option on reads

The older services support a `mask_sensitive=true` query option and
dedicated `GET /api/<plural>/{pid}/masked` and
`GET /api/<plural>/{pid}/export` (GDPR) endpoints. **Per-field masking
and GDPR export are now implemented in the loco services too** —
organization, care-pathway, and portfolio each ship a `src/privacy.rs`
module wired to the ABAC `mask` obligation; case masks and exports
inline without a dedicated module. This corrects an earlier "deferred"
claim here — see [`../privacy/index.md`](../privacy/index.md) §7 for
the corrected per-crate table. `/search` itself carries no
`mask_sensitive` option on the loco services because it returns only
slim `{pid, name}`/`{pid, title}` refs with no sensitive fields to mask
(same reasoning as [`../search/index.md`](../search/index.md) §6.2).

Audit and event reads (`/audit/recent`, `/{pid}/audit`,
`/events/recent`) are described in [`../auditability/index.md`](../auditability/index.md).
The event stream is in-memory **by default**
(`<ENTITY>_EVENT_TRANSPORT=memory`); a durable Postgres outbox +
`FluvioSink` relay is shipped, default-off, on all ten entity
registries — see [`../event-streaming/index.md`](../event-streaming/index.md)
§4/§8, which corrects an earlier "not durable" claim here that no
longer reflects what ships, only what's on by default.

---

## 8. Metrics endpoint

Where present, Prometheus metrics are exposed as:

| Method | Path | Format |
|---|---|---|
| `GET` | `/metrics.prom` | Prometheus text exposition (`text/plain; version=0.0.4`) |

Configure the scraper with `metrics_path: /metrics.prom`. The metric
inventory (entity-CRUD counters, an HTTP request counter, latency
histograms) lives in each service's `src/metrics.rs`, with the handler
in `src/api/rest/handlers.rs` (older services) or
`src/controllers/metrics.rs` (loco services).

- **Status: implemented on every service, both generations** —
  person/worker/place/thing/event/course (older Axum) **and**
  organization/care-pathway/case/portfolio/authentication (loco), each
  via its own `src/metrics.rs` + handler. This corrects an earlier
  version of this section that said the loco services "do not yet
  expose `/metrics.prom`" — they do; `/metrics.prom` (Prometheus) is
  separate from OTLP distributed tracing, which is a genuinely
  per-crate-varying picture (§9's OTLP row, and
  [`../observability/index.md`](../observability/index.md)). See also
  [`../../agents/share/observability.md`](../../agents/share/observability.md).

---

## 9. Implemented vs deferred (summary)

| Capability | Loco services (org / care-pathway / case / portfolio) | Older Axum services (person / …) |
|---|---|---|
| CRUD + soft-delete | Implemented | Implemented |
| `ApiResponse` envelope | No (raw loco JSON) | Yes |
| Search | **Tantivy full-text** (`q`,`fuzzy`,`phonetic`,paging) — historical `ILIKE` superseded, see [`../search/index.md`](../search/index.md) | Tantivy full-text (`q`,`fuzzy`,`phonetic`,paging) |
| Match / check-duplicates / merge | Implemented; `check-duplicates` search-blocked via Tantivy on all four | Implemented |
| Batch `deduplicate` | Implemented in **organization** only; care-pathway/case/portfolio don't yet expose it | Implemented |
| OpenAPI / Swagger | Hand-written, test-pinned | Utoipa-derived |
| Token auth (MaybeAuthUser / AuthUser / whoami) — PASETO v4.public, RS256/JWKS decommissioned | **Implemented** (shipped 2026-07-04; no code follow-up pending) | Implemented (same PASETO model, all six services) |
| Blanket `/api/*` enforcement (`<ENTITY>_REQUIRE_AUTH`) | Implemented, flag off by default | **Implemented, flag off by default** — this corrects an earlier "—" here; all six older services carry it too, not just the loco four (§4) |
| FHIR R5 | Organization/care-pathway/case: **implemented**; portfolio: no (no meaningful FHIR resource) | Implemented (partial; course maps to a non-standard `Basic` resource instead) |
| gRPC (Tonic) | No | Stub, in person/worker/event only |
| Privacy masking / GDPR export | **Implemented** — organization/care-pathway/portfolio via `src/privacy.rs`, case inline (§7.3) | Implemented |
| `/metrics.prom` | Implemented | Implemented |
| OTLP distributed tracing | organization: implemented; care-pathway/case/portfolio: not yet (see [`../observability/index.md`](../observability/index.md)) | person/worker/event/course/place/thing: implemented |

---

## Cross-references

- [`../../agents/share/restful.md`](../../agents/share/restful.md) — brief shared RESTful note, including the current pagination header contract (§7.1)
- [`../authentication/index.md`](../authentication/index.md) — token issuance + verification
- [`../auditability/index.md`](../auditability/index.md) — audit log + event stream
- [`../privacy/index.md`](../privacy/index.md) — masking + GDPR export
- [`../observability/index.md`](../observability/index.md) — tracing + metrics
- [`../validation/index.md`](../validation/index.md) — validation rules / `422`
- [`../search/index.md`](../search/index.md) — search params + pagination
- [`../event-streaming/index.md`](../event-streaming/index.md) — the durable outbox + Fluvio transport (§7.3)
- [`../merge/index.md`](../merge/index.md) — record merge, including the loco-lineage batch-`deduplicate` gap (§9)
- [`../index.md`](../index.md) — monorepo spec index
- Per-entity API references, e.g.
  [`../../person/person-service-with-loco/agents/restful.md`](../../person/person-service-with-loco/agents/restful.md)
  and
  [`../../organization/organization-service-with-loco/AGENTS.md`](../../organization/organization-service-with-loco/AGENTS.md)
