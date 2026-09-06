# Search

Monorepo-wide reference for how the **Main X Index** family does record
search. This is the comprehensive spec; the short version lives at
[`agents/share/search.md`](../../agents/share/search.md). Each service
owns its own database and its own search surface — there is no shared
search service — but they all follow the conventions below.

> Status legend used throughout: **[implemented]** ships today,
> **[planned]** is the intended upgrade not yet wired, **[partial]**
> exists in some crates only.
>
> **Tantivy full-text search is live on all ten entity registries**
> (person, worker, place, thing, event, course, organization,
> care-pathway, case, portfolio) — organization landed 2026-07-31,
> care-pathway 2026-08-01, case + portfolio 2026-08-02, the other six
> earlier. This supersedes most of §1 below, which described the
> pre-Tantivy state of the four newest loco services; §1 is kept as a
> historical record of the `ILIKE` tier plus a note on what, if
> anything, still uses it (see §1.5). See
> [`agents/share/overview.md`](../../agents/share/overview.md) footnote
> ¹ and [`agents/share/search.md`](../../agents/share/search.md).

> Related topics: [postgresql](../postgresql/index.md) ·
> [match (short)](../../agents/share/match.md) ·
> [merge (short)](../../agents/share/merge.md) ·
> [privacy (short)](../../agents/share/privacy.md) ·
> [match-search-merge (combined)](../../agents/share/match-search-merge.md).
> Per-entity specs:
> [organization](../../organization/organization-service-with-loco/spec/index.md) ·
> [care-pathway](../../care-pathway/care-pathway-service-with-loco/spec/index.md) ·
> [case](../../case/case-service-with-loco/spec/index.md) ·
> [person](../../person/person-service-with-loco/spec/index.md).

---

## 1. Historical state — Postgres `ILIKE` substring search [superseded]

> **This section describes the pre-Tantivy state of the loco.rs
> services.** All four (organization, care-pathway, case, portfolio)
> have since migrated their live `GET /<plural>/search` endpoint to
> Tantivy (§2); this section is kept because the `ILIKE` mechanics it
> documents (the `escape_like` wildcard guard, the blank-query guard)
> either still exist as dead code in some crates or were carried
> forward conceptually into the Tantivy handlers. See §1.5 for the
> current, per-crate reality.

The **loco.rs** services (organization, care-pathway, case, portfolio)
originally did **pragmatic Postgres `ILIKE '%q%'` substring search**
over a single **denormalised scalar column**, not over the JSONB
payload — before Tantivy landed in each (§2).

### 1.1 What the query does

The search column is the entity's denormalised name/title, populated on
every create/update alongside the JSONB `data` payload:

| Service | Search column | Source file (model `search`) |
|---|---|---|
| organization-service | `name` | [`src/models/organizations.rs`](../../organization/organization-service-with-loco/src/models/organizations.rs) |
| care-pathway-service | `name` | [`src/models/care_pathways.rs`](../../care-pathway/care-pathway-service-with-loco/src/models/care_pathways.rs) |
| case-service | `title` | [`src/models/cases.rs`](../../case/case-service-with-loco/src/models/cases.rs) |

The SeaORM query is uniform across the three:

```rust
// care-pathway-service: src/models/care_pathways.rs
pub async fn search(db: &DatabaseConnection, q: &str, limit: u64) -> ModelResult<Vec<Self>> {
    let pattern = format!("%{}%", escape_like(q));
    let rows = care_pathways::Entity::find()
        .filter(care_pathways::Column::DeletedAt.is_null())  // active rows only
        .filter(Expr::col(care_pathways::Column::Name).ilike(pattern))
        .order_by_desc(care_pathways::Column::Id)            // newest first
        .limit(limit)                                        // controller passes 50
        .all(db)
        .await?;
    Ok(rows)
}
```

Properties (identical across the loco services):

- **Case-insensitive substring** — `ILIKE '%q%'`, so `acme` matches
  `Acme Corp` and `ACME`.
