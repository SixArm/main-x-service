## 16. Open Questions

Entity-level questions only. Crate-internal OQs live in
[service §16](../worker-service-with-loco/spec/16-open-questions.md),
[matcher §22](../worker-matcher-rust-crate/spec/22-open-questions-and-risks.md),
[front-end §16](../worker-front-end-with-svelte/spec/16-open-questions.md).
Resolved questions move into the relevant section and are deleted
here.

- **OQ-1 — Per-jurisdiction masking policy.** Licence numbers (NPI,
  board licence) are public record in some jurisdictions and
  sensitive personal data in others. Today masking defaults are
  global (SSN, tax ID, DEA, home address). Does a worldwide
  governmental deployment need per-jurisdiction masking profiles, and
  if so, where does the policy live — service config, or a
  deployment-side concern?
- **OQ-2 — GDPR erasure vs. audit retention.** Soft delete satisfies
  the registry's audit posture but is not physical erasure. What is
  the retention schedule, and what does a defensible Art. 17 response
  look like for a record that participated in a merge (its data was
  snapshotted into another record's merge history)?
- **OQ-3 — Schema snapshot authority.** Is the entity-root
  [`worker-service-schema.sql`](../worker-service-schema.sql) meant
  to be generated (from migrations) or hand-maintained? §10.1 assumes
  generated; §13 T-6 implements that assumption — confirm or correct.
- **OQ-4 — Two matchers, one truth.** The service still ships its own
  in-service matcher alongside the embedded canonical matcher
  (§5.3). Which scores which API responses today, and is the long-term
  intent to retire the in-service implementation in favour of the
  adapter path? The answer changes where tuning work should land.
- **OQ-5 — Worker↔person identity linkage.** A professional is also a
  person in the [person entity](../../person/). Is there a planned
  cross-entity link (shared identifier, federation key), or do the
  registries stay deliberately unlinked for privacy reasons?
- **OQ-6 — Event-bus choice for government infrastructure.** Service
  roadmap names Fluvio; multi-region governmental deployments more
  commonly mandate Kafka or NATS. Decide before consumers proliferate
  (§15.2).
