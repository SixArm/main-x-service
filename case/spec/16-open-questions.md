## 16. Open Questions

Open questions resolve into §13 tasks or §5–§12 amendments when
decisions are made. Matcher-internal questions live in the matcher
spec §16.

- **OQ-1 — Privacy controls before production?** Case data is personal
  data (§12), yet masking / GDPR export are not built (T-10). Is the
  registry deployable into a real governmental environment on access
  control + DPIA alone, or must T-10 land first? (Drives T-10
  priority.)
- **OQ-2 — Duplicate-check scale strategy.** `check-duplicates` scans
  at most `CHECK_DUPLICATES_SCAN_CAP` (1 000) stored rows in memory. At
  population volumes: search-based blocking (Tantivy), JSONB GIN
  pre-filtering on `subjects`, or both? (Feeds T-6.)
- **OQ-3 — Normalising subjects.** Keep the pure-JSONB model, or break
  `subjects` / `keywords` into side tables once search and subject-level
  queries land? (Crate spec carries the same question.)
- **OQ-4 — Real-time duplicate detection on create.** Mature siblings
  return `409 Conflict` with candidates on create; today duplicate
  checking is explicit-only. Adopt the `409` behaviour, or keep create
  cheap and rely on the front-end calling `check-duplicates` first?
- **OQ-5 — Subject identity.** `subjects` and `agency_id` are free
  strings. Should `subjects` reference the [person entity](../../person/)
  / [organization entity](../../organization/) `pid`, and `agency_id`
  the organization entity `pid`, so scoping survives renames and merges?
- **OQ-6 — Status / type taxonomy.** The `CaseType` / `CaseStatus`
  enums are a pragmatic cross-agency set with `Custom` escape hatches.
  Do specific jurisdictions need richer, namespaced taxonomies (and
  should `Custom` values be matched at all)?
- **OQ-7 — Cross-language duplicates.** The same matter titled in
  another language scores low on title similarity. Rely solely on
  deterministic identifiers / `same_as` / `subjects`, or add a
  translation-aware title component (roadmap localization work)?
