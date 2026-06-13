## 16. Open Questions

Entity-level questions (cross-subproject). Per-subproject questions
stay in the subproject specs; those that affect the integration
contract are mirrored here.

- **OQ-2 — JSONB vs normalised tables.** Should identifiers and
  addresses move into their own tables once Tantivy search and
  register-feed ingestion land (deterministic-identifier lookup wants
  an index), or does the JSONB payload + external search index
  suffice? (Mirrors the service crate's §16.)
- **OQ-3 — Real-time duplicate detection on create.** Family
  convention is `409 Conflict` with candidate matches on `POST`; this
  entity currently only offers the explicit `/check-duplicates`
  endpoint, and the front-end runs the check from the detail page.
  Decide where the check belongs (service-enforced vs UI-advisory) —
  mirrored from the service and front-end §16s.
- **OQ-4 — URL domain as a deterministic pin.** An exact registered
  domain match is currently strong-but-probabilistic (0.15 weight)
  because parents / subsidiaries can share a domain. Should it ever
  pin? (Matcher spec §16; affects check-duplicates precision.)
- **OQ-5 — Sole-trader flagging.** Should records carry an explicit
  "natural-person-linked" marker so the privacy layer (T-5) can apply
  GDPR handling deterministically, rather than inferring it from
  legal form?

Open questions resolve into §13 tasks or §5–§9 amendments when
decisions are made.

### Resolved

- **OQ-1 — Wire naming: snake_case or schema.org camelCase?**
  *Resolved 2026-06-13: snake_case is canonical.* The DTO serialises
  snake_case (`legal_name`, `founding_date`) with no serde rename; the
  stored JSONB payloads, the front-end TS mirror
  (`src/lib/api/types.ts`), and the OpenAPI document all already use
  it, so adopting `#[serde(rename_all = "camelCase")]` would have
  broken every stored payload for JSON-LD interop this MVP does not
  need. schema.org's camelCase names remain a documentation mapping
  only (§5). Docs corrected to snake_case via T-3 (service `README.md`
  / `index.md` / `AGENTS.md`, front-end `README.md`).