- **Active rows only** — `deleted_at IS NULL`; soft-deleted records are
  invisible to search (see [postgresql](../postgresql/index.md) for the
  soft-delete convention).
- **Capped at 50** — the controller passes `limit = 50`; there is no
  client-supplied `limit`/`offset` yet (contrast §6).
- **Newest first** — `ORDER BY id DESC`; there is no relevance ranking
  (substring match is boolean, not scored).
- **Returns refs, not full records** — the handler maps rows to a slim
  `{pid, name}` / `{pid, title}` projection (`OrgRef` / `PathwayRef` /
  `CaseRef`), so callers fetch the full payload by pid in a second call.

### 1.2 Blank-query guard

A missing or whitespace-only `q` is rejected with **`400 Bad Request`**
at the controller, before any DB call — an empty term would `ILIKE` on
`%%` and return everything. (This is distinct from create-time
validation failures, which are `422`.)

```rust
// controllers/{organizations,care_pathways,cases}.rs — search handler
let q = params.q.unwrap_or_default();
if q.trim().is_empty() {
    return bad_request("query parameter `q` is required");
}
let rows = Model::search(&ctx.db, q.trim(), 50).await?;
```

### 1.3 `escape_like` — wildcard neutralisation [partial]

User input must be matched **literally**: the SQL `LIKE`/`ILIKE`
metacharacters `%` (any run), `_` (any single char), and the escape
char `\` are escaped so a user typing `100%` searches for the literal
string `100%`, not "anything starting with `100`". This is both a
**correctness** fix (no surprise wildcard matches) and a **defence-in-depth
security** measure (a user cannot smuggle wildcard semantics into the
pattern; SeaORM still parameterises the value, so this is not the SQLi
boundary — it is the wildcard-injection boundary).

```rust
// care-pathway-service & case-service: models/{care_pathways,cases}.rs
fn escape_like(q: &str) -> String {
    q.replace('\\', "\\\\")  // escape the escape char first
     .replace('%', "\\%")    // literal percent
     .replace('_', "\\_")    // literal underscore
}
```

Pinned by a unit test in each crate (`escape_like_neutralises_wildcards`):
`escape_like("100%") == "100\\%"`, `escape_like("a_b") == "a\\_b"`,
`escape_like("a\\%") == "a\\\\\\%"`.

> **Historical drift note (moot).** This subsection used to record that
> care-pathway-service and case-service applied `escape_like` while
> organization-service did not. That gap was never closed by porting
> `escape_like` into organization — it was closed by **removing** the
> `ILIKE` search method from organization entirely once Tantivy landed
> (2026-07-31): `organizations.rs` now issues no `LIKE` query at all
> (see the removal comment near line 261 of
> [`organization-service/src/models/organizations.rs`](../../organization/organization-service-with-loco/src/models/organizations.rs)),
> so there is no wildcard-injection surface left to have drift on.
> care-pathway and case still carry `escape_like` in their model files
> as unused dead code behind their own now-superseded `ILIKE` methods
> (§1.5) — nothing in the live `/search` request path calls them.

### 1.4 Why a denormalised column, not the JSONB payload

The full record is stored as a single JSONB `data` column (the matcher
type, verbatim). `ILIKE` over deep JSON would be both awkward to express
and unindexable in the simple case. Denormalising the one human-facing
label (`name`/`title`) to a top-level column kept list + search fast
before Tantivy landed. Richer search across payload fields is now the
live Tantivy behaviour (§2), not a future job.

### 1.5 Current per-crate reality (read this, not §1.1–§1.4, for "what ships today")

| Service | `GET /<plural>/search` backend today | What remains of the `ILIKE` code |
|---|---|---|
| organization | Tantivy (§2) | **Removed entirely** — no `LIKE` query issues from this crate any more. |
| care-pathway | Tantivy (§2) | `escape_like` + the model's `search`/`search_paged`/`search_count` remain in `src/models/care_pathways.rs`, unit-tested but with no live caller — dead code, not a fallback path. |
| case | Tantivy (§2) | Same shape as care-pathway: `escape_like` + the ILIKE model methods are unit-tested dead code in `src/models/cases.rs`. |
| portfolio | Tantivy (§2) | Migrated 2026-08-02 alongside case; see that crate's own model file for whether any ILIKE remnant remains. |

The **denormalised `name`/`title` column** itself is not obsolete —
`list`/`GET {pid}` still read it, and it is what Tantivy indexes from
— only the `ILIKE`-based **search query** against it has been
superseded.

---

## 2. Full-text search via Tantivy [implemented, all ten registries]

The upgrade beyond `ILIKE` is an embedded **Tantivy** full-text
index (see the stack note in
[`agents/share/loco.md`](../../agents/share/loco.md) and the dependency
table in
[`agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md)),
and it is **live on every entity registry today** — not a planned
upgrade for the four newest loco.rs services.

