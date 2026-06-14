# Auditability

Monorepo-wide reference for **auditability** across the **Main X Index**
family: the durable record of *who changed what, when*, and the
operational feed of *what just changed*. This is the comprehensive
spec; the short version lives at
[`agents/share/auditability.md`](../../agents/share/auditability.md).

Auditability is delivered by **two complementary mechanisms** that are
deliberately kept separate:

1. The **audit log** — a durable, append-only Postgres table (`audit_logs`)
   holding the who / what / when plus before/after JSON snapshots. It is
   the system of record for compliance.
2. The **event stream** — an operational change feed (today an in-memory
   ring buffer behind a publisher seam) that powers recent-activity views
   and, in the roadmap, downstream consumers.

Every service owns its own implementation — there is no shared audit
crate — but they all follow the conventions below. As with merge, the
family splits into two implementation lineages:

- **Loco lineage** (`organization`, `care-pathway`, `case`): the
  canonical shape this spec describes — an `audit_logs` table written by
  a controller `audit()` helper, plus a Phase-1 in-memory event stream
  (`src/streaming.rs`) behind the `EventPublisher` seam.
- **MPI lineage** (`person`, `place`, `thing`, `event`, `worker`,
  `course`): the older `AuditLogRepository` over an `audit_log` table
  (`src/db/audit.rs`) with typed `log_create` / `log_update` /
  `log_delete` helpers, plus an `EventProducer`-based stream under
  `src/streaming/`. Same intent, richer column set; see §2 and §3.

> Related monorepo topic specs that exist:
> [postgresql](../postgresql/index.md),
> [merge](../merge/index.md),
> [authentication](../authentication/index.md),
> [architecture](../architecture/index.md).
> The event-stream roadmap (durable outbox → Fluvio) is not yet a
> sibling spec; until it lands the live source is
> [`agents/share/event-bus.md`](../../agents/share/event-bus.md).
> Compliance is covered by
> [`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
> and
> [`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).

---

## 1. Two complementary mechanisms

The audit log and the event stream answer different questions and have
different durability guarantees. Keeping them separate is intentional.

| | Audit log | Event stream |
|---|---|---|
| Question answered | *Who changed what, when?* (forensic / compliance) | *What just changed?* (operational / reactive) |
| Durability | Durable — Postgres `audit_logs` table | Ephemeral — in-memory ring buffer (Phase 1) |
| Retention | Append-only, indefinite, soft-delete-friendly | Most-recent **1000** events per process |
| Payload | `action` + entity pid + before/after JSON snapshot + actor + (ip/user-agent in MPI lineage) | Versioned `Envelope` (`kind`, `pid`, `name`, `seq`, `actor`, `event_id`) |
| Read surface | `/audit/recent`, `/{pid}/audit`, `/account/audit` | `/events/recent` (projected to `EventView`) |
| Write rule | **Best-effort** — never fails the request | **Best-effort** — never fails the request; may drop on a poisoned lock |
| Source of truth | **Yes** — the durable record | No — the audit log is authoritative |

Both are written from the same place in each mutating handler: after the
database write succeeds, the handler records an audit row **and**
publishes an event, both stamped with the same `actor`. Neither is
allowed to fail the request — auditing is a side effect, not a gate.

---

## 2. Audit log (durable)

### 2.1 The `audit_logs` table (Loco lineage)

One row per CRUD/merge action. The migration
(`migration/src/..._audit_logs.rs`) defines a deliberately small column
set:

| Column | Type | Meaning |
|---|---|---|
| `id` | `PkAuto` | Surrogate primary key; also the newest-first ordering key. |
| `entity_pid` | `Uuid` | The public id of the record the entry concerns. |
| `action` | `String` | `created` / `updated` / `deleted` / `merged` / `merged_into`. |
| `actor` | `String NULL` | The caller's bearer-token `sub` (user pid) — `NULL` until a verified token is presented. |
| `snapshot` | `JsonBinary NULL` | Snapshot of the record payload at the time of the action (`None` for delete). |

The write/query helpers live on the model
(`src/models/audit_logs.rs`):

- `Model::record(db, entity_pid, action, actor, snapshot)` — insert one
  entry. **Best-effort**: callers log but never fail the request on an
  audit error.
- `Model::recent(db, limit)` — newest-first system-wide trail.
- `Model::for_entity(db, entity_pid)` — every row for one record,
  newest first.

### 2.2 The `audit_log` table (MPI lineage)

The older services carry a richer column set via `AuditLogRepository`
(`src/db/audit.rs`). It stores both an `old_values` and a `new_values`
JSON snapshot plus request provenance:

| Column | Meaning |
|---|---|
| `id` (`Uuid`) | Surrogate key. |
| `timestamp` | UTC write time (`jiff`/`time` `now_utc`). |
| `action` | `CREATE` / `UPDATE` / `DELETE`. |
| `entity_type` | The entity kind (e.g. `person`). |
| `entity_id` (`Uuid`) | The record id. |
| `old_values` (`JSON NULL`) | Prior snapshot (`None` on create). |
| `new_values` (`JSON NULL`) | New snapshot (`None` on delete). |
| `user_id` (`String NULL`) | The actor. |
| `ip_address` (`String NULL`) | Request provenance. |
| `user_agent` (`String NULL`) | Request provenance. |

