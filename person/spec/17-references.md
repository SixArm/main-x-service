## 17. References

### Subproject documentation

- Service: [spec](../person-service-with-loco/spec/index.md) ·
  [AGENTS](../person-service-with-loco/AGENTS/index.md) ·
  [README](../person-service-with-loco/README.md) ·
  [index](../person-service-with-loco/index.md)
- Matcher: [spec](../person-matcher-rust-crate/spec/index.md) ·
  [AGENTS.md](../person-matcher-rust-crate/AGENTS.md) ·
  [README](../person-matcher-rust-crate/README.md) ·
  [index](../person-matcher-rust-crate/index.md)
- Front-end: [spec](../person-front-end-with-svelte/spec/index.md) ·
  [AGENTS.md](../person-front-end-with-svelte/AGENTS.md) ·
  [README](../person-front-end-with-svelte/README.md) ·
  [CHANGELOG](../person-front-end-with-svelte/CHANGELOG.md)

### Entity-level

- [`AGENTS/index.md`](../AGENTS/index.md) — entity agent reference set.
- [`person-service-schema.sql`](../person-service-schema.sql) —
  reference schema dump.

### Project root

- [`AGENTS.md`](../../AGENTS.md) — monorepo guide and subproject
  directory.
- [`agents/share/index.md`](../../agents/share/index.md) — shared
  reference docs (architecture, dataflow, match/search/merge, privacy,
  auditability, availability, observability, locales, compliance,
  PostgreSQL, stack).

### Sibling entities

- [authentication](../../authentication/) — SSO provider (magic-link,
  RS256 JWT + JWKS) the person entity will verify against (E-1).
- [worker](../../worker/) — workforce refinement of person identity
  (EOQ-4).
- [organization](../../organization/) — schema.org/Organization
  registry; relevant to service §16 OQ-1.

### External standards

- HL7 FHIR R5 Person resource — the shape of the service model.
- schema.org/Person — vocabulary alignment across the index.
- EU/UK GDPR, UK DPA 2018, HIPAA, ISO/IEC 27001, ISO/IEC 42001:2023 —
  see §12.
- ISO 3166-1 (country codes), E.164 (phone numbers), ISO 639-1
  (locale codes per [agents/share/locales.md](../../agents/share/locales.md)).
