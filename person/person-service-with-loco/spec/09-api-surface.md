## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 16 endpoints under `/api/persons/*` + `/api/audit/*` + `/api/health` — including `GET /api/persons`, the database-backed collection list added after a live investigation found `/persons/search?q=*` unreliable for enumeration (see `CHANGELOG.md`) |
| Auth (Axum) | `GET /api/whoami` — echo the verified PASETO bearer-token claims (`401` without a valid token) |
| FHIR R5 (Axum) | Person CRUD + search under `/fhir/Person` |
| gRPC (Tonic) | Stubbed; not yet implemented |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure.

Authentication is opt-in per handler by default: taking an `AuthUser`
argument requires a valid `Authorization: Bearer <paseto>` token,
verified offline (PASETO `v4.public`, Ed25519) against the
auth-service published key set (see §13 T-1a). The key set comes from
`PERSON_PASETO_KEYS` (key-set JSON), or — when `PERSON_PASETO_KEYS_URL`
is set — is fetched once at boot from that URL
(`/.well-known/paseto-keys` on the auth service); the fetched set wins
over the env key set, and any fetch failure logs a warning and falls
back to the env path, so the service **always boots** (§13 T-1c fetch
item; no refresh loop). **Blanket enforcement**
(§13 T-1b) is wired on both router surfaces and gated by the
default-off `PERSON_REQUIRE_AUTH` env flag (`1`/`true`/`yes`/`on` ⇒
on; unset/blank/junk ⇒ off; read once at boot — restart to change).
When on, every route requires a valid bearer token except the public
allow-list: `/api/health`, loco's `/_health` / `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom`.

Authorization (ABAC, inside the same guard — so only when
`PERSON_REQUIRE_AUTH` is on): the request's action is derived from
the HTTP method plus this crate's destructive named POSTs
(`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the configured policy (`PERSON_ABAC_POLICY` inline JSON /
`PERSON_ABAC_POLICY_FILE`; unset/unparsable ⇒ built-in default: any
authenticated subject reads, `access=write` writes, `access=admin`
adds delete/merge/deduplicate, `svc=true` does everything) over the
token's `attrs` claim. A valid token the policy denies gets `403`
with the deciding rule; see
[authorization-attributes](../../../agents/share/authorization-attributes.md).

### 9.1 Cross-service link endpoints

The write side of cross-service entity links (§5.4, §8.6) adds three
endpoints under the existing person resource, mirroring the controller
style above and per
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md):

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/persons/{pid}/links` | Create / upsert an outbound edge; emits `linked` |
| `GET` | `/api/persons/{pid}/links` | List this person's outbound edges |
| `DELETE` | `/api/persons/{pid}/links/{id}` | Soft-delete an edge; emits `unlinked` |

Creating a link is **optimistic** — it stores the assertion and emits a
`linked` event without calling the target service. Verification status is
not returned here; it is the aggregator's read-model concern.

These three per-record endpoints return the crate's uniform
`{success,data,error}` envelope (`ApiResponse<T>`), matching every other
person REST endpoint — fixed 2026-08-03; they previously returned bare
JSON, which a front-end client that unwraps `.data` would have silently
read as `undefined` rather than erroring. The bulk aggregator endpoint
(`GET /api/persons/links`, §9.3-adjacent — see
[cross-service linking §4.2](../../../agents/share/cross-service-linking.md))
is deliberately **not** wrapped: it stays bare `{"edges": [...]}` for the
link-graph aggregator's HTTP client, which deserializes that shape
directly.

### 9.2 Bulk import / export

The async, job-based bulk contract is fixed family-wide in
[bulk import/export](../../../agents/share/bulk-import-export.md) (execution
model on `bg_pg`, the five endpoints, JSONL/CSV/Parquet codecs,
upsert-by-stable-key + dedupe-to-review, the per-row error report, and
export masking + audit). This section declares only the **person-specific**
bits; the shared doc is the source of truth for everything else.

The five endpoints (shared doc §4) mount under the person resource:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/persons/import` | `202 {job_id}` — body: `format`, `dedupe_mode`, `dry_run`; file upload |
| `GET` | `/api/persons/import/{id}` | Job status + counts + `errors_url` + `review_url` |
| `POST` | `/api/persons/export` | `202 {job_id}` — body: `format`, `filter`, `fields`, `include_soft_deleted`, `masking_profile` |
| `GET` | `/api/persons/export/{id}` | Job status + `download_url` |
| `GET` | `/api/persons/bulk-jobs` | List (filter by `kind`/`status`); `GET .../{id}` for one |

**Stable key(s) for upsert** (shared doc §6, §10). A row upserts in place
when it carries either:

- a **scheme-scoped national / health identifier** — the same identifiers
  the matcher short-circuits on (§5.3.1), keyed by `(identifier_type,
  system, value)`: e.g. UK NHS number, US SSN, BR CPF, FR NIR, IN Aadhaar,
  JP My Number, MX CURP, SE personnummer, DE KVNR, NL BSN, … (the 26
  schemes routed in §5.3.1); or
- the record **`pid`** (the person UUID) when present in the row.

A row with neither runs the normal duplicate detection (§5.2 review queue),
routing likely duplicates to the review queue with `provenance = import`
(matching the cross-service-linking provenance vocabulary, §5.4).

**CSV column set + flattening** (shared doc §5). CSV is the operator /
spreadsheet format and is lossy for deep nesting — steer fidelity-sensitive
loads to **JSONL** (the lossless reference). Flat columns:

- **scalar** (one column each): `pid`, `gender`, `birth_date`,
  `marital_status`, `multiple_birth`, `deceased`, `deceased_datetime`,
  `active`, plus the primary name parts `name.family`, `name.given`,
  `name.prefix`, `name.suffix`, `name.use_type`;
- **single nested object** → dotted columns: the primary address
  (`address.line`, `address.locality`, `address.region`, `address.postcode`,
  `address.country`) and `managing_organization.reference`;
- **arrays / arrays-of-objects** → a single **JSON-encoded cell** each:
  `identifiers`, `additional_names`, `telecom`, `addresses` (beyond the
  primary), `identity_documents`, `emergency_contacts`, `links`, and
  cross-service `entity_links`.

**Export sensitivity** (shared doc §8). Person rows are **personal data**
(HIPAA / UK DPA / GDPR — see §12 Compliance): export defaults to **masked**;
full / unmasked output requires a `masking_profile` selecting elevated
authorisation and must never reveal more than the caller could read one
record at a time. `include_soft_deleted` defaults `false` and is gated.
**Every export is audited** (actor, filter, format, row count, masking
profile, timestamp — written even for a zero-row export). The existing
single-record GDPR export is the single-subject special case of this
machinery (filter = one `pid`).


> **2026-07-19 — stored review queue + decision endpoints.** The batch
> scan now **persists** its candidate pairs in a `review_queue` table
> (migration `m20260719_000001`; normalized pair order under a UNIQUE
> constraint, so a re-scan upserts in place: score columns refresh,
> decided rows keep their decision, and item ids are stable across
> scans — the scan response reports the stored rows). Two endpoints:
> `GET /api/persons/review-queue[?status=&limit=]` lists the stored
> queue (newest first, limit cap 500; unknown status token → `422`),
> and `POST /api/persons/review-queue/{id}/decision` with
> `{"status": "confirmed" | "rejected"}` decides a `pending` item —
> the transition guard is first-writer-wins in SQL (`404` unknown id,
> `422` already decided); the reviewer identity is the verified bearer `sub` (or absent). Each decision writes a `review_decision` audit row.
> Under ABAC the decision POST derives as a `write` action (not
> destructive-classed).
