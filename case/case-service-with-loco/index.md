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
search   ──>  GET    /api/cases/search?q=housing                          -> [{pid, title}]
update   ──>  PUT    /api/cases/{pid}                 {Case}              -> {pid, title}
delete   ──>  DELETE /api/cases/{pid}                                     -> {}
match    ──>  POST   /api/cases/match                 {query, candidates} -> ranked results
dedupe   ──>  POST   /api/cases/check-duplicates      {Case query}        -> [{pid, title, score, confidence, is_match}]
merge    ──>  POST   /api/cases/merge                 {main_pid, duplicate_pid, reason?}
                                                                          -> {main_pid, duplicate_pid, main}
merges   ──>  GET    /api/cases/merges/recent                             -> [merge records]
whoami   ──>  GET    /api/cases/whoami                Authorization: Bearer <PASETO v4.public>
                                                                          -> verified claims (401 without a token)
audit    ──>  GET    /api/cases/audit/recent · /api/cases/{pid}/audit     -> [audit rows]
events   ──>  GET    /api/cases/events/recent                             -> [{kind, pid, name, seq}]
docs     ──>  GET    /api-docs/openapi.json · /swagger-ui                 -> OpenAPI 3 + Swagger UI
metrics  ──>  GET    /metrics.prom                                        -> Prometheus text (root-mounted, public)
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
