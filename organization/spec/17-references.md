## 17. References

### Subprojects (this entity)

- Service: [spec](../organization-service-rust-crate/spec/index.md) ·
  [README](../organization-service-rust-crate/README.md) ·
  [AGENTS.md](../organization-service-rust-crate/AGENTS.md) ·
  [index](../organization-service-rust-crate/index.md) ·
  [CHANGELOG](../organization-service-rust-crate/CHANGELOG.md)
- Matcher: [spec](../organization-matcher-rust-crate/spec/index.md) ·
  [README](../organization-matcher-rust-crate/README.md) ·
  [AGENTS.md](../organization-matcher-rust-crate/AGENTS.md) ·
  [AGENTS/](../organization-matcher-rust-crate/AGENTS/index.md) ·
  [CHANGELOG](../organization-matcher-rust-crate/CHANGELOG.md)
- Front-end: [spec](../organization-front-end-with-svelte/spec/index.md) ·
  [README](../organization-front-end-with-svelte/README.md) ·
  [AGENTS.md](../organization-front-end-with-svelte/AGENTS.md) ·
  [CHANGELOG](../organization-front-end-with-svelte/CHANGELOG.md)

### Sibling entities

- [Person entity spec](../../person/spec/index.md) — the entity-level
  exemplar; its service is the mature feature reference
  (match-search-merge, privacy, FHIR).
- [Authentication entity](../../authentication/) — the central SSO
  provider this entity will verify JWTs against (§15).
- Care-pathway entity — the other trio built on the same
  matcher-type-as-DTO recipe.

### Shared project docs

- [`AGENTS.md`](../../AGENTS.md) (root) and
  [`agents/share/index.md`](../../agents/share/index.md) — full
  shared-doc directory.
- [`agents/share/overview.md`](../../agents/share/overview.md),
  [`architecture.md`](../../agents/share/architecture.md),
  [`match-search-merge.md`](../../agents/share/match-search-merge.md),
  [`auditability.md`](../../agents/share/auditability.md),
  [`privacy.md`](../../agents/share/privacy.md),
  [`availability.md`](../../agents/share/availability.md),
  [`locales.md`](../../agents/share/locales.md),
  [`compliance-for-technology.md`](../../agents/share/compliance-for-technology.md),
  [`postgresql.md`](../../agents/share/postgresql.md),
  [`rust-loco-stack.md`](../../agents/share/rust-loco-stack.md).

### External

- [schema.org/Organization](https://schema.org/Organization) — the
  domain model's vocabulary.
- Identifier registries: [GLEIF](https://www.gleif.org/) (LEI),
  D&B DUNS, [GS1 GLN](https://www.gs1.org/standards/id-keys/gln),
  [ROR](https://ror.org/), [ISNI](https://isni.org/),
  [Wikidata](https://www.wikidata.org/), ISO 6523, EU VAT.
- [loco.rs](https://loco.rs/) — the service framework.
- [SvelteKit](https://kit.svelte.dev/) + Svelte 5 runes — the
  front-end stack.
