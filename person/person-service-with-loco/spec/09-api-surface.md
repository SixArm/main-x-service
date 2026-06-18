## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/persons/*` + `/api/audit/*` + `/api/health` |
| FHIR R5 (Axum) | Person CRUD + search under `/fhir/Person` |
| gRPC (Tonic) | Stubbed; not yet implemented |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0 via utoipa) |

All REST endpoints return `{ "success": bool, "data": …, "error": … }`.
HTTP status codes follow REST conventions: `409` for duplicate
detection on create, `422` for validation failure.

### 9.1 Cross-service link endpoints

The write side of cross-service entity links (§5.4, §8.6) adds three
endpoints under the existing person resource, mirroring the controller
style above and per
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md):

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/v1/persons/{pid}/links` | Create / upsert an outbound edge; emits `linked` |
| `GET` | `/api/v1/persons/{pid}/links` | List this person's outbound edges |
| `DELETE` | `/api/v1/persons/{pid}/links/{id}` | Soft-delete an edge; emits `unlinked` |

Creating a link is **optimistic** — it stores the assertion and emits a
`linked` event without calling the target service. Verification status is
not returned here; it is the aggregator's read-model concern.

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
| `POST` | `/api/v1/persons/import` | `202 {job_id}` — body: `format`, `dedupe_mode`, `dry_run`; file upload |
| `GET` | `/api/v1/persons/import/{id}` | Job status + counts + `errors_url` + `review_url` |
| `POST` | `/api/v1/persons/export` | `202 {job_id}` — body: `format`, `filter`, `fields`, `include_soft_deleted`, `masking_profile` |
| `GET` | `/api/v1/persons/export/{id}` | Job status + `download_url` |
| `GET` | `/api/v1/persons/bulk-jobs` | List (filter by `kind`/`status`); `GET .../{id}` for one |

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

