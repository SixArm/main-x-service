## 9. API Surface

Complete reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/things/*` + `/api/audit/*` + `/api/health` |
| Auth (Axum) | `GET /api/whoami` — echo the verified PASETO bearer-token claims (`401` without a valid token) |
| Metrics | `GET /metrics.prom` at the application **root** (not under `/api`) — Prometheus text-exposition (`text/plain; version=0.0.4`); scrape with `metrics_path: /metrics.prom` |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../../spec/index.md) |
| Docs | Swagger UI at `/swagger-ui` |

This crate does **not** expose a FHIR R5 surface — Things are not a
FHIR-resource concern.

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

**Server-managed fields.** `id`, `is_deleted`, `created_at`,
`updated_at`, and every collection field (`alternate_names`,
`identifiers`, `images`, `same_as`) are **optional on the wire**
(`#[serde(default)]`): `POST /api/things` previously *required* all of
them and answered `422 missing field id` (or `created_at`, …) to a
body it would then have ignored or discarded. The create handler now
mints a fresh `id` whenever the wire value is nil, and the repository
stamps `created_at`/`updated_at` to "now" on insert (preserving
`created_at` and refreshing `updated_at` on update), matching the
`created_at`/`updated_at` fix already shipped in the event service.
`name` — the one field the server does **not** own — is also
`#[serde(default)]` so an omitted value now reaches the normal
validation path (`422 validation_error`, field `name`) instead of
being refused by the JSON extractor before any handler code runs.

Authentication is opt-in per handler: taking an `AuthUser` argument
requires a valid `Authorization: Bearer <paseto>` token, verified
offline (PASETO `v4.public`, Ed25519) against the auth-service
published key set (see §13 T-4).

Key-set configuration (issuer/audience from `THING_TOKEN_ISSUER` /
`THING_TOKEN_AUDIENCE`, defaults `authentication-service` /
`main-x-service`):

- `THING_PASETO_KEYS_URL` set (non-blank) — the key-set JSON is
  fetched over HTTP **once at boot** (async, in `after_routes`, before
  the routers/middleware capture the verifier) from the auth service
  (normally `/.well-known/paseto-keys`). A successful fetch **wins**
  over `THING_PASETO_KEYS`; a failed fetch warn-logs and falls back to
  the env path. No refresh loop (rotation re-fetch is roadmap, §15).
- Unset/blank — the key set comes from the `THING_PASETO_KEYS` env
  var; absent/unparseable ⇒ an empty reject-all key set.

Either way the service **always boots**.

Blanket enforcement is implemented **default-off**: when
`THING_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
case-insensitive), every `/api/*` route requires a valid bearer token
except the public `/api/health`. Root-level `/_health`, `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` sit
outside the `/api` scope and stay public. The flag is read once at
construction — restart to change. Family contract:
[jwt-enforcement](../../../agents/share/jwt-enforcement.md).

Authorization (ABAC, inside the same guard — so only when
`THING_REQUIRE_AUTH` is on): the request's action is derived from the
HTTP method plus this crate's destructive named POSTs
(`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the configured policy (`THING_ABAC_POLICY` inline JSON /
`THING_ABAC_POLICY_FILE`; unset/unparsable ⇒ built-in default: any
authenticated subject reads, `access=write` writes, `access=admin`
adds delete/merge/deduplicate, `svc=true` does everything) over the
token's `attrs` claim. A valid token the policy denies gets `403`
with the deciding rule; see
[authorization-attributes](../../../agents/share/authorization-attributes.md).

> **2026-07-19 — enriched dedup report.** `POST /api/things/deduplicate`
> now returns the family's person/worker-shaped report: alongside the
> scan counts it carries `auto_merged` (always 0 — no auto-merge path
> here), `queued_for_review`, and `review_items[]` (pair ids,
> `match_score`, `match_quality`, `detection_method`, lowercase
> `status` wire tokens, `created_at`). (The stored-queue + decision
> endpoints below superseded this the same day.)

> **2026-07-19 — stored review queue + decision endpoints.** The batch
> scan now **persists** its candidate pairs in a `review_queue` table
> (migration `m20260719_000001`; normalized pair order under a UNIQUE
> constraint, so a re-scan upserts in place: score columns refresh,
> decided rows keep their decision, and item ids are stable across
> scans — the scan response reports the stored rows). Two endpoints:
> `GET /api/things/review-queue[?status=&limit=]` lists the stored
> queue (newest first, limit cap 500; unknown status token → `422`),
> and `POST /api/things/review-queue/{id}/decision` with
> `{"status": "confirmed" | "rejected"}` decides a `pending` item —
> the transition guard is first-writer-wins in SQL (`404` unknown id,
> `422` already decided); the reviewer identity is not recorded yet (no optional-claims extractor — accepted drift from person/worker).
> Under ABAC the decision POST derives as a `write` action (not
> destructive-classed).
