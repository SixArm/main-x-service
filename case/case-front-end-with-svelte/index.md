# case-front-end-with-svelte — documentation index

Operator UI for case CRUD + matching, consuming the
[Case Service](../case-service-rust-crate).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Flow

```text
/         ──>  GET  /api/cases                  list
/new      ──>  POST /api/cases  {Case}          create -> /[pid]
/[pid]    ──>  GET  /api/cases/{pid}            detail
              POST /api/cases/check-duplicates   -> scored matches
              DELETE /api/cases/{pid}             soft-delete
/[pid]/edit ─> PUT  /api/cases/{pid}             edit
```
