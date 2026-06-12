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
create   ──>  POST /api/care-pathways              {CarePathway}     -> {pid, name}
read     ──>  GET  /api/care-pathways/{pid}                          -> CarePathway
dedupe   ──>  POST /api/care-pathways/check-duplicates  {query}      -> [{pid, score, ...}]
match    ──>  POST /api/care-pathways/match   {query, candidates}    -> ranked results
```

The `CarePathway` body shape is the `care-pathway-matcher` type
(name, pathway code, provider, care setting, condition codes (ICD/SNOMED),
interventions, keywords, identifiers, sameAs).
