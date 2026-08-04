# Organization Service

A registry of **organization identities**
([schema.org/Organization](https://schema.org/Organization)): CRUD +
matching, built on **loco.rs** and embedding the canonical
[organization-matcher](../organization-matcher-rust-crate).

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [organization-front-end-with-svelte](../organization-front-end-with-svelte)

## API

API URLs are version-free; select the version with the
`Accepts-version` request header (default `1.0`) — see
[`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/organizations` | Create |
| GET | `/api/organizations?limit=&offset=` | List (paginated: `X-Total-Count`/`X-Limit`/`X-Offset`) |
| GET | `/api/organizations/{pid}` | Fetch (record-level ABAC; a `mask`-obligation allow returns the redacted view) |
| GET | `/api/organizations/{pid}/masked` | The masked view (telephone/email/street line/fiscal identifiers redacted) |
| GET | `/api/organizations/{pid}/export` | GDPR right-of-access export (audited every call) |
| PUT | `/api/organizations/{pid}` | Update |
| DELETE | `/api/organizations/{pid}` | Soft-delete |
| POST | `/api/organizations/match` | Rank `{query, candidates}` |
| POST | `/api/organizations/check-duplicates` | Match query vs stored orgs (blocked on the search index) |
| POST | `/api/organizations/deduplicate` | Batch-scan stored orgs pairwise; persists candidates in the stored review queue (destructive-classed under ABAC) |
| GET | `/api/organizations/review-queue` | Stored review queue (filter `status`, `limit`) |
| POST | `/api/organizations/review-queue/{id}/decision` | Decide a pending item (`confirmed` / `rejected`; first-writer-wins) |
| GET | `/api/organizations/search?q=[&fuzzy][&phonetic]` | Tantivy full-text search — fuzzy = typo-tolerant, phonetic = Soundex |
| POST | `/api/organizations/merge` | Fold a duplicate into a survivor |
| GET | `/api/organizations/merges/recent` | Merge-history records |
| POST | `/api/organizations/import` | Bulk import (multipart JSONL/CSV) → `202 {job_id}` |
| POST | `/api/organizations/export` | Bulk export → `202 {job_id}` |
| GET | `/api/organizations/bulk-jobs` | Recent bulk import/export jobs |
| GET | `/api/organizations/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api/organizations/audit/recent` · `/{pid}/audit` | Audit trail |
| GET | `/api/organizations/events/recent` | In-memory event stream (`EventView`) |
| GET | `/fhir/Organization{,/{id}}` · `/fhir/metadata` | FHIR R5 API (family reference implementation) |
| GET | `/swagger-ui` · `/api-docs/openapi.json` | API docs |
| GET | `/metrics.prom` | Prometheus metrics (root path, public) |

The request/response body for an organization **is** the
`organization_matcher::Organization` shape, serialized snake_case
(`name`, `legal_name`, `alternate_names`, `identifiers`, `url`,
`same_as`, `address`, `jurisdiction`, `founding_date`, `keywords`).
schema.org publishes the camelCase property names (`legalName`,
`sameAs`, …); the wire format here is the Rust DTO's snake_case
(entity spec OQ-1, resolved). The `/fhir/Organization` resource is a
separate, standards-faithful FHIR representation of the same stored
record — see [AGENTS.md](./AGENTS.md).

## Quick start

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/organization_service_development
cargo loco start        # migrations auto-run in development

curl -s localhost:5150/api/organizations -H 'content-type: application/json' \
  -d '{"name":"Acme, Inc.","jurisdiction":"US","url":"https://acme.com"}'
# -> {"pid":"<uuid>","name":"Acme, Inc."}

# Batch-scan stored orgs pairwise; persists candidate pairs in the review queue.
curl -s -X POST localhost:5150/api/organizations/deduplicate
# -> {"organizations_scanned":...,"duplicates_found":...,"queued_for_review":...,"review_items":[...]}

# Decide a pending review-queue item (confirmed or rejected; first-writer-wins).
curl -s localhost:5150/api/organizations/review-queue/<id>/decision \
  -H 'content-type: application/json' -d '{"status":"confirmed"}'

# Fold a confirmed duplicate into a survivor.
curl -s localhost:5150/api/organizations/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<dup-uuid>","reason":"confirmed"}'
# -> {"main_pid":"...","duplicate_pid":"...","main":{<merged Organization>}}

# Inspect the verified bearer-token claims (401 without a token).
curl -s localhost:5150/api/organizations/whoami -H 'authorization: Bearer <paseto>'

# Scrape Prometheus metrics (root path, public even under blanket auth).
curl -s localhost:5150/metrics.prom
```

## Testing

```bash
cargo test                   # DB-free: unit + matcher embedding + JSON round-trip
cargo test -- --ignored      # request-level suite; needs Postgres (config/test.yaml)
cargo clippy --all-targets
```

The request-level tests (`tests/requests/organizations.rs`) boot the
real app against `config/test.yaml` and are `#[ignore]`d so the
default run stays green without a database. Validation failures
(blank `name`) return `422 Unprocessable Entity`.

## Status

Done: CRUD + matching + **Tantivy full-text search** (fuzzy + phonetic,
`check-duplicates` blocked on the index) + a stored review queue + audit
log + event streaming (in-memory + a durable outbox with a Fluvio relay,
default-off) + record merge + per-field masking + the audited GDPR
export + OpenAPI/Swagger + Prometheus metrics (`/metrics.prom`) +
offline PASETO v4.public verification + ABAC policy authorization
(blanket `/api/*` and `/fhir/*` enforcement is wired, default-off; the
published key set is fetched over HTTP at boot when
`ORGANIZATION_PASETO_KEYS_URL` is set and refreshed periodically
thereafter, with warn + env fallback) + a **FHIR R5 API** (`Organization`
— this crate is the family's reference implementation) + header-based
API versioning + async bulk import/export (JSONL/CSV). Still deferred
(see [spec §13](./spec/index.md)): richer validation beyond identifier
check-digits (URL/country-code format), real-time duplicate check on
create, and an S3 bulk-artifact backend. Auth is provided by the central
[authentication-service](../../authentication/authentication-service-with-loco).

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against
a published Ed25519 key (RS256/JWKS decommissioned). See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