### 2.1 Where it is wired today

| Service tier | Search backend | Notes |
|---|---|---|
| Older Axum services (person, worker, place, thing, event, course — each with a `src/search/` module) | **Tantivy** [implemented] | `src/search/index.rs` declares the per-entity schema (indexed fields, analyzers), `src/search/mod.rs` is the `SearchEngine` wrapper, `src/search/query.rs` builds queries. |
| loco.rs services (organization, care-pathway, case, portfolio) | **Tantivy** [implemented] | Landed organization 2026-07-31, care-pathway 2026-08-01, case + portfolio 2026-08-02. `GET /<plural>/search` calls `crate::search::engine()` (fuzzy/phonetic/exact modes) exactly like the older services; organization additionally **removed** its prior `ILIKE` model method rather than leaving it dormant (§1). |

Every service's `GET /<plural>/search` handler now follows the same
shape: reject a blank `q` with `400`, resolve `limit`/`offset` (§6),
look up the Tantivy engine (`503` if the index is unavailable — an
operator must be able to tell a broken index from an empty result set),
select `Exact`/`Fuzzy`/`Phonetic` mode from the query params, and
resolve the returned pids back to rows.

The reference Tantivy implementation lives in
[`person-service/src/search/index.rs`](../../person/person-service-with-loco/src/search/index.rs):
a `PersonIndexSchema` declares ~11 indexed fields, choosing `TEXT`
(tokenised + lowercased, for fuzzy name search) vs `STRING` (verbatim,
for exact lookups like postal code / id) per field, plus a `FAST` field
for the active-flag filter. `PersonIndex` owns the on-disk index, a
cached schema, and a long-lived `IndexReader` reloaded after each commit.

### 2.2 Capabilities of the Tantivy tier

- **Full-text across many indexed fields** per entity (names, address
  components, identifier strings, …) rather than one denormalised label.
- **Fuzzy search** — edit-distance term queries (`FuzzyTermQuery`,
  distance 1–2) so `jonson` finds `johnson` (pinned by
  `test_fuzzy_search_typo`).
- **Phonetic search** — Soundex variants folded into name matching (§5).
- **Boolean query syntax** — `AND` / `OR` / `NOT`, e.g. an intersection
  of a name term and an exact birth-date term to disambiguate same-name
  records (`BooleanQuery::intersection`, pinned by
  `test_search_by_name_and_year_filter`).
- **Automatic index synchronisation** with DB writes — create/update
  re-indexes the doc, delete removes it by id term, then the reader
  reloads (§6).
- **Pagination** — `offset` + `limit` via `TopDocs` collector (§6).
- **Optional masking** of results before serialisation (§6, and
  [privacy](../../agents/share/privacy.md)).

### 2.3 Per-entity indexed fields

Each entity indexes the fields meaningful to it. Indicative set (the
person index is the worked example):

