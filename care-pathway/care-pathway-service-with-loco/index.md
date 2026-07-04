# Care Pathway Service — documentation index

A loco.rs registry of clinical care-pathway records: CRUD + matching,
embedding the canonical care-pathway-matcher.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | How to work here; API surface; MVP scope. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
create   ──>  POST   /api/care-pathways                {CarePathway}      -> {pid, name}
read     ──>  GET    /api/care-pathways/{pid}                             -> CarePathway
update   ──>  PUT    /api/care-pathways/{pid}          {CarePathway}      -> {pid, name}
delete   ──>  DELETE /api/care-pathways/{pid}                             -> 204
list     ──>  GET    /api/care-pathways                                   -> [{pid, name}]  (cap 100)
search   ──>  GET    /api/care-pathways/search?q=stroke                   -> [{pid, name}]  (ILIKE, cap 50)
dedupe   ──>  POST   /api/care-pathways/check-duplicates  {query}         -> [{pid, score, ...}]
match    ──>  POST   /api/care-pathways/match   {query, candidates}       -> ranked results
merge    ──>  POST   /api/care-pathways/merge   {main_pid, duplicate_pid}  -> merge record
merges   ──>  GET    /api/care-pathways/merges/recent                     -> [merge record]
audit    ──>  GET    /api/care-pathways/audit/recent  ·  /{pid}/audit     -> [audit row]
events   ──>  GET    /api/care-pathways/events/recent                     -> [{kind, pid, name, seq}]
whoami   ──>  GET    /api/care-pathways/whoami          (Bearer PASETO)   -> verified claims (401 without)
docs     ──>  GET    /api-docs/openapi.json  ·  /swagger-ui               -> OpenAPI 3 + Swagger UI
metrics  ──>  GET    /metrics.prom                                        -> Prometheus text (public)
```

The `CarePathway` body shape is the `care-pathway-matcher` type
(name, pathway code, provider, care setting, condition codes (ICD/SNOMED),
interventions, keywords, identifiers, sameAs).

A create/update payload is validated (blank `name`,
ICD/SNOMED/UUID/DOI/BCP-47 shapes) and returns `422` with every problem
in one body — a single call with a blank `name`, a malformed condition
code, and a non-UUID `Uuid` identifier reports all three at once (pinned
by `validation::problems_aggregates_across_all_dimensions`). Each
create/update/delete/merge writes an `audit_logs` row, publishes a
`created`/`updated`/`deleted`/`merged` event, and bumps the matching
Prometheus counter.

Auth is optional per route by default: send `Authorization: Bearer
<paseto>` (a short-lived PASETO v4.public token minted by the
[authentication-service](../../authentication/authentication-service-with-loco),
verified offline against its published Ed25519 key) and `whoami` echoes
the verified claims (`401` without one), while every other handler stamps
the token `sub` as the audit `actor`. Flip `CARE_PATHWAY_REQUIRE_AUTH=1`
to require that bearer token on every `/api/*` route (`/metrics.prom`,
`/api-docs/openapi.json`, `/swagger-ui`, `/_health`, `/_ping` stay
public).

Auth pivot done in this crate (RS256 JWT + JWKS decommissioned in favour
of cookie sessions + PASETO v4.public; `src/auth.rs` embeds the
`authentication-verifier` crate). Source of truth:
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md).
