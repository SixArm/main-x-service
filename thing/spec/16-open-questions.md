## 16. Open Questions

Entity-level questions only — crate-internal OQs stay in the owning
crate's spec ([service §16](../thing-service-rust-crate/spec/16-open-questions.md),
[matcher §10](../thing-matcher-rust-crate/spec/10-open-questions.md)).

- **OQ-1 — One matching engine or two?** The service carries its own
  5-component scorer (`matching::scoring::compute_match`, weights
  0.40 / 0.30 / 0.10 / 0.10 / 0.10) *and* embeds the canonical
  10-component matcher engine via the adapter. Should the in-service
  scorer be retired so the embedded engine is the only scoring path,
  or kept as a cheap pre-filter? Interacts with service OQ-2
  (`ThingMatcher` trait) and entity T-8.
- **OQ-2 — Confidence vocabulary.** Service: Certain / Probable /
  Possible / Unlikely (0.95 / 0.80 / 0.60, configurable). Matcher:
  High / Medium / Low (0.90 / 0.75, fixed). Which is API-facing, and
  where is the mapping defined — adapter or handler? (Entity T-8.)
- **OQ-3 — Where do bulk-import pipelines live?** Wikidata /
  OpenLibrary / agency feeds (§15.5): inside the service as
  background jobs (`bg_pg`), or a separate ETL subproject in the
  entity directory?
- **OQ-4 — Entity spec vs repo-root spec.** A repo-root
  [`spec/`](../../spec/) exists (data modelling). Should entity-level
  schema files such as
  [`thing-service-schema.sql`](../thing-service-schema.sql) be owned
  by this spec's §10 or by the root data-modelling spec?
- **OQ-5 — Front-end auth posture during the SSO gap.** Until entity
  T-6 lands, the operator UI is unauthenticated. Is a deployment
  control (network allowlist / reverse-proxy auth) sufficient interim
  mitigation for governmental pilots, and should this spec mandate it?
