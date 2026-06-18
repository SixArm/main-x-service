## 17. References

### Subprojects (the trio)

- [event-service-with-loco](../event-service-with-loco/) —
  [spec](../event-service-with-loco/spec/index.md) ·
  [AGENTS](../event-service-with-loco/AGENTS/index.md) ·
  [README](../event-service-with-loco/README.md)
- [event-matcher-rust-crate](../event-matcher-rust-crate/) —
  [spec](../event-matcher-rust-crate/spec/index.md) ·
  [AGENTS.md](../event-matcher-rust-crate/AGENTS.md) ·
  [README](../event-matcher-rust-crate/README.md)
- [event-front-end-with-svelte](../event-front-end-with-svelte/) —
  [spec](../event-front-end-with-svelte/spec/index.md) ·
  [AGENTS.md](../event-front-end-with-svelte/AGENTS.md) ·
  [README](../event-front-end-with-svelte/README.md)

### Entity-level

- [`AGENTS/index.md`](../AGENTS/index.md) — entity agent reference set.
- [`AGENTS/spec-driven-development.md`](../AGENTS/spec-driven-development.md) — SDD discipline + authority model.
- [`../event-service-schema.sql`](../event-service-schema.sql) — schema snapshot (§10.1).

### Sibling entities

- [authentication](../../authentication/) — SSO provider (§8.4).
- [person](../../person/), [place](../../place/),
  [organization](../../organization/), [worker](../../worker/) —
  targets of `Party.id` / `Place.id` external references.
- [person/spec](../../person/spec/index.md) — the first entity-level
  spec; this one follows its shape.

### Shared project docs

- [`AGENTS.md`](../../AGENTS.md) — project root.
- [`agents/share/index.md`](../../agents/share/index.md) — shared
  reference set (architecture, dataflow, match/search/merge,
  privacy, auditability, availability, observability, locales,
  compliance, PostgreSQL, REST).

### External

- [schema.org/Event](https://schema.org/Event) — domain alignment.
- RFC 5545 (iCalendar / RRULE) — roadmap recurrence.
- ISO 8601 (durations, dates), ISO 639-1 (languages), ISO 4217
  (currencies), IANA tz database.
- GDPR, UK DPA 2018, ISO/IEC 27001, ISO/IEC 42001 — §12.
