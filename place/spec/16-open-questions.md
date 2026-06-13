## 16. Open Questions

Entity-level questions. Crate-internal questions stay in the owning
crate's spec (service §16, matcher §10, front-end §16).

- **OQ-1 — Data residency for multi-region replication.** A worldwide
  governmental deployment replicates across jurisdictions; GDPR /
  UK DPA constrain where residence-linked place records may rest.
  Single write region with per-jurisdiction read replicas? Regional
  sharding by `address_country`? Blocks [§15.2](15-roadmap.md).
- **OQ-2 — Coordinate-masking precision policy.** The service rounds
  to 2 dp (~1 km) (service spec §16 OQ-1). Is one bucket right for a
  governmental registry, or do we need `coarse` / `medium` / `precise`
  tiers bound to operator roles once SSO lands (E-5)?
- **OQ-3 — Authoritative-source precedence in merges.** When a
  gazetteer-imported record and an operator-entered record merge,
  which side's fields win? Today merge is direction-explicit
  (main + duplicate) with no source-authority weighting.
- **OQ-4 — Front-end type generation.** Wire types are hand-mirrored
  (`src/lib/api/types.ts`); should they be generated from the
  service's OpenAPI document, or schema-checked in CI? Interacts with
  the accepted drift policy (no shared package).
- **OQ-5 — Cross-entity place ↔ organization reconciliation.** An
  organization's site (organization entity) and a place record can
  describe the same building. Is reconciliation an entity concern, an
  index-level concern, or out of scope?
- **OQ-6 — SVAR DataGrid licensing.** `wx-svelte-grid` free tier is
  GPL-3.0 — fitness for a government deployment needs a decision
  (front-end spec §16 OQ-1 / §13 T-21).
