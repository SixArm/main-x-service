## 10. Persistence

Exactly one subproject persists data: the service. This is an
entity-level invariant.

### 10.1 Service (system of record)

- **PostgreSQL 18** via SeaORM with migrations: 12+ tables —
  `workers`, `worker_names`, `worker_identifiers`,
  `worker_addresses`, `worker_contacts`, `worker_links`,
  `worker_match_scores`, organization tables, and the
  `audit_log` trail. Table inventory:
  [service `agents/models.md`](../worker-service-with-loco/agents/models.md);
  detail: [service §10](../worker-service-with-loco/spec/10-persistence.md).
- **Tantivy search index** on local disk (`SEARCH_INDEX_PATH`),
  synchronised with database writes. Single-node constraint; see
  roadmap §15 for externalisation.
- **Soft delete only.** No physical deletes — required by §12.
- Extensions and conventions per
  [`agents/share/postgresql.md`](../../agents/share/postgresql.md).

#### 10.1.1 Entity-root schema snapshot (`worker-service-schema.sql`)

[`worker-service-schema.sql`](../worker-service-schema.sql) (369
lines, 23 `CREATE TABLE`s) is a **hand-written, point-in-time
reading aid** at the entity root: a fully-normalised PostgreSQL
rendition of the service schema (JSONB kept only for audit snapshots
and match-score breakdowns; word lists as `TEXT[]`). The service
crate's migrations (`migrations/*/up.sql` + the SeaORM `migration/`
crate) are **authoritative**; the snapshot is not executed, not
generated, and not checked by anything.

**No regeneration story exists** (verified 2026-06-13: no script,
Make target, or CI step references the file), and it has already
drifted from the migrations:

- Missing from the snapshot (created by migrations):
  `postcode_geography`, `geography_name_references`,
  `ods_role_references`, `ods_relationship_references`,
  `ods_record_class_references`, `ods_record_use_type_references`,
  `practitioner_role_references`.
- In the snapshot only (no migration creates it): `worker_consents`.

§13 T-6 (documented regeneration command + drift check) and §16 OQ-3
(generated vs. hand-maintained authority) remain **open**; this
subsection records the current honest state until they are resolved.

### 10.2 Matcher (no persistence — by design)

The matcher is a pure library: no filesystem, no network, no global
state, no clocks. Persisting anything in the matcher is a spec
violation ([matcher NFR-6](../worker-matcher-rust-crate/spec/07-non-functional-requirements.md)).

### 10.3 Front-end (no server persistence)

The SPA holds transient page state only; the service is the system of
record for every fact it displays. See
[front-end §10](../worker-front-end-with-svelte/spec/10-persistence.md).

### 10.4 Event stream

Event publishing is currently the service's `InMemoryEventPublisher`
— events do not survive a restart and have no cross-node delivery.
Durable streaming (Fluvio production publisher, service §13 T-2;
Kafka/NATS evaluation) is roadmap §15. Until then, the **audit log is
the only durable record of change history** — another reason §12
treats it as load-bearing.
