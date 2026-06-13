## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------+
|                 case-front-end-with-svelte                   |
|  SvelteKit 2 SPA · Svelte 5 runes · TypeScript strict        |
|  routes: /  /new  /[pid]  /[pid]/edit                        |
|  lib/api: client.ts → cases.ts (repository)                  |
+------------------------------+-------------------------------+
                               | REST (raw loco JSON, no envelope)
                               | PUBLIC_API_BASE_URL (default :5150)
+------------------------------v-------------------------------+
|                  case-service-rust-crate                     |
|  loco.rs 0.16 (Axum 0.8) · port 5150                         |
|  controllers/cases.rs                                        |
|    CRUD + /search + /match + /check-duplicates + /merge      |
|    + /audit + /events + /whoami + /api-docs + /swagger-ui    |
|  models/cases.rs  (CRUD over the JSONB payload)              |
|  auth.rs (RS256 JWT) · merge.rs · streaming.rs · openapi.rs  |
+--------------+-------------------------------+---------------+
               |  path dependency (Cargo)      |
+--------------v---------------+  +------------v---------------+
|  case-matcher                |  |  PostgreSQL (SeaORM 1.1)   |
|  pure library, no IO         |  |  cases · audit_logs ·      |
|  MatchingEngine ·            |  |  merge_records             |
|  MatchConfig · Case          |  |  pid · title · data JSONB  |
+------------------------------+  +----------------------------+
```

Dependency direction is strictly downward: front-end → service →
matcher. The matcher depends on nothing in the workspace (serde,
strsim, unicode-normalization, thiserror only). The service declares
`case-matcher = { path = "../case-matcher-rust-crate" }` and uses the
matcher's `Case` directly as its API DTO — there is no adapter layer
(contrast with the person entity).

### 8.2 Service layout (loco.rs)

```
case-service-rust-crate/
├── src/
│   ├── app.rs                  loco Hooks (routes, truncate)
│   ├── bin/main.rs             loco CLI entrypoint
│   ├── auth.rs                 RS256 JWT verification (embeds authentication-verifier)
│   ├── merge.rs                pure merge_cases
│   ├── streaming.rs            in-memory CaseEvent ring buffer
│   ├── validation.rs           422 validation (title, dates, identifiers)
│   ├── openapi.rs              hand-written OpenAPI 3 document
│   ├── controllers/
│   │   ├── cases.rs            CRUD + search + match + check-duplicates + merge + audit + events + whoami
│   │   └── docs.rs             /api-docs/openapi.json + /swagger-ui
│   └── models/
│       ├── cases.rs            CRUD helpers over the payload
│       ├── audit_logs.rs       record / recent / for_entity
│       ├── merge_records.rs    merge history
│       └── _entities/…         SeaORM entities
├── migration/src/
│   ├── m20220101_000001_cases.rs
│   ├── m20220101_000002_audit_logs.rs
│   └── m20220101_000003_merge_records.rs
├── config/{development,production,test}.yaml
└── tests/{matching.rs, requests/cases.rs}
```

Run with `cargo loco start` (needs PostgreSQL; `auto_migrate` on in
development). The front-end runs with `pnpm dev` against
`PUBLIC_API_BASE_URL`.

### 8.3 Matching data flow

- **`/match`** — request carries `{query, candidates}`; the controller
  calls `MatchingEngine::rank` and returns the scored pairs. No
  database access.
- **`/check-duplicates`** — request carries a `Case`; the controller
  loads up to `CHECK_DUPLICATES_SCAN_CAP` (= 1 000) active rows,
  deserialises each payload, calls `match_cases` per candidate, and
  returns hits with `is_match == true`, sorted by score (a
  `tracing::warn!` fires when the scan hits the cap). *(roadmap:
  replace the full scan with search-based candidate blocking.)*

### 8.4 SSO integration

The service is a JWT *verifier*, not an issuer. It embeds the
[authentication entity](../../authentication/)'s
`authentication-verifier` crate, builds a process-wide verifier from
`CASE_JWKS` / `CASE_JWT_ISSUER` / `CASE_JWT_AUDIENCE`, and exposes
`AuthUser` / `MaybeAuthUser` extractors. `whoami` is protected; CRUD
and merge stamp the audit `actor` from the token's `sub` when present.
Blanket `/api/*` enforcement and JWKS-over-HTTP fetch from the auth
service at boot are follow-ups (§13 T-7).

### 8.5 Deployment topology (governmental scale)

Today: one stateless service instance + PostgreSQL, the SPA served as
static assets. Target shape *(roadmap, §15)*, consistent with
[`agents/share/architecture.md`](../../agents/share/architecture.md)
and [`agents/share/availability.md`](../../agents/share/availability.md):

- N stateless service replicas behind a load balancer; PostgreSQL
  primary + replicas; connection pooling.
- Blanket JWT verification at the service edge against the central
  auth-service JWKS (offline, no per-request auth-service call).
- Durable event bus for CRUD/merge events; OTLP observability pipeline.
- Per-jurisdiction deployment with cross-registry linkage through
  deterministic identifiers rather than shared databases.
- A privacy tier (per-field masking, GDPR data-subject export)
  in front of the read API (roadmap, §15).