Typed helpers `log_create` / `log_update` / `log_delete` wrap a private
`log_action` insert; query helpers `get_logs_for_entity`,
`get_recent_logs`, and `get_logs_by_user` back the read endpoints.

### 2.3 How the audit row is written

In the Loco lineage every mutating controller action calls a private
`audit()` helper after the DB write, passing the verified caller's
`sub`:

```text
audit(&ctx, entity_pid, "created", caller.actor(), Some(snapshot)).await;
streaming::publish_with_actor(EventKind::Created, pid, name, caller.actor());
```

The `audit()` helper calls `Model::record` and, on error, logs and
continues — the request never fails because the audit insert failed. A
merge writes **two** rows: `merged` on the survivor and `merged_into` on
the duplicate.

### 2.4 Read endpoints

Per service (paths shown for `care-pathways`; substitute the entity
plural):

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/api/care-pathways/audit/recent` | — | Newest-first system-wide trail (capped). |
| GET | `/api/care-pathways/{pid}/audit` | — | Every audit row for one record, newest first. |
| GET | `/api/auth/account/audit` | Bearer | A subject's **own** audit trail (GDPR right of access). |

The authentication service additionally exposes `/api/auth/audit/recent`
(system-wide, deliberately unauthenticated, mirroring the family
pattern) and the bearer-gated per-subject `/api/auth/account/audit`.

### 2.5 The actor

`actor` is the **bearer-token `sub`** (the caller's user pid) when a
verified RS256 JWT was presented, obtained via the `AuthUser` /
`MaybeAuthUser` extractor (`src/auth.rs`, backed by the
[authentication-verifier](../authentication/index.md) library). When no
token is present, `actor` is `None` / `NULL`. The same value flows into
both the audit row and the event envelope, so the two mechanisms agree
on identity.

### 2.6 Rule: audit rows never carry tokens or secrets

Audit rows record **identity and outcome, never credentials**. The
actor is a user pid (`sub`), not a token. Snapshots are record payloads,
not secrets. In the authentication service this is sharpened: an
`auth_events` row may distinguish outcomes internally (e.g.
`unknown_email`, `expired_token`, `rate_limited`) but must never store
the magic-link token or let a reader of the HTTP response infer account
existence (anti-enumeration is preserved at the wire). **No bearer
token, magic-link token, signing key, or password hash ever appears in
an audit row.**

---

## 3. Event stream (operational)

### 3.1 Today: in-memory ring buffer behind a publisher seam

The Loco lineage event stream (`src/streaming.rs`) is **Phase 1** of the
family's durable event-bus design: the canonical versioned `Envelope`
plus the `EventPublisher` trait, with an `InMemoryPublisher` wired as
the default. The buffer is a process-wide `OnceLock<InMemoryPublisher>`
holding a `VecDeque` ring buffer with capacity **1000** — publishing past
the cap evicts the oldest. A per-process atomic allocates a monotonic
`seq` starting at 1.

The publish path is **best-effort and never fails**: if the mutex is
poisoned the event is silently dropped, because the **audit log is the
durable record** and the stream is only the operational feed.

### 3.2 The canonical `Envelope`

One shape for every entity and transport (design §4):

| Field | Type | Meaning |
|---|---|---|
| `event_id` | `Uuid` | Idempotency / dedup key for consumers. |
| `schema_version` | `u32` | Envelope schema version (currently `1`). |
| `entity` | `&'static str` | The snake_case entity name, e.g. `care_pathway`. |
| `kind` | `EventKind` | The kind of change (see §4). |
| `pid` | `String` | The record's public id. |
| `seq` | `u64` | Monotonic per-process sequence number. |
| `actor` | `Option<String>` | The bearer-token `sub`, when known. |
| `name` | `String` | The record's name at the time of the event. |

Phase 1 deliberately **omits** `occurred_at` and `data`; the design
places those at the outbox stage (Phase 2), so they arrive with
`OutboxPublisher` rather than being threaded through the in-memory path.

### 3.3 The `EventPublisher` seam

```rust
pub trait EventPublisher: Send + Sync {
    fn publish(&self, env: Envelope);
    fn recent(&self, limit: usize) -> Vec<EventView>;
}
```

Phase 1's only implementation is `InMemoryPublisher`. Phase 2 adds
`OutboxPublisher` (an in-transaction `event_outbox` insert) behind the
same trait; the durable implementation never silently drops. The seam is
what makes the swap a contained change.

The MPI lineage uses an analogous `EventProducer` trait
(`src/streaming/producer.rs`, `InMemoryEventPublisher`) with an
`EventConsumer` stub — the same publish-on-every-CRUD intent predating
the canonical envelope.

### 3.4 Read endpoint and the operator projection

