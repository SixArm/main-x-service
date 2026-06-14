# Data flow

Monorepo-wide reference for the request/data flows that every Main X
Index service crate shares. This is the canonical, repo-accurate
expansion of the one-page brief at
[../../agents/share/dataflow.md](../../agents/share/dataflow.md): it
shows the **actual ordered steps**, with HTTP status codes, for each
pipeline as implemented in the service crates.

Two generations of service exist in the repo, and the step *order* is
the same in both; the per-step *detail* differs:

- **loco.rs services** (authentication, organization, care-pathway,
  case, and the loco-converted person/worker/place/thing/event/course
  controllers). The API DTO **is** the matcher type, stored verbatim as
  a single JSONB `data` column. Reference handler:
  [`care-pathway/care-pathway-service-rust-crate/src/controllers/care_pathways.rs`](../../care-pathway/care-pathway-service-rust-crate/src/controllers/care_pathways.rs).
- **older Axum services** (the pre-conversion `src/api/rest/handlers.rs`
  layer, e.g.
  [`person/person-service-rust-crate/src/api/rest/handlers.rs`](../../person/person-service-rust-crate/src/api/rest/handlers.rs)).
  Records are spread across **normalized child tables** and indexed in
  **Tantivy**.

Where they differ, the difference is called out under
[§7 loco-vs-older differences](#7-loco-vs-older-differences). Sibling
topic specs hold the deeper per-stage detail:
[validation](../validation/index.md) ·
[matching](../matching/index.md) ·
[search](../search/index.md) ·
[merge](../merge/index.md) ·
[auditability](../auditability/index.md) ·
[event-streaming](../event-streaming/index.md) ·
[privacy](../privacy/index.md) ·
[authentication](../authentication/index.md) ·
[postgresql](../postgresql/index.md).

---

## 1. Create flow

`POST /api/<plural>` — create a record. The body is the entity DTO
(in loco services, the matcher type itself).

```
HTTP POST
  │
  ├─ 1. Validate the payload ........................ 422 on any problem
  │     all problems reported together (one round-trip)
  │
  ├─ 2. Duplicate detection (where wired)
  │       a. search → blocking candidates
  │       b. match each candidate → score + classify
  │
  ├─ 3. If duplicates found ......................... 409 Conflict + candidates
  │
  ├─ 4. Persist: INSERT (transactional) ............. row committed
  │
  ├─ 5. Search index: upsert into Tantivy (where present)
  │
  ├─ 6. Publish `Created` event (in-memory stream)
  │
  ├─ 7. Write audit row (best-effort, actor from token)
  │
  └─ 8. Response ........................... 201 Created (older) / 200 (loco)
```

Step-by-step:

1. **Validate.** `validate(&payload)` collects *every* problem and
   reports them in one response so the caller fixes them in a single
   round-trip. A failure is **`422 Unprocessable Entity`** (family
   convention; loco has no `unprocessable_entity` helper, so the loco
   services raise `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY,
   …)`). Rules live in each crate's `src/validation.rs`. See
   [validation](../validation/index.md).
2. **Duplicate detection.** Where wired, search produces blocking
   candidates, then the matcher scores each and classifies the
   confidence band. See [matching](../matching/index.md) and
   [search](../search/index.md). (In the current loco services this
   real-time gate on create is deferred; the equivalent is the explicit
   [check-duplicates](#2-match--check-duplicates-flow) endpoint.)
3. **Conflict.** If duplicates are detected, return **`409 Conflict`**
   with the candidate matches in the body — nothing is persisted.
4. **Persist.** `Model::create(&db, &payload)` performs the INSERT.
   In the older services this fans out across normalized child tables
   inside one transaction; in loco services it is a single-row INSERT of
   the JSONB `data` column plus the denormalized `name`/`pid`.
5. **Search index.** Older services call the search engine to index the
   new record (Tantivy). Loco services skip this — full-text indexing is
   deferred (`ILIKE` name search stands in).
6. **Publish event.** A `Created` event goes to the in-memory event
   stream (`streaming::publish_with_actor(EventKind::Created, …)`),
   carrying the entity id, name, and actor. See
   [event-streaming](../event-streaming/index.md).
7. **Audit.** A best-effort audit row is written
   (`AuditModel::record(&db, pid, "created", actor, snapshot)`). The
   actor is the verified bearer-token `sub` when a token was presented
   (see [§6 Auth flow](#6-auth-flow)). A failure here is logged at
   `WARN` and swallowed — the create has already committed and the audit
   trail is a secondary side-channel. See
   [auditability](../auditability/index.md).
8. **Respond.** Older services return **`201 Created`**; loco services
   return **`200`** with the lightweight `{pid, name}` reference.

> Ordering note: steps 6–7 are *post-commit* side-channels. The record
> exists the instant step 4 commits; event and audit are emitted after,
> best-effort. They never roll back the write.

---

## 2. Match / check-duplicates flow

Two read-only scoring endpoints. **Nothing is persisted.**

`POST /api/<plural>/match` — rank an explicit candidate list:

```
HTTP POST {query, candidates}
  │
  ├─ 1. engine = MatchingEngine::new(MatchConfig::default())
  ├─ 2. engine.rank(&query, &candidates)   (score every candidate)
  └─ 3. Response: ranked [index, MatchResult] + per-component breakdown
```

`POST /api/<plural>/check-duplicates` — find stored records that match:

```
HTTP POST {query}
  │
  ├─ 1. Candidate set
  │       loco today: in-memory scan of up to CHECK_DUPLICATES_SCAN_CAP
  │                   (1000) active rows; WARN if the cap is hit
  │       target / older: search → blocking candidates
  │
  ├─ 2. Score each candidate with the matcher
  ├─ 3. Classify (confidence bands) + filter to is_match
  ├─ 4. Sort best-first (descending score)
  └─ 5. Response: [{pid, name, score, confidence, is_match}]
```

The matcher returns `score` in `[0.0, 1.0]`, an `is_match` boolean, and
a `confidence` band. Classification bands (configurable):

| Quality  | Range         |
| -------- | ------------- |
| Certain  | ≥ 0.95        |
| Probable | ≥ 0.80–0.85   |
| Possible | ≥ 0.50–0.60   |
| Unlikely | below         |

`f64` is only partially ordered, so the sort falls back to "equal" for
any `NaN` (which the engine never emits) rather than panicking. The
loco in-memory scan is a stop-gap: hitting the cap means the result may
be incomplete (search-backed candidate blocking is deferred). See
[matching](../matching/index.md).

---

## 3. Merge flow

`POST /api/<plural>/merge` — fold a confirmed duplicate into a
surviving (main) record.

```
HTTP POST {main_pid, duplicate_pid, reason?}
  │
  ├─ 1. Reject self-merge: main_pid == duplicate_pid .... 422
  │
  ├─ 2. Fetch main, fetch duplicate (find_by_pid) ....... 404 if either missing
  │
  ├─ 3. Fold: union duplicate's data into main
  │       (pure logic, e.g. merge.rs::merge_pathways → {merged, transferred})
  │       duplicate's title kept as an alternate/"former" name
  │
  ├─ 4. Update main with the merged payload
  │
  ├─ 5. Soft-delete the duplicate (mark inactive, stamp deleted_at)
  │
  ├─ 6. Record merge history (merge_records: main, dup, reason, actor,
  │       transferred snapshot) — best-effort, WARN on failure
  │
  ├─ 7. Publish `Merged` (survivor) + `Deleted` (duplicate)
  │
  ├─ 8. Audit: "merged" on survivor, "merged_into" on duplicate
  │       (both pids carry the trail)
  │
  └─ 9. Response: {main_pid, duplicate_pid, main}
```

Step notes:

- **Self-merge guard (1).** Folding a record into itself is a no-op that
  would still soft-delete the only copy, so it is rejected up front as
  **`422`**.
- **Existence (2).** Both records must exist and be active; a missing
  pid is the `find_by_pid` **`404`**.
- **Fold (3).** The pure union logic lives in `src/merge.rs` and returns
  both the merged record and a `transferred` snapshot of what moved.
  The conceptual model is "transfer identifiers/names/addresses/contacts
  from duplicate to main; add the duplicate's primary name as a former
  alias; create a `Replaces` link main → duplicate." See
  [merge](../merge/index.md).
- **History (6) and audit (8) are best-effort.** A failure to write the
  merge-history row is logged at `WARN` and swallowed — it must not roll
  back the already-committed merge. The merge emits **two** audit rows so
  both pids carry the trail. See
  [auditability](../auditability/index.md).

`GET /api/<plural>/merges/recent` returns the merge-history records,
newest first.

---

## 4. Search flow

`GET /api/<plural>/search?q=…` — name/title substring search.

```
HTTP GET ?q=stroke
  │
  ├─ 1. Reject absent / blank q ......................... 400
  │
  ├─ 2. Query
  │       loco today: Postgres ILIKE over denormalized name (cap 50)
  │       older / target: Tantivy full-text query (fuzzy + phonetic)
  │
  ├─ 3. Fetch matched rows / fetch-by-id batch
  │
  ├─ 4. Optional: mask sensitive fields (search param)
  │
  └─ 5. Response: JSON array (+ pagination: offset + limit)
```

The loco services do a pragmatic case-insensitive `ILIKE` over the
denormalized `name`/`title` column (capped, currently 50 rows) and
return lightweight `{pid, name}` references; full-text, fuzzy, and
phonetic search via Tantivy is deferred. The older services run a
Tantivy query, collect the matching ids, batch-fetch the records via the
repository, optionally mask, then serialize with `offset`/`limit`
pagination. See [search](../search/index.md) and
[postgresql](../postgresql/index.md).

---

## 5. Read / masked / export flows

Plain reads and the privacy-oriented variants.

```
GET /api/<plural>/{id}            → 200 full record  (404 if unknown/soft-deleted)
GET /api/<plural>/{id}/masked     → 200 record with sensitive fields masked
GET /api/<plural>/{id}/export     → 200 GDPR data-export JSON
```

1. **Read.** `GET {id}` returns the full stored record. A pid that is
   unknown or soft-deleted is a **`404`** (`find_by_pid`). A pid that is
   not a well-formed UUID, where validated up front, is a **`400`**.
2. **Masked (`{id}/masked`).** Returns the record with sensitive fields
   masked (coordinates, phone, email, postal address). See
   [privacy](../privacy/index.md).
3. **Export (`{id}/export`).** Returns a GDPR right-of-access data
   export of the record.

Masked/export are present in the older privacy-complete services
(person/place/worker) and deferred in several loco services (e.g.
case privacy masking + GDPR export is on the deferred list) — consult
the per-crate `spec.md §13` for the live status.

---

## 6. Auth flow

Brief; the full design is in [authentication](../authentication/index.md).

```
1. Magic-link:  user → authentication-service → emailed link
2. JWT:         link callback → RS256 JWT issued (+ JWKS published)
3. Bearer:      caller sends `Authorization: Bearer <jwt>` on requests
4. Verify:      each service verifies the token OFFLINE against the
                auth-service JWKS (no introspection hop) — authentication-verifier
5. Actor:       the verified `sub` becomes the audit/event actor
```

Two extractors gate handlers in the loco services
(`src/auth.rs`, embedding `authentication-verifier`):

- `AuthUser` — **required** token. The extractor verifies `kid` / `iss`
  / `aud` / `exp` and rejects a missing/invalid token with **`401`**
  *before the handler runs* (e.g. `GET /whoami`).
- `MaybeAuthUser` — **optional** token. When present and valid, its
  `sub` is stamped as the audit `actor` and event actor via
  `caller.actor()`; when absent, the mutation still proceeds with a
  `None` actor.

Blanket `/api/*` JWT enforcement is deferred in several services; today
auth is opt-in per route. See the per-crate `spec.md`.

---

## 7. Cross-cutting

### Every mutation → audit + event

Every create / update / delete / merge:

1. commits the DB change first, then
2. publishes an `EventKind` event (`Created`, `Updated`, `Deleted`,
   `Merged`) to the in-memory stream, and
3. writes a best-effort `audit_logs` row (`created`, `updated`,
   `deleted`, `merged`, `merged_into`) with the actor from the verified
   token.

Audit and event are **post-commit side-channels**: a failure is logged
at `WARN` and never fails the request or rolls back the write. See
[auditability](../auditability/index.md) and
[event-streaming](../event-streaming/index.md).

### Soft-delete everywhere

`DELETE` never hard-deletes. It marks the row inactive and stamps
`deleted_at`; the row is retained for audit. Reads (`find_by_pid`,
`list`, `search`) only surface active rows, so a soft-deleted record
404s on subsequent reads. Merge reuses the same soft-delete on the
retired duplicate.

### loco-vs-older differences

The pipelines are step-for-step identical; the per-step *detail*
differs between the two service generations:

| Stage | loco.rs services | older Axum services |
| --- | --- | --- |
| Persistence (create §1.4) | single-row INSERT of JSONB `data` + denormalized `name`/`pid` | INSERT fanned across normalized child tables in one transaction |
| DTO | the matcher type stored verbatim (no adapter) | a separate `Person`/etc. model + adapter to the matcher type |
| Search (§4) | Postgres `ILIKE` over the denormalized name (cap) | Tantivy full-text / fuzzy / phonetic |
| Create-time dedup (§1.2–§1.3) | deferred; explicit `check-duplicates` only | real-time on create → `409` |
| Success status (§1.8) | `200` with `{pid, name}` | `201 Created` |
| Validation error helper | `Error::CustomError(422, …)` | `ApiResponse::error` + `StatusCode::UNPROCESSABLE_ENTITY` |
| Privacy (§5) | masked/export deferred in several crates | present (person/place/worker) |
| check-duplicates candidates (§2) | in-memory scan (cap 1000, WARN on cap) | search-blocked candidates |

For any specific crate, the per-crate `spec.md §13` (live task queue) is
the source of truth on which of these stages are wired versus deferred.

---

## See also

- [../../agents/share/dataflow.md](../../agents/share/dataflow.md) — the one-page brief
- [../matching/index.md](../matching/index.md) — scoring + classification
- [../search/index.md](../search/index.md) — search backends
- [../merge/index.md](../merge/index.md) — merge / fold logic
- [../validation/index.md](../validation/index.md) — payload validation → 422
- [../auditability/index.md](../auditability/index.md) — audit log
- [../event-streaming/index.md](../event-streaming/index.md) — event stream
- [../privacy/index.md](../privacy/index.md) — masking + GDPR export
- [../authentication/index.md](../authentication/index.md) — magic-link + JWT
- [../postgresql/index.md](../postgresql/index.md) — persistence
