## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

API URLs are version-free (`/api/workers`, not `/api/v1/workers`);
clients select the representation version with the `Accepts-version`
request header (default `1.0`) — see
[`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).
Implemented by `src/api/rest/version.rs` (`require_version_mw`).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/workers/*` + `/api/audit/*` + `/api/health` |
| Auth (Axum) | `GET /api/whoami` — echo the verified PASETO bearer-token claims (`401` without a valid token) |
| FHIR R5 (Axum) | `Worker` CRUD + search under `/fhir/Worker` (handlers implemented and **mounted** via `fhir_routes()` in `App::routes`; pinned by `tests/api_integration_test.rs::test_fhir_worker_route_is_mounted`) |
| gRPC (Tonic) | Stubbed |
| Web UI | Full set documented in project-root [`spec.md`](../../spec/index.md) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

Authentication is opt-in per handler by default: taking an `AuthUser`
argument requires a valid `Authorization: Bearer <paseto>` token,
verified offline (PASETO `v4.public`, Ed25519) against the auth-service
published key set (see §13 T-1a). The key set comes from
`WORKER_PASETO_KEYS` (key-set JSON), or — when `WORKER_PASETO_KEYS_URL`
is set — is fetched once at boot from that URL
(`/.well-known/paseto-keys` on the auth service); the fetched set wins
over the env key set, and any fetch failure logs a warning and falls
back to the env path, so the service **always boots** (§13 T-1b fetch
item; no refresh loop). **Blanket enforcement** (§13 T-1b) is
implemented and **off by default**: when `WORKER_REQUIRE_AUTH` is truthy
(`1`/`true`/`yes`/`on`; read once at router construction — restart to
change), every route on both router surfaces requires a valid bearer
token and returns `401` otherwise, except the public allow-list:
`/_health`, `/_ping`, `/api/health`, `/api-docs/openapi.json`,
`/metrics.prom`, and `/swagger-ui*`. The `/fhir` surface is deliberately
protected (worker PII).

Authorization (ABAC, inside the same guard — so only when
`WORKER_REQUIRE_AUTH` is on): the request's action is derived from the
HTTP method plus this crate's destructive named POSTs
(`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the configured policy (`WORKER_ABAC_POLICY` inline JSON /
`WORKER_ABAC_POLICY_FILE`; unset/unparsable ⇒ built-in default: any
authenticated subject reads, `access=write` writes, `access=admin`
adds delete/merge/deduplicate, `svc=true` does everything) over the
token's `attrs` claim. A valid token the policy denies gets `403`
with the deciding rule; see
[authorization-attributes](../../../agents/share/authorization-attributes.md).

### 9.1 Cross-service link endpoints

Worker is a link **originator** in the federated cross-service graph
(domain model §5.4). Its outbound-edge surface, mirroring the existing
controller style:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/workers/{pid}/links` | create/upsert an outbound edge (`same_identity` → person, `employed_by` → organization) |
| `GET` | `/api/workers/{pid}/links` | list this worker's outbound edges |
| `DELETE` | `/api/workers/{pid}/links/{id}` | soft-delete an edge (emits `unlinked`) |

Writes are **optimistic** — the assertion is stored and a `linked` /
`unlinked` event is published; the target service is **not** called
(§8.6). Verification status is the read-side aggregator's concern, not
returned here. Graph traversal (`neighbors` / `single-view`) lives in the
separate `link-graph-service-with-loco` aggregator, not this service. See
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md).


> **2026-07-19 — stored review queue + decision endpoints.** The batch
> scan now **persists** its candidate pairs in a `review_queue` table
> (migration `m20260719_000001`; normalized pair order under a UNIQUE
> constraint, so a re-scan upserts in place: score columns refresh,
> decided rows keep their decision, and item ids are stable across
> scans — the scan response reports the stored rows). Two endpoints:
> `GET /api/workers/review-queue[?status=&limit=]` lists the stored
> queue (newest first, limit cap 500; unknown status token → `422`),
> and `POST /api/workers/review-queue/{id}/decision` with
> `{"status": "confirmed" | "rejected"}` decides a `pending` item —
> the transition guard is first-writer-wins in SQL (`404` unknown id,
> `422` already decided); the reviewer identity is the verified bearer `sub` (or absent).
> Under ABAC the decision POST derives as a `write` action (not
> destructive-classed).