`GET /api/care-pathways/events/recent` returns the **frozen** operator
projection `EventView`:

| Field | Notes |
|---|---|
| `kind` | The change kind (lowercase wire rendering). |
| `pid` | Record public id. |
| `name` | Record name at event time. |
| `seq` | Monotonic sequence number. |

`EventView` is a deliberate projection of `Envelope` that **drops the
internal fields** (`event_id`, `schema_version`, `entity`, and
crucially `actor`) — the operator recent-activity view never exposes the
actor. The four-key JSON shape (`kind`, `pid`, `name`, `seq`) is frozen
and byte-identical to the pre-envelope wire shape, because the
front-end recent-activity view depends on it.

---

## 4. Event taxonomy

The canonical `EventKind` (and the family-wide audit `action` set):

| Kind / action | When | Notes |
|---|---|---|
| `Created` | Record created | |
| `Updated` | Record updated | |
| `Deleted` | Record soft-deleted | History is preserved; see §5. |
| `Merged` | A duplicate merged into this survivor | Survivor side. |
| `Linked` / `Unlinked` | Link added/removed between records | Modeled where the entity supports links (e.g. person links). |

The merge action writes the survivor's `merged` audit row plus a
`merged_into` row on the duplicate (§2.3), and publishes a `Merged`
event for each side.

### 4.1 The authentication service's `auth_events` trail

The authentication service keeps its **own** durable audit trail in
`auth_events` (`src/models/auth_events.rs`) rather than the generic
`audit_logs` table, because its events are authentication outcomes, not
entity CRUD:

| Column | Meaning |
|---|---|
| `event` | `signup` / `magic-link` request / `redeem` / `signout` / `me` / `account_erased`. |
| `email` | Normalised (trimmed, lowercased) subject email, when applicable. |
| `user_pid` | The subject pid, when known. |
| `detail` | Outcome marker (e.g. `rate_limited`, `unknown_email`, `expired_token`). |

`Model::record_best_effort` writes the row without ever failing the
request. `Model::for_subject` unions rows by `user_pid` **or** normalised
`email` (early flow events carry only the email; later ones carry the
pid), which is what makes the GDPR per-subject export complete. As in
§2.6, these rows never carry tokens or secrets, and the response must
not leak account existence.

---

## 5. Compliance angle

Auditability is the foundation of the family's compliance posture. See
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
(US HIPAA, UK NHS / DPA 2018, EU/UK GDPR) and
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
(GDPR, ISO/IEC 27001, ISO/IEC 42001).

| Requirement | How auditability satisfies it |
|---|---|
| **HIPAA-style who/what/when trail** | The durable `audit_logs` / `audit_log` table records actor (`sub`/`user_id`), action, entity, timestamp, and before/after snapshots for every mutation. |
| **GDPR right of access (Art. 15)** | The bearer-gated per-subject `/account/audit` (and `/account/export`) returns the subject's own trail; `auth_events::for_subject` unions email + pid so nothing is missed. |
| **Immutability / append-only** | Audit tables are append-only; helpers only `insert` and query — there is no update or delete path for audit rows. |
| **Soft-delete preserves history** | `Deleted` is a soft delete (`active = false` / `deleted_at`); the record and all its audit rows remain, so the trail survives "deletion". |
| **No secret leakage** | §2.6 / §4.1: rows carry identity and outcome only — never tokens, keys, or password hashes — and authentication outcomes never leak account existence at the wire. |

The audit log being **durable in Postgres** (not the ephemeral stream)
is what makes it admissible as the compliance system of record. The
event stream is explicitly *not* relied upon for compliance.

---

## 6. Implemented vs planned

| Capability | Status |
|---|---|
| Durable `audit_logs` / `audit_log` table + best-effort write on every CRUD/merge | **Implemented** (both lineages). |
| Audit read endpoints (`/audit/recent`, `/{pid}/audit`, `/account/audit`) | **Implemented**. |
| Bearer-token `actor` (`sub`) stamped on audit rows + envelopes | **Implemented**. |
| `auth_events` authentication trail + GDPR per-subject access | **Implemented** (authentication service). |
| Phase 1 event stream: canonical `Envelope` + `EventPublisher` seam + `InMemoryPublisher` + frozen `EventView` | **Implemented** (Loco lineage). |
| `/events/recent` operator projection | **Implemented**. |
| Phase 2: transactional outbox (`event_outbox` insert in the same DB transaction, `OutboxPublisher` behind the seam) | **Planned** — see [`agents/share/event-bus.md`](../../agents/share/event-bus.md). |
| Phase 3: durable bus sink (Fluvio); per-service durable broker | **Planned** (infra-gated roadmap). |
| `occurred_at` + `data` on the envelope | **Planned** (arrive at the outbox stage). |

The roadmap is intentionally staged so the operational stream becomes
durable **without changing the wire shape consumers already depend on**:
the `EventPublisher` seam and the frozen `EventView` projection are the
contracts that let Phases 2–3 land underneath them.
