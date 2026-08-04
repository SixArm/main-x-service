# Case Service — documentation index

A loco.rs registry of governmental case records: CRUD + matching,
embedding the canonical case-matcher.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | How to work here; API surface; MVP scope. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

The full v0.1 surface (spec §6). The `Case` body shape is the
`case-matcher` type (title, alternate titles, case number, agency, case
type, status, priority, opened date, subjects, keywords, identifiers,
sameAs, languages).

```text
create   ──>  POST   /api/cases                       {Case}              -> {pid, title}
read     ──>  GET    /api/cases/{pid}                                     -> Case
list     ──>  GET    /api/cases                                           -> [{pid, title}]
search   ──>  GET    /api/cases/search?q=housing                          -> [{pid, title}]  (Tantivy: ?fuzzy=, ?phonetic=)
masked   ──>  GET    /api/cases/{pid}/masked                              -> Case (always redacted)
export   ──>  GET    /api/cases/{pid}/export                              -> GDPR right-of-access envelope (audited)
update   ──>  PUT    /api/cases/{pid}                 {Case}              -> {pid, title}
delete   ──>  DELETE /api/cases/{pid}                                     -> {}
erase    ──>  POST   /api/cases/{pid}/erase                               -> GDPR Art. 17 erasure (destructive, access=admin)
match    ──>  POST   /api/cases/match                 {query, candidates} -> ranked results
dedupe   ──>  POST   /api/cases/check-duplicates      {Case query}        -> [{pid, title, score, confidence, is_match}]
merge    ──>  POST   /api/cases/merge                 {main_pid, duplicate_pid, reason?}
                                                                          -> {main_pid, duplicate_pid, main}
merges   ──>  GET    /api/cases/merges/recent                             -> [merge records]
whoami   ──>  GET    /api/cases/whoami                Authorization: Bearer <PASETO v4.public>
                                                                          -> verified claims (401 without a token)
audit    ──>  GET    /api/cases/audit/recent · /api/cases/{pid}/audit     -> [audit rows]
disclose ──>  GET    /api/cases/{pid}/audit/disclosures                   -> HIPAA §164.528 accounting (record-level gated)
verify   ──>  GET    /api/cases/audit/verify · /api/cases/records/verify  -> chain / row-integrity verification report
checkpt  ──>  GET    /api/cases/checkpoint · POST /api/cases/checkpoint/verify -> external-witness checkpoint take/check
links    ──>  POST/GET /api/cases/{pid}/links · DELETE .../{id}           -> subject_of case→person edges (§8.6)
links?   ──>  GET    /api/cases/links                                     -> bulk edge pull (privileged, audited)
events   ──>  GET    /api/cases/events/recent                             -> [{kind, pid, name, seq}]
import   ──>  POST   /api/cases/import                multipart JSONL/CSV -> 202 {job_id}
import?  ──>  GET    /api/cases/import/{id}                               -> job status + counts + errors_url
export?  ──>  POST   /api/cases/export                {q?, format?, masking_profile?, …} -> 202 {job_id}
export!  ──>  GET    /api/cases/export/{id}                               -> job status + download_url
jobs     ──>  GET    /api/cases/bulk-jobs                                 -> [bulk job status]
fhir     ──>  GET/POST/PUT/DELETE /fhir/Task{,/{id}} · GET /fhir/metadata -> FHIR R5 Task CRUD/search (best-effort)
compliance ─> GET    /api/compliance · /api/compliance/sbom               -> identification + CycloneDX SBOM
docs     ──>  GET    /api-docs/openapi.json · /swagger-ui                 -> OpenAPI 3 + Swagger UI
metrics  ──>  GET    /metrics.prom                                        -> Prometheus text (root-mounted, public)
```

### Bulk import / export

Async, job-based (BLK-5; `agents/share/bulk-import-export.md`; spec
§8.7). JSONL (lossless reference) or CSV; the stable upsert key is the
agency-scoped `(agency_id, case_number)` pair, then the row's own `pid`
— a keyless row runs the same duplicate-detection path
`check-duplicates` uses and, on a likely match, is still created **and**
queued in the review queue (`provenance = "import"`). Export defaults to
the masked view (`mask_case`); the unmasked `full` profile requires
elevated authorisation, and every export is audited before its
`download_url` is ever surfaced.

```text
POST /api/cases/import  (multipart: file, format=jsonl|csv, dry_run?)
  -> 202 { "job_id": "…" }

GET /api/cases/import/{job_id}
  -> { id, kind: "import", status, rows_total, rows_created, rows_upserted,
       rows_to_review, rows_errored, errors_url }

POST /api/cases/export
  { "format": "jsonl", "q": "housing", "masking_profile": "masked" }
  -> 202 { "job_id": "…" }

GET /api/cases/export/{job_id}
  -> { id, kind: "export", status, rows_total, download_url }
```

### Merge request / response

A merge folds a confirmed-duplicate case into a surviving (main) case:
list fields union, the duplicate's title becomes a former
`alternate_titles` entry, the duplicate is soft-deleted, a
`merge_records` row is written, and a `Merged` event (plus a `Deleted`
for the duplicate) is published. The merge writes two audit rows —
`merged` on the survivor and `merged_into` on the duplicate (spec §6.8).

```text
POST /api/cases/merge
  { "main_pid": "…", "duplicate_pid": "…", "reason": "confirmed duplicate" }

200
  { "main_pid": "…", "duplicate_pid": "…", "main": { …merged Case… } }

422  when main_pid == duplicate_pid
404  when either pid is unknown
```
