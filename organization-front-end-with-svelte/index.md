# organization-front-end-with-svelte — documentation index

Operator UI for organization CRUD + matching, consuming the
[Organization Service](../organization-service-rust-crate).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Flow

```text
/         ──>  GET  /api/organizations                 list
/new      ──>  POST /api/organizations  {Organization} create -> /[pid]
/[pid]    ──>  GET  /api/organizations/{pid}           detail
              POST /api/organizations/check-duplicates  -> scored matches
              DELETE /api/organizations/{pid}            soft-delete
/[pid]/edit ─> PUT  /api/organizations/{pid}            edit
```
