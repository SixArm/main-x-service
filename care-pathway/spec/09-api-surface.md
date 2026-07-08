## 9. API Surface

Endpoint detail: [`AGENTS/restful.md`](../AGENTS/restful.md); source:
[`src/controllers/care_pathways.rs`](../care-pathway-service-with-loco/src/controllers/care_pathways.rs).

### 9.1 Service REST API

| Method | Path | Purpose | Returns |
|---|---|---|---|
| POST | `/api/care-pathways` | Create (body: `CarePathway`; blank `name` rejected) | `{pid, name}` |
| GET | `/api/care-pathways` | List active, most-recent first, capped 100 | `[{pid, name}]` |
| GET | `/api/care-pathways/search?q=` | Case-insensitive name search (Postgres `ILIKE`, cap 50); blank `q` → `400` | `[{pid, name}]` |
| GET | `/api/care-pathways/{pid}` | Fetch the stored `CarePathway` | `CarePathway` |
| PUT | `/api/care-pathways/{pid}` | Replace the payload | `{pid, name}` |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete | empty JSON |
| POST | `/api/care-pathways/match` | Rank `{query, candidates}` (no persistence) | `[(index, MatchResult)]` |
| POST | `/api/care-pathways/check-duplicates` | Match a query against stored pathways | `[{pid, name, score, confidence, is_match}]` sorted by score |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor (`{main_pid, duplicate_pid, reason?}`); equal pids → `422`, unknown → `404` | `{main_pid, duplicate_pid, main: CarePathway}` |
| GET | `/api/care-pathways/merges/recent` | Recent merge-history records, newest first | `[MergeRecord]` |
| GET | `/api/care-pathways/whoami` | Echo verified bearer-token claims; `401` without a valid token | `Claims` |
| GET | `/api/care-pathways/audit/recent` | Recent audit-log entries (all pathways), newest first, cap 100 | `[AuditLog]` |
| GET | `/api/care-pathways/{pid}/audit` | Audit trail for one pathway, newest first | `[AuditLog]` |
| GET | `/api/care-pathways/events/recent` | Recent CRUD events from the in-memory stream | `[PathwayEvent]` |
| GET | `/api-docs/openapi.json` | OpenAPI 3 document for the API | `OpenAPI` JSON |
| GET | `/swagger-ui` | Swagger UI rendering the spec (CDN assets) | HTML |

Plus loco's built-in `/_health` and `/_ping`.

Every create / update / delete / merge writes a best-effort `audit_logs`
row (action + JSON snapshot, durable in Postgres) and publishes a
`PathwayEvent` (`created`/`updated`/`deleted`/`merged`) to the in-memory
event stream. Merge additionally writes a `merge_records` history row
with a snapshot of the transferred (duplicate) payload. A durable broker
is roadmap (§15).

Bearer-token verification (PASETO v4 public against the auth-service's
published Ed25519 key, offline — per
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
which supersedes the prior RS256-JWT model) is available via the
`AuthUser` extractor; `whoami` is protected by it, and
create/update/delete stamp the audit `actor` from the token when one is
present (`MaybeAuthUser`). Blanket `/api/*` enforcement is wired but
**default-off** (`CARE_PATHWAY_REQUIRE_AUTH`); paseto-keys-over-HTTP
fetch at boot remains a follow-up (§13 T-7).

Conventions: **raw loco JSON** (no `{success, data, error}` envelope
— this is the loco-era convention, unlike the pre-loco person
service). `404` for unknown / soft-deleted `pid`; `422` for a
validation failure (blank `name` on create/update — family
convention, OQ-1 resolved via T-2); `400` for a malformed body. No
authentication yet (roadmap §15).

### 9.2 Front-end routes

| Route | Purpose | Endpoints consumed |
|---|---|---|
| `/` | List care pathways | `GET /api/care-pathways` |
| `/new` | Create form | `POST /api/care-pathways` |
| `/[pid]` | Detail + delete + check-duplicates | `GET`, `DELETE /api/care-pathways/{pid}`; `POST …/check-duplicates` |
| `/[pid]/edit` | Edit form | `GET`, `PUT /api/care-pathways/{pid}` |