| Field class | Analyzer | Examples |
|---|---|---|
| Identity id | `STRING` (verbatim, stored) | record UUID |
| Name parts | `TEXT` (tokenised) | family name, given names, full name |
| Coded demographics | `STRING` | birth date `YYYY-MM-DD`, gender label |
| Address parts | mixed | postal code / state `STRING`; city `TEXT` |
| Identifier strings | `TEXT` | space-joined `type:value` |
| Active flag | `STRING` + `FAST` | `"true"`/`"false"` for filtering |

Other entities substitute their own labels (organization: legal name,
url/domain, jurisdiction; case: title, agency, case number; care-pathway:
name, condition codes), but the `TEXT`-for-fuzzy vs `STRING`-for-exact
split is the shared rule.

---

## 3. Geo-radius search [planned]

Entities that model coordinates (notably **place**; also any record with
a `GeoCoordinates` / lat-lon) should support **"find within distance of
a point"**. This is roadmap, not implemented.

- **Backend:** **PostGIS** (`postgis` extension, listed in
  [postgresql](../postgresql/index.md)) — store a `geography(Point)`,
  index it with GiST, and query `ST_DWithin(geom, point, metres)`.
- **Match-side counterpart:** the place matcher already scores geo
  proximity with **Haversine distance + sigmoid decay** (see
  [match](../../agents/share/match.md)); geo-radius search is the
  blocking-stage analogue that narrows candidates before that scorer
  runs (§4).
- Until PostGIS is wired, geographic narrowing falls back to the
  Tantivy text search (§2) already live on place — matching by name /
  postal code text, not by coordinate distance — rather than to `ILIKE`
  (place has never had an `ILIKE` search path; it is one of the
  original Tantivy-backed Axum services).

---

## 4. How search feeds matching and dedup

Search is the **blocking stage** of the match/dedup pipeline: it cheaply
narrows the corpus to a candidate set, then the matcher scores each
candidate pairwise. Search answers "which records are plausibly related";
matching answers "how confident are we, and on what evidence".

```
query record
   │
   ▼
search  ──►  candidate set        (Tantivy fuzzy/phonetic, all ten registries)
   │              │
   │              ▼
   │         matcher.rank / score  (per-component breakdown, 0.00–1.00)
   │              │
   ▼              ▼
 results     classify: certain / probable / possible / unlikely
```

- **Duplicate-check on demand** — `POST /<plural>/check-duplicates`
  pulls candidates and ranks them with the embedded matcher, returning
  scored hits without persisting. On **all ten registries** the
  candidates are now **search-blocked** (`index.candidates(&query,
  …)`, the same Tantivy engine as `/search`), not a full scan — landed
  on organization 2026-07-31, care-pathway 2026-08-01, case + portfolio
  2026-08-02, alongside their Tantivy migration. A `CHECK_DUPLICATES_*`
  cap still bounds the candidate count returned by the index, but it no
  longer bounds a row-by-row DB scan the way the pre-Tantivy comments in
  some of these crates still describe.
- **Real-time on create** — high-confidence candidates surface as a
  `409 Conflict` with the matches (the entity services that wire this).
- **Batch dedup** — `POST /<plural>/deduplicate` scans the corpus and
  queues review-queue items. Implemented on the MPI-lineage services and
  on **organization** (the only loco-lineage crate with this endpoint
  today); care-pathway, case, and portfolio do not yet expose it — see
  [`spec/merge/index.md`](../merge/index.md) §4, §9.

See [match-search-merge](../../agents/share/match-search-merge.md) for
the combined pipeline and the confidence classification table, and
[merge](../../agents/share/merge.md) for what happens to a confirmed
duplicate pair.

---

## 5. Phonetic search (Soundex)

Phonetic matching is integrated into **name search/matching**, not a
separate endpoint. A **4-character Soundex code** (first letter + three
digits) is computed for name terms; `Smith` and `Smyth` both encode to
`S530`, so a phonetic-enabled search treats them as candidates.

