# care-pathway-front-end-with-svelte — documentation index

Operator UI for care-pathway CRUD + matching, consuming the
[Care Pathway Service](../care-pathway-service-rust-crate).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Flow

```text
/         ──>  GET  /api/care-pathways                  list
/new      ──>  POST /api/care-pathways  {CarePathway}   create -> /[pid]
/[pid]    ──>  GET  /api/care-pathways/{pid}            detail
              POST /api/care-pathways/check-duplicates   -> scored matches
              DELETE /api/care-pathways/{pid}             soft-delete
/[pid]/edit ─> PUT  /api/care-pathways/{pid}             edit
```
