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

```text
create   ──>  POST /api/cases                     {Case}            -> {pid, title}
read     ──>  GET  /api/cases/{pid}                                 -> Case
dedupe   ──>  POST /api/cases/check-duplicates  {query}            -> [{pid, score, ...}]
match    ──>  POST /api/cases/match   {query, candidates}          -> ranked results
```

The `Case` body shape is the `case-matcher` type (title, alternate
titles, case number, agency, case type, status, priority, opened date,
subjects, keywords, identifiers, sameAs, languages).