- In the **matcher**, Soundex is applied as a **+0.05 bonus** when the
  codes match and the current name score is `< 0.95` (so it nudges
  near-misses without overriding strong evidence) — see
  [match](../../agents/share/match.md) and the per-entity matching docs.
- In the **Tantivy tier** (§2) — now every entity registry's live
  `/search` — phonetic variants are folded into the name query so
  phonetically-equal names enter the candidate set.
- The historical **`ILIKE` tier** (§1) had **no** phonetic expansion —
  pure substring only. No live `/search` endpoint uses it any more.

---

## 6. Pagination, masking, index/DB synchronisation

### 6.1 Pagination

- **Historical `ILIKE` tier [superseded]:** server-side `LIMIT 50`,
  newest-first by id; no client `offset`/`limit`, no total count. No
  longer how any live `/search` endpoint behaves (§1.5).
- **Tantivy tier [implemented, all ten registries]:** client `limit` +
  `offset` via the `TopDocs` collector, on every entity's `/search`.
  The loco services additionally report the family-wide header contract
  (`X-Total-Count`/`X-Limit`/`X-Offset`) per
  [`agents/share/restful.md`](../../agents/share/restful.md)'s
  Pagination section — that document is the current source of truth for
  the limit-clamping (`MAX_LIMIT`) and bounded-offset (`400` beyond the
  bound) rules; this section only notes that search participates in it.

### 6.2 Result masking [implemented at record level; not applicable to search refs]

Per-field masking is **implemented** on organization, care-pathway, and
portfolio (`src/privacy.rs` + `GET /<plural>/{pid}/masked` + the ABAC
`mask` obligation on `GET /{pid}` and export — see
[`spec/privacy/index.md`](../privacy/index.md) for the full picture);
case has no dedicated masking module but does apply inline redaction on
its own read/export paths. This is a correction of an earlier claim
here that these were all "deferred" — masking exists, it is just not a
`/search` option.

The older Axum services expose `mask_sensitive=true` on search and a
`GET /<plural>/{id}/masked` view because their search can return
sensitive fields. **The Tantivy-backed loco `/search` endpoints have no
`mask_sensitive` parameter**, and that is a deliberate consequence of
§1.1/§2's ref-only response shape, not a masking gap: `/search` returns
only `{pid, name}` / `{pid, title}` (`OrgRef` / `PathwayRef` / …), which
carries no telephone, email, address, or coordinate field to mask in the
first place. Masking applies where the sensitive fields actually appear
— the single-record `GET /{pid}` (via the `mask` obligation), the
dedicated `/{pid}/masked` view, and export. See
[privacy](../../agents/share/privacy.md) for the masking model,
masked-view endpoint, and GDPR export.

### 6.3 Index ↔ DB synchronisation

- **Historical `ILIKE` tier:** the search column lived **in the same
  table/row** as the record, written transactionally on create/update;
  soft-delete set `deleted_at`, which the `IS NULL` filter honoured.
  There was no separate index to keep in sync — consistency was free.
  This was the reason the loco services started with `ILIKE` (§1) before
  taking on the cost below.
- **Tantivy tier [implemented, all ten registries]:** the on-disk index
  is a **second store** that must be kept in step with Postgres.
  Convention: create/update re-indexes the document, delete removes it
  by id term, then the `IndexReader` reloads to observe the committed
  segments (`reload`/`ReloadPolicy::OnCommitWithDelay`). A bulk re-index
  can call `optimize` to settle segment merges. Every service now pays
  this cost; the `ILIKE` tier's free-consistency property no longer
  applies to any live `/search` endpoint.

---

## 7. API surface

### 7.1 loco services (organization, care-pathway, case, portfolio) [implemented, Tantivy]

