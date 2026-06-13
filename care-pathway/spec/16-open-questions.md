## 16. Open Questions

Open questions resolve into §13 tasks or §5–§12 amendments when
decisions are made. Matcher-internal questions live in the matcher
spec §16.

- ~~**OQ-1 — Validation status code.**~~ **Resolved 2026-06-13
  (T-2): `422` is normative**, matching the family convention
  (person/place services) and the service crate spec §6. loco 0.16
  has no `unprocessable_entity` helper, so the controller returns
  `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)` from a
  shared `validate()` used by create and update. `400` remains for
  malformed bodies (loco JSON rejection). See §9.1 and §13 T-2.
- **OQ-2 — Duplicate-check scale strategy.** `check-duplicates`
  scans at most 1 000 stored rows in memory. At national volumes:
  search-based blocking (Tantivy), JSONB GIN pre-filtering on
  condition codes, or both? (Feeds T-6.)
- **OQ-3 — Normalising condition codes.** Keep the pure-JSONB model,
  or break `condition_codes` / `interventions` into side tables once
  search and code-level queries land? (Crate spec carries the same
  question.)
- **OQ-4 — Real-time duplicate detection on create.** Mature
  siblings return `409 Conflict` with candidates on create; today
  duplicate checking is explicit-only. Adopt the `409` behaviour, or
  keep create cheap and rely on the front-end calling
  `check-duplicates` first?
- **OQ-5 — Cross-language duplicates.** The same NICE-style pathway
  translated into another language scores low on name similarity.
  Rely solely on deterministic identifiers / `same_as`, or add a
  translation-aware name component (roadmap localization work)?
- **OQ-6 — Provider identity.** `provider_id` is a free string.
  Should it reference the
  [organization entity](../../organization/) (`pid`) so provider
  scoping survives organisation renames and merges?
