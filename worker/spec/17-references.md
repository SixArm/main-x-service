## 17. References

### Subproject canonical documents

| Subproject | Spec | AGENTS | Intro |
|---|---|---|---|
| worker-service-with-loco | [spec](../worker-service-with-loco/spec/index.md) (§1–§18) | [AGENTS/](../worker-service-with-loco/AGENTS/index.md) | [README](../worker-service-with-loco/README.md) · [index](../worker-service-with-loco/index.md) |
| worker-matcher-rust-crate | [spec](../worker-matcher-rust-crate/spec/index.md) (§1–§25) | [AGENTS.md](../worker-matcher-rust-crate/AGENTS.md) | [README](../worker-matcher-rust-crate/README.md) · [index](../worker-matcher-rust-crate/index.md) |
| worker-front-end-with-svelte | [spec](../worker-front-end-with-svelte/spec/index.md) (§1–§18) | [AGENTS.md](../worker-front-end-with-svelte/AGENTS.md) | [README](../worker-front-end-with-svelte/README.md) |

### Entity-level reference docs

[`../AGENTS/`](../AGENTS/index.md) — subproject map, model shapes,
matching layers, REST summary, testing seams.

### Project-root shared docs

- [`AGENTS.md`](../../AGENTS.md) — monorepo entry point and crate index
- [`agents/share/overview.md`](../../agents/share/overview.md) — project overview
- [`agents/share/architecture.md`](../../agents/share/architecture.md) — layered architecture
- [`agents/share/dataflow.md`](../../agents/share/dataflow.md) — create / merge / search flows
- [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md) — duplicate-handling workflows
- [`agents/share/privacy.md`](../../agents/share/privacy.md), [`auditability.md`](../../agents/share/auditability.md), [`availability.md`](../../agents/share/availability.md), [`observability.md`](../../agents/share/observability.md)
- [`agents/share/locales.md`](../../agents/share/locales.md) — locale catalogue
- [`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md), [`compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
- [`agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md), [`postgresql.md`](../../agents/share/postgresql.md), [`restful.md`](../../agents/share/restful.md)

### Sibling entities

- [authentication](../../authentication/) — SSO provider the trio
  will consume (§8.3)
- [person](../../person/) — general person registry (see §16 OQ-5)
- [organization](../../organization/) — organizations referenced by
  `managing_organization`

### External standards

- HL7 FHIR R5 Practitioner — the service's FHIR resource mapping
- ISO 3166-1 alpha-2 — passport-book and address country codes
- E.164 — phone normalisation target format
- RFC 2119 — MUST / SHOULD / MAY usage in these specs
