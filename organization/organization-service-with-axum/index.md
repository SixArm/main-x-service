# Organization Service — documentation index

A loco.rs registry of organization identities (schema.org/Organization):
CRUD + matching, embedding the canonical organization-matcher.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [../spec/index.md](../spec/index.md) | Entity umbrella spec — cross-subproject contract + `R-DUP`/`T-N` IDs. |
| [AGENTS.md](./AGENTS.md) | How to work here; API surface; MVP scope. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
create   ──>  POST /api/organizations          {Organization}      -> {pid, name}
read     ──>  GET  /api/organizations/{pid}                         -> Organization
search   ──>  GET  /api/organizations/search?q=acme                 -> [{pid, name}]
dedupe   ──>  POST /api/organizations/check-duplicates  {query}     -> [{pid, score, is_match}]
scan     ──>  POST /api/organizations/deduplicate                   -> stored review-queue candidates
queue    ──>  GET  /api/organizations/review-queue?status=pending   -> [review-queue items]
decide   ──>  POST /api/organizations/review-queue/{id}/decision  {status: confirmed|rejected}
                                                                    -> decided item (first-writer-wins)
match    ──>  POST /api/organizations/match   {query, candidates}   -> ranked results
merge    ──>  POST /api/organizations/merge   {main_pid, duplicate_pid, reason}
                                                                    -> {main_pid, duplicate_pid, main}
merges   ──>  GET  /api/organizations/merges/recent                 -> [merge-history rows]
audit    ──>  GET  /api/organizations/audit/recent | /{pid}/audit   -> [audit rows]
events   ──>  GET  /api/organizations/events/recent                 -> [{kind, pid, name, seq}]
whoami   ──>  GET  /api/organizations/whoami    (Bearer <paseto>)   -> verified claims (401 without)
metrics  ──>  GET  /metrics.prom                                    -> Prometheus text exposition
import   ──>  POST /api/organizations/import  (multipart: file, format, dry_run)
                                                                    -> 202 {job_id}
             GET  /api/organizations/import/{id}                   -> job status + counts + errors_url
export   ──>  POST /api/organizations/export  {format, q, masking_profile, ...}
                                                                    -> 202 {job_id}
             GET  /api/organizations/export/{id}                   -> job status + download_url
bulk-jobs──>  GET  /api/organizations/bulk-jobs                    -> [recent bulk jobs]
fhir     ──>  GET  /fhir/Organization/{id}                          -> FHIR R5 Organization
                                                                       (family reference implementation)
             GET  /fhir/metadata                                    -> CapabilityStatement
```

Every `/api/*` request is negotiated by the `Accepts-version` header
(default `1.0`, no version in the URL) — see
[`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

A worked **merge** call (fold a confirmed duplicate into a survivor):

```bash
curl -s localhost:5150/api/organizations/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<dup-uuid>","reason":"confirmed"}'
# -> {"main_pid":"...","duplicate_pid":"...","main":{ ...merged Organization,
#      alternate_names now include the duplicate's former name... }}
```

The `Organization` body shape is the `organization-matcher` type,
serialized snake_case (`name`, `legal_name`, `identifiers`
(LEI/DUNS/…), `url`, `same_as`, `address`, `jurisdiction`,
`founding_date`, `keywords`) — not schema.org's camelCase (entity
spec OQ-1, resolved).

The `/whoami` bearer is a short-lived PASETO v4.public token verified
offline against the authentication-service's published Ed25519 key
(RS256/JWKS decommissioned); `src/auth.rs` embeds the
`authentication-verifier` crate. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth).