| Method | Path | Query params | Behaviour |
|---|---|---|---|
| GET | `/api/organizations/search` | `q` (required), `limit`, `offset`, `fuzzy`, `phonetic` | Tantivy full-text over indexed fields, active rows → `[{pid, name}]`, `X-Total-Count`/`X-Limit`/`X-Offset` headers |
| GET | `/api/care-pathways/search` | `q` (required), `limit`, `offset`, `fuzzy`, `phonetic` | Same shape → `[{pid, name}]` |
| GET | `/api/cases/search` | `q` (required), `limit`, `offset`, `fuzzy`, `phonetic` | Same shape → `[{pid, title}]` |
| GET | `/api/plans/search` (portfolio) | `q` (required), `limit`, `offset`, `fuzzy`, `phonetic`, `kind` | Same shape; `kind` is a search **filter**, never a matching gate |

- Blank/absent `q` → **`400`** (§1.2).
- No `mask_sensitive` param (§6.2) — the response shape carries nothing
  to mask.
- Documented in each crate's hand-written OpenAPI (`src/openapi.rs`),
  surfaced at `/swagger-ui` + `/api-docs/openapi.json`.
- This table **replaces** an earlier version describing these four as
  `ILIKE`-only with no `limit`/`offset`/`fuzzy`/`phonetic` — see §1.5.

### 7.2 Older Axum services (person and the original entity services) [implemented, Tantivy]

| Method | Path | Query params |
|---|---|---|
| GET | `/api/persons/search` | `q`, `limit` (default 10, max 100), `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool) |

Other Axum entity services (worker, place, thing, event, course) mirror
this shape under their own plural path.

### 7.3 Convergence — reached

The target end-state described here used to be a *future* single search
contract across all entities. It has **landed**:
`GET /<plural>/search?q=&limit=&offset=&fuzzy=&phonetic=`, backed by
Tantivy, on every one of the ten entity registries (§7.1, §7.2); the
`mask_sensitive` param remains specific to the older Axum services'
richer response shape (§6.2), and portfolio additionally accepts `kind`
as a filter. The `ILIKE` implementation is no longer a fallback for any
crate — it is historical (§1.5). Consult each crate's own `spec/§13` /
`CHANGELOG.md` only for entity-specific detail (indexed field lists,
per-entity search params); the contract itself is no longer in flux.

---

## 8. Summary: implemented, family-wide

| Capability | loco services (org / care-pathway / case / portfolio) | Axum services (person, …) |
|---|---|---|
| Historical substring `ILIKE` name/title search | superseded by Tantivy (§1.5) | never had it — Tantivy from the start |
| `escape_like` wildcard neutralisation | dead code in care-pathway/case (unit-tested, uncalled); removed entirely in organization (§1.3) | n/a |
| Tantivy full-text | **[implemented]** — organization 2026-07-31, care-pathway 2026-08-01, case + portfolio 2026-08-02 | **[implemented]** |
| Fuzzy (edit-distance) | **[implemented]** | **[implemented]** |
| Phonetic (Soundex) search | **[implemented]** | **[implemented]** |
| Boolean AND/OR/NOT | **[implemented]** (via the shared Tantivy query layer) | **[implemented]** |
| Client `limit`/`offset` pagination + `X-Total-Count`/`X-Limit`/`X-Offset` headers | **[implemented]** | **[implemented]** |
| Result masking | n/a — search returns refs with no sensitive fields (§6.2); masking lives on `GET /{pid}` / `/{pid}/masked` / export instead, and is **[implemented]** there for organization, care-pathway, portfolio (case: inline, no dedicated module) | **[implemented]** (`mask_sensitive` on search + `/{id}/masked`) |
| Geo-radius (PostGIS) | [planned] | [planned] |
| Search-blocked candidates for `check-duplicates` | **[implemented]**, all four (§4) | **[implemented]** |
| Batch `/deduplicate` scan endpoint | **[implemented]** in organization only; not yet in care-pathway/case/portfolio (§4, [`spec/merge/index.md`](../merge/index.md) §9) | **[implemented]** |
