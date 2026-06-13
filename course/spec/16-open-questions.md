## 16. Open Questions

Entity-level questions. Crate-internal questions stay in the crate
specs (service [§16](../course-service-rust-crate/spec/16-open-questions.md),
front-end [§16](../course-front-end-with-svelte/spec/16-open-questions.md)).

- **OQ-1**: SVAR DataGrid is GPL-3.0 in its free tier. A worldwide
  governmental deployment is exactly the case where the licensing
  decision (Pro / Enterprise vs GPL compliance vs replacement grid)
  must be settled before production. Mirrors front-end OQ-1; raised
  to entity level because it gates every deployment plan in §15.
- **OQ-2**: Should `CourseInstance` reference the
  [event entity's](../../event/) `Event` resource rather than carry
  its own inline `Schedule`? Mirrors service OQ-1 — deferred until a
  real cross-entity integration requirement appears; inline keeps the
  front-end join-free for MVP.
- **OQ-3**: Should `Provider` link to the
  [organization entity](../../organization/) registry? The service's
  OQ-2 deferred this "until two consumers ask"; the organization
  entity now exists as a real registry, so the question is live:
  keep the inline `providers` table, or hold an `organization_id`
  cross-reference (or both, with the inline record as a cache)?
- **OQ-4**: Who owns `../course-service-schema.sql` (entity-directory
  level)? The service crate's `migrations/` are the schema source of
  truth; an unowned sibling SQL dump will drift. Delete it, generate
  it from migrations, or document it as a snapshot artefact.
  *Update 2026-06-13:* inspected and documented in §10.3 — it is a
  hand-written normalization design document (not a dump), with no
  regeneration story, and it has already drifted from the live
  schema in two table names. The delete / generate / adopt decision
  remains open.
- **OQ-5**: Erasure depth for instructor personal data: is
  soft-delete + masking sufficient for GDPR erasure requests against
  `instructor_names` in historical instances and merge snapshots, or
  does a governmental deployment need field-level hard purge /
  crypto-shredding? Interacts with the audit-trail retention duty
  (§12.3).
- **OQ-6**: Locale strategy for matching: course names in different
  scripts ("Введение в программирование" vs "Introduction to
  Programming") will never match probabilistically — should
  cross-language identity rely purely on deterministic identifiers
  (DOI / Wikidata / `same_as`), or does the matcher need a
  translation-aware component? Today the answer is implicitly
  "identifiers only"; make it explicit before E-5.
