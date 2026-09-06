# Record merge

Monorepo-wide reference for **record merging** across the **Main X
Index** family: folding a confirmed-duplicate record into a surviving
**main** record. This is the comprehensive spec; the short version
lives at [`agents/share/merge.md`](../../agents/share/merge.md), and the
consolidated dedup view at
[`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md).

Each service owns its own merge implementation — there is no shared
merge crate — but they all follow the conventions below. The family
splits into two implementation lineages:

- **Loco lineage** (`organization`, `care-pathway`, `case`, and
  `portfolio` — the latter carries the same `src/merge.rs` +
  `merge_records` + `POST /merge` shape and is included in this lineage
  throughout this doc, even though earlier revisions of this list
  omitted it): pure fold in `src/merge.rs`, persisted to a
  `merge_records` table, exposed at `POST /merge` + `GET /merges/recent`.
  This is the canonical shape and the one this spec describes in detail.
- **MPI lineage** (`person`, `place`, `thing`, `event`, `worker`,
  `course`): the older multi-table model in `src/models/merge.rs`
  (`MergeRequest` / `MergeRecord` / `MergeResponse`). Same workflow,
  different persistence; see §8.

> Related (monorepo topic specs that exist): [postgresql](../postgresql/index.md),
> [search](../search/index.md), [architecture](../architecture/index.md),
> [matching](../matching/index.md), [auditability](../auditability/index.md),
> and [event-streaming](../event-streaming/index.md) — this corrects an
> earlier version of this note that described the latter three as "not
> yet written"; all three now exist and are linked directly below rather
> than only through the `agents/share/*.md` briefs.

---

## 1. What merge does

Merge **folds a confirmed-duplicate record into a surviving main
record**. Two records that have been confirmed to denote the same
real-world entity are reconciled: one is chosen as the survivor (the
**main** record) and the other as the **duplicate**. The duplicate's
data is transferred onto main, the duplicate is soft-deleted, and a
permanent history row plus events record what happened.

**Terminology.** The codebase uses **main** / **duplicate**, not
"master" / "slave". Models name the surviving id `main_pid` (loco
lineage) or `main_person_id` (MPI lineage). The retired record is the
`duplicate_pid` / `duplicate_person_id`.

| Concept | Meaning |
| --- | --- |
| **main** | The surviving record. Keeps its own identity (`pid`, primary name/title). |
| **duplicate** | The record folded in and then soft-deleted. Its data is transferred to main. |
| **transferred** | A JSON snapshot of the duplicate's full payload at merge time, stored for audit. |

Merge is **not** symmetric: main keeps its title and pid; the duplicate
is retired. Choosing which record is main is the caller's decision
(operator UI or auto-merge rule), not the service's.

---

## 2. The merge algorithm (pure fold)

The loco services factor the merge into a **pure, DB-free fold** in
`src/merge.rs` (e.g.
[`care-pathway-service/src/merge.rs`](../../care-pathway/care-pathway-service-with-loco/src/merge.rs)
`merge_pathways`,
[`organization-service/src/merge.rs`](../../organization/organization-service-with-loco/src/merge.rs),
[`case-service/src/merge.rs`](../../case/case-service-with-loco/src/merge.rs)).
The fold takes `(main, duplicate)` and returns a `MergeOutcome { merged,
transferred }`. It is unit-testable without a database; the DB
orchestration and side effects live in the controller (see §3).

The fold applies three field strategies plus a former-name rule:

| Field kind | Rule |
| --- | --- |
| **Scalars** (e.g. `pathway_code`, `provider_id`, `care_setting`) | Keep main's value; adopt the duplicate's **only when main's is empty** (`main.x.or(duplicate.x)`). |
| **Lists** (e.g. `keywords`, `interventions`, `same_as`, `in_language`, `condition_codes`, `identifiers`) | **Union**, preserving main's order and appending each of the duplicate's entries not already present (dedup by exact equality, or by an identity key such as `system`+`code` / `scheme`+`value`). |
| **Primary name / title** | **Never changed** — the survivor keeps its own `name`/`title`. |
| **Former name** | The duplicate's primary name/title is **added to main's `alternate_names`** as a "former" alias, when it differs from main's and is not already present. |

The `transferred` value is the **full serialized duplicate** at merge
time (`serde_json::to_value(duplicate)`), carried into the history row
so the merge is reconstructable for audit.

### Orchestration (the full operation)

The controller wraps the pure fold with the persistence and
side-effects. The end-to-end operation is:

1. **Validate** the request (see §3 guards): reject self-merge, fetch
   both records.
2. **Fold**: `merge_pathways(&main, &duplicate)` → union lists, keep
   the duplicate's title as a "former" alternate name on main, fill
   empty scalars from the duplicate, transfer identifiers / addresses /
   contacts / etc.
3. **Update main** with the merged payload.
4. **Soft-delete the duplicate** (set `deleted_at`; the row is retained).
5. **Link** main → duplicate with a `Replaces` link (and, in MPI
   lineage, `ReplacedBy` on the duplicate). _Loco lineage records the
   `Replaces` relationship implicitly via the `merge_records` row
   (`main_pid` → `duplicate_pid`); MPI lineage writes an explicit
   `PersonLink`._
6. **Write a `merge_records` history row** with a JSON snapshot of the
   transferred data (§5).
7. **Audit**: one `audit_logs` row on the survivor (`"merged"`) and one
   on the retired duplicate (`"merged_into"`), so both pids carry the
   trail (§6).
8. **Publish events**: a `Merged` event for main **and** a `Deleted`
   event for the duplicate, both stamped with the actor (§6).
9. **Respond** with `{main_pid, duplicate_pid, main}` (the survivor's
   merged payload).

Steps 6–8 are **best-effort**: a failure to write the history row or
audit row is logged (`tracing::warn!`) but does not roll back the
already-committed merge. The merge itself (steps 3–4) is the
authoritative state change.

---

## 3. Status codes & guards

The `POST /merge` handler enforces two guards before doing any work:

| Condition | Status | Reason |
| --- | --- | --- |
| `main_pid == duplicate_pid` | **422 Unprocessable Entity** | Cannot merge a record into itself — it would soft-delete the only copy. Error body: `validation` / `"main_pid and duplicate_pid must differ"`. |
| `main_pid` or `duplicate_pid` not found / soft-deleted | **404 Not Found** | Surfaced by `find_by_pid`; either record missing fails the lookup. |
| `pid` not a valid UUID | **400 Bad Request** | Malformed path/body id. |
| Success | **200 OK** | Body: `{main_pid, duplicate_pid, main}`. |

The self-merge `422` and unknown-pid `404` are pinned by a DB-free
controller test in each loco service (e.g. the care-pathway controller
422 test), so they hold without a live database.

---

## 4. Auto-merge vs. review queue

Whether a candidate pair is merged automatically or queued for a human
depends on the **match confidence** (see the matching reference —
[`agents/share/match.md`](../../agents/share/match.md) — and the planned
[matching spec](../matching/index.md)).

| Confidence | Band (default) | Action |
| --- | --- | --- |
| **Certain** | ≥ 0.95 | Eligible for **auto-merge** (`AutoMerged`). |
| **Probable** | ≥ 0.80–0.85 | **Review queue** — operator confirms. |
| **Possible** | ≥ 0.50–0.60 | **Review queue** — operator reviews. |
| **Unlikely** | below | Not surfaced as a duplicate. |

Thresholds are configurable per service (`threshold`,
`auto_merge_threshold`, `max_candidates`).

**Review-queue item lifecycle** (MPI lineage `ReviewQueueItem`):
`Pending` → `Confirmed` / `Rejected` / `AutoMerged`. A confirmed item
drives a `POST /merge`. Candidate pairs come from:

- **Real-time** duplicate detection on create (`409 Conflict` with
  candidate matches),
- the **explicit** `POST /check-duplicates` endpoint,
- the **batch** `POST /deduplicate` scan (MPI lineage).

Candidate generation (blocking / search) is covered by the search /
dedup reference — [`agents/share/search.md`](../../agents/share/search.md)
and the [search spec](../search/index.md). In the loco lineage,
`check-duplicates` candidates are now **search-blocked** on all four
crates (organization, care-pathway, case, portfolio — Tantivy
`index.candidates(&query, …)`, landed alongside each crate's Tantivy
migration, 2026-07-31 through 2026-08-02), not deferred; what remains
**deferred**, and only in three of the four, is the **batch
`deduplicate` scan endpoint** — organization has it, care-pathway/case/
portfolio do not yet (see §9).

---

## 5. Persistence

### Loco lineage — `merge_records` table

One row per merge, written by `MergeRecordModel::record(...)`. Migration:
[`m20220101_000003_merge_records.rs`](../../care-pathway/care-pathway-service-with-loco/migration/src/m20220101_000003_merge_records.rs).

| Column | Type | Purpose |
| --- | --- | --- |
| `id` | `PkAuto` | Surrogate key; `recent` orders by `id DESC`. |
| `main_pid` | `Uuid` | The surviving (main) record's pid. |
| `duplicate_pid` | `Uuid` | The merged-away duplicate's pid (now soft-deleted). |
| `reason` | `StringNull` | Optional operator-supplied reason. |
| `actor` | `StringNull` | Caller's `sub` from a verified bearer token; null when anonymous. |
| `transferred` | `JsonBinaryNull` | JSONB snapshot of the duplicate's payload at merge time. |

**Read endpoint.** `GET /api/<plural>/merges/recent` returns the most
recent merge-history rows (capped at 100), newest first
(`MergeRecordModel::recent`).

**Soft-delete of the duplicate.** The duplicate row is never physically
removed: `soft_delete` sets its `deleted_at` and drops it from the
active set, but the row (and thus its data) is retained. Combined with
the `transferred` snapshot, the merge is fully reconstructable.

### Data-modeling notes

The `transferred` snapshot is the one place JSONB is used to capture a
point-in-time payload; this is consistent with the monorepo
[data-modeling](../data-modeling.md) JSONB policy (snapshots and verbatim
payloads, not normalized child data). See
[postgresql](../postgresql/index.md) for the SeaORM / migration
conventions all these tables follow.

---

## 6. Auditability

Every merge produces both an **audit trail** (queryable history in
PostgreSQL) and **events** (the streaming envelope). See the
auditability brief — [`agents/share/auditability.md`](../../agents/share/auditability.md)
— and the planned [auditability](../auditability/index.md) /
[event-streaming](../event-streaming/index.md) specs.

**Audit rows (two per merge).** So both pids carry the trail:

| Subject | Action | Snapshot |
| --- | --- | --- |
| survivor (`main_pid`) | `"merged"` | the survivor's post-merge payload |
| duplicate (`duplicate_pid`) | `"merged_into"` | none |

Audit writes are **best-effort**: a failure is logged but never fails
the request. Queryable via `GET /audit/recent` and `GET /{pid}/audit`.
The `actor` (caller's verified `sub`, or null) is stamped on every audit
row and merge record.

**Events (two per merge).** Published with the actor:

| Event | Subject |
| --- | --- |
| `Merged` | the survivor (`main_pid` + merged name) |
| `Deleted` | the retired duplicate (`duplicate_pid` + its name) |

In the loco lineage these go through the `EventPublisher` seam
(`streaming::publish_with_actor`), in-memory by default
(`<ENTITY>_EVENT_TRANSPORT=memory`); a durable outbox + `FluvioSink`
relay is **shipped, default-off, on all ten entity registries** — not
deferred code, just off by default — see
[`spec/event-streaming/index.md`](../event-streaming/index.md) §4/§8.
The event taxonomy — `Created` / `Updated` / `Deleted` / `Merged` (and
`Linked` / `Unlinked` in the MPI lineage) — is shared across the family.

---

## 7. Reversibility & limitations

- **Soft-delete preserves data.** The duplicate is retained
  (`deleted_at` set), and the `transferred` JSONB snapshot captures its
  full payload. No data is destroyed by a merge, so a merge is
  recoverable by hand from the history row.
- **No automated un-merge endpoint.** The loco lineage exposes no
  reverse operation; restoring is a manual data task. The MPI-lineage
  `MergeStatus` enum models a `Reversed` state
  ([`person-service/src/models/merge.rs`](../../person/person-service-with-loco/src/models/merge.rs)),
  but a reverse endpoint is not implemented.
- **Best-effort side effects.** History, audit, and event writes are
  logged-but-non-fatal; the primary update + soft-delete is the
  authoritative state change. A monitoring gap (lost audit/event) does
  not corrupt the merged record.
- **No cross-entity merge.** Merge is within a single entity service;
  there is no cross-service identity reconciliation here.

### Front-end merge action

| Lineage / project | Merge UI | Notes |
| --- | --- | --- |
| [person front-end](../../person/person-front-end-with-svelte) | **Implemented** | Dedicated `/persons/merge` route: enter both ids + reason, optional side-by-side preview, confirm. |
| [care-pathway front-end](../../care-pathway/care-pathway-front-end-with-svelte) | **Implemented** | Inline two-step "Confirm merge?" on the `[pid]` detail page; detail record is the survivor. |
| [organization front-end](../../organization/organization-front-end-with-svelte) | **API only** | `client.ts` has a `merge` call; no merge UI page (deferred). |
| [case front-end](../../case/case-front-end-with-svelte) | **API only** | `client.ts` has a `merge` call; no merge UI page (deferred). |

---

## 8. Lineage comparison

| Aspect | Loco lineage (`organization`, `care-pathway`, `case`, `portfolio`) | MPI lineage (`person`, `place`, `thing`, `event`, `worker`, `course`) |
| --- | --- | --- |
| Pure fold | `src/merge.rs` (`merge_*`) → `MergeOutcome` | inline in REST handler |
| History model | `src/models/merge_records.rs` + `merge_records` table | `src/models/merge.rs` (`MergeRecord`, `MergeStatus`) |
| Surviving id | `main_pid` | `main_person_id` (and entity equivalents) |
| Retired id | `duplicate_pid` | `duplicate_person_id` |
| Snapshot column | `transferred` (JSONB) | `transferred_data` (JSON) |
| Request | `{main_pid, duplicate_pid, reason}` | `MergeRequest { main_*_id, duplicate_*_id, merge_reason, merged_by }` |
| Status | implicit (row exists) | `MergeStatus { Completed, Reversed }` |
| Match score | not stored on the row | `match_score: Option<f64>` |
| Endpoints | `POST /merge`, `GET /merges/recent` | `POST /merge` (+ `POST /deduplicate` batch) |
| Link | implicit via history row | explicit `PersonLink` (`Replaces` / `ReplacedBy`) |

Both lineages implement the same conceptual workflow (§1–§2); they
differ only in persistence shape and surrounding plumbing.

---

## 9. Implemented vs. deferred

**Implemented (all loco services):**

- Pure merge fold (scalar fill, list union, former-name alias).
- `POST /merge` with `422` (self-merge) / `404` (unknown pid) guards.
- `merge_records` history table + JSONB `transferred` snapshot.
- Soft-delete of the duplicate (data retained).
- `GET /merges/recent` read endpoint.
- Dual audit rows (`merged` / `merged_into`) + `Merged` + `Deleted`
  events, stamped with the actor.
- `POST /check-duplicates` (query-vs-stored matching).

**Deferred:**

- Automated **un-merge / reverse** endpoint (data is recoverable
  manually; MPI `Reversed` status is modeled but unused).
- The **batch `deduplicate`** scan endpoint in the loco lineage —
  implemented only in **organization**; care-pathway, case, and
  portfolio don't yet expose it (MPI lineage has the batch scan on
  every crate). **Not** deferred any more: search-blocked **candidate
  generation** for `check-duplicates`, which is now implemented on all
  four loco crates (§4) — an earlier version of this list grouped the
  two together as both deferred, which was true only for the batch scan.
- A formal **review queue** table in the loco lineage (MPI lineage has
  `ReviewQueueItem`).
- **Durable event bus flip** — the outbox + `FluvioSink` transport
  itself has **shipped**, default-off, on all ten entity registries
  (not deferred code); what's still open is a deployment actually
  setting `<ENTITY>_EVENT_TRANSPORT=outbox` for `Merged`/`Deleted`
  events, which remains a per-deployment decision (§6,
  [`spec/event-streaming/index.md`](../event-streaming/index.md)).
- **Merge UI** in the organization and case front-ends (API client
  present; no page).

---

## 10. See also

- [`agents/share/merge.md`](../../agents/share/merge.md) — short merge brief.
- [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md) — consolidated dedup reference.
- [`agents/share/match.md`](../../agents/share/match.md) — matching algorithms & confidence bands.
- [`agents/share/search.md`](../../agents/share/search.md) — search / candidate generation.
- [`agents/share/auditability.md`](../../agents/share/auditability.md) — audit + event taxonomy.
- [postgresql](../postgresql/index.md) — DB / SeaORM / migration conventions.
- [data-modeling](../data-modeling.md) — JSONB & child-table policy.
- Entity specs: [person](../../person/spec/index.md) ·
  [care-pathway](../../care-pathway/spec/index.md) ·
  [organization](../../organization/spec/index.md) ·
  [case](../../case/spec/index.md).
- Topic siblings: [search](../search/index.md),
  [architecture](../architecture/index.md),
  [matching](../matching/index.md),
  [auditability](../auditability/index.md),
  [event-streaming](../event-streaming/index.md) — all now written
  (corrects the "not yet written" note this list used to carry).
