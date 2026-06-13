# Organization Service — documentation index

A loco.rs registry of organization identities (schema.org/Organization):
CRUD + matching, embedding the canonical organization-matcher.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | How to work here; API surface; MVP scope. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
create   ──>  POST /api/organizations          {Organization}      -> {pid, name}
read     ──>  GET  /api/organizations/{pid}                         -> Organization
dedupe   ──>  POST /api/organizations/check-duplicates  {query}     -> [{pid, score, ...}]
match    ──>  POST /api/organizations/match   {query, candidates}   -> ranked results
```

The `Organization` body shape is the `organization-matcher` type,
serialized snake_case (`name`, `legal_name`, `identifiers`
(LEI/DUNS/…), `url`, `same_as`, `address`, `jurisdiction`,
`founding_date`, `keywords`) — not schema.org's camelCase (entity
spec OQ-1, resolved).
