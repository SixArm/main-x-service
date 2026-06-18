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
match    ──>  POST /api/organizations/match   {query, candidates}   -> ranked results
merge    ──>  POST /api/organizations/merge   {main_pid, duplicate_pid, reason}
                                                                    -> {main_pid, duplicate_pid, main}
merges   ──>  GET  /api/organizations/merges/recent                 -> [merge-history rows]
audit    ──>  GET  /api/organizations/audit/recent | /{pid}/audit   -> [audit rows]
events   ──>  GET  /api/organizations/events/recent                 -> [{kind, pid, name, seq}]
whoami   ──>  GET  /api/organizations/whoami    (Bearer <paseto>)   -> verified claims (401 without)
metrics  ──>  GET  /metrics.prom                                    -> Prometheus text exposition
```

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
(RS256/JWKS decommissioned). Auth pivot in progress; see
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); code follow-up tracked in spec §13.