### 9.3 Library API (matcher)

The matcher's SemVer surface is its `lib.rs` re-exports:
`CarePathway`, `PathwayIdentifier`, `IdentifierScheme`,
`ConditionCode`, `CodeSystem`, `CareSetting`, `MatchingEngine`,
`MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`,
`Error`, `Result`. Entry point:

```rust
let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_care_pathways(&a, &b);
// also: engine.rank(&query, &candidates), engine.find_matches(…)
```

### 9.4 Bulk import / export

The async, job-based bulk contract is fixed family-wide in
[bulk import/export](../../agents/share/bulk-import-export.md) (execution
model on `bg_pg`, the five endpoints, JSONL/CSV/Parquet codecs,
upsert-by-stable-key + dedupe-to-review, the per-row error report, and
export masking + audit). This section declares only the
**care-pathway-specific** bits; the shared doc is the source of truth for
everything else.

The five endpoints (shared doc §4) mount under the care-pathway resource:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/care-pathways/import` | `202 {job_id}` — body: `format`, `dedupe_mode`, `dry_run`; file upload |
| `GET` | `/api/care-pathways/import/{id}` | Job status + counts + `errors_url` + `review_url` |
| `POST` | `/api/care-pathways/export` | `202 {job_id}` — body: `format`, `filter`, `fields`, `include_soft_deleted`, `masking_profile` |
| `GET` | `/api/care-pathways/export/{id}` | Job status + `download_url` |
| `GET` | `/api/care-pathways/bulk-jobs` | List (filter by `kind`/`status`); `GET .../{id}` for one |

**Stable key(s) for upsert** (shared doc §6, §10). A row upserts in place
when it carries either:

- a **deterministic scheme-scoped identifier** — the same `identifiers`
  the matcher short-circuits on (R-0, matcher §15), keyed by
  `(scheme, value)`: `Doi`, `Wikidata`, `GuidelineId`, `Uri`, `Uuid`; or
- the **provider-scoped pathway code** keyed by `(provider_id,
  pathway_code)` — never globally unique, so it upserts **only within the
  same `provider_id`** (the cross-provider-uniqueness invariant, §5.5);
  the matcher likewise never short-circuits across providers; or
- the record **`pid`** (the pathway UUID) when present in the row.

A row with neither runs the normal duplicate detection (`check-duplicates`
path, §6.7), routing likely duplicates to the review queue with
`provenance = import` (matching the cross-service-linking provenance
vocabulary).

**CSV column set + flattening** (shared doc §5). CSV is the operator /
spreadsheet format and is lossy for deep nesting — steer fidelity-sensitive
loads to **JSONL** (the lossless reference). Flat columns:

- **scalar** (one column each): `pid`, `name`, `pathway_code`,
  `provider_id`, `provider_name`, `care_setting`, `in_language`, `active`;
- **arrays / arrays-of-objects** → a single **JSON-encoded cell** each:
  `alternate_names`, `condition_codes` (`{system, code}`), `interventions`,
  `keywords`, `tags`, `identifiers` (`{scheme, value}`), `same_as`, and
  `relationships` (`{relation, pathway_id}`).

There is no single-nested-object field, so no dotted columns; every
repeated / nested field is a JSON-encoded cell. JSONL round-trips the whole
`CarePathway` payload losslessly.

**Export sensitivity** (shared doc §8). Care pathways are **clinical
reference data** (medium sensitivity — clinical guidance, but pathway
templates carry **no patient-level data**, §12 / §5.5): export defaults to
**masked** per the shared contract; full / unmasked output requires a
`masking_profile` selecting elevated authorisation and must never reveal
more than the caller could read one record at a time. `include_soft_deleted`
defaults `false` and is gated. **Every export is audited** (actor, filter,
format, row count, masking profile, timestamp — written even for a zero-row
export).
