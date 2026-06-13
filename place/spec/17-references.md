## 17. References

### Subproject documentation

| Subproject | Spec | AGENTS | Index |
|---|---|---|---|
| place-service-rust-crate | [spec](../place-service-rust-crate/spec/index.md) | [AGENTS/](../place-service-rust-crate/AGENTS/index.md) | [index](../place-service-rust-crate/index.md) |
| place-matcher-rust-crate | [spec](../place-matcher-rust-crate/spec/index.md) | [AGENTS.md](../place-matcher-rust-crate/AGENTS.md) | [index](../place-matcher-rust-crate/index.md) |
| place-front-end-with-svelte | [spec](../place-front-end-with-svelte/spec/index.md) | [AGENTS.md](../place-front-end-with-svelte/AGENTS.md) | [index](../place-front-end-with-svelte/index.md) |

### Shared project docs

- [`agents/share/overview.md`](../../agents/share/overview.md) — Main X Index family overview
- [`agents/share/architecture.md`](../../agents/share/architecture.md) — layered service architecture
- [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md) — duplicate-handling workflows
- [`agents/share/postgresql.md`](../../agents/share/postgresql.md) — PostgreSQL + extensions (incl. PostGIS)
- [`agents/share/locales.md`](../../agents/share/locales.md) — supported locales
- [`agents/share/privacy.md`](../../agents/share/privacy.md) — masking, GDPR, consent
- [`agents/share/auditability.md`](../../agents/share/auditability.md) — audit + events
- [`agents/share/availability.md`](../../agents/share/availability.md) — scaling, health checks
- [`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md) — GDPR, UK DPA, ISO 27001 / 42001
- [`agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md) — dependency stack

### External standards

- [schema.org/Place](https://schema.org/Place) — domain model basis
- [schema.org/PostalAddress](https://schema.org/PostalAddress), [schema.org/GeoCoordinates](https://schema.org/GeoCoordinates)
- GS1 General Specifications — Global Location Number (GLN) + check digit
- ISO 3166-1 alpha-2 — country codes (matcher `country_code` field)
- WGS 84 — coordinate reference system
- RFC 2119 / RFC 8174 — normative keywords (used by the matcher spec and §5–§6 here)
- EU GDPR, UK DPA 2018, ISO/IEC 27001, ISO/IEC 42001 — compliance frameworks ([§12](12-compliance.md))
