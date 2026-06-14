# Search

Monorepo-wide reference for how the **Main X Index** family does record
search. This is the comprehensive spec; the short version lives at
[`agents/share/search.md`](../../agents/share/search.md). Each service
owns its own database and its own search surface — there is no shared
search service — but they all follow the conventions below.

> Status legend used throughout: **[implemented]** ships today,
> **[planned]** is the intended upgrade not yet wired, **[partial]**
> exists in some crates only.

> Related topics: [postgresql](../postgresql/index.md) ·
> [match (short)](../../agents/share/match.md) ·
> [merge (short)](../../agents/share/merge.md) ·
> [privacy (short)](../../agents/share/privacy.md) ·
> [match-search-merge (combined)](../../agents/share/match-search-merge.md).
> Per-entity specs:
> [organization](../../organization/organization-service-rust-crate/spec/index.md) ·
> [care-pathway](../../care-pathway/care-pathway-service-rust-crate/spec/index.md) ·
> [case](../../case/case-service-rust-crate/spec/index.md) ·
> [person](../../person/person-service-rust-crate/spec/index.md).

---

## 1. Current state — Postgres `ILIKE` substring search [implemented]

The **loco.rs** services (organization, care-pathway, case — and the
other entities converted to the loco shape) do **pragmatic Postgres
`ILIKE '%q%'` substring search** over a single **denormalised scalar
column**, not over the JSONB payload. There is no Tantivy in these
crates yet; the full-text engine is the planned upgrade (§2).

### 1.1 What the query does

The search column is the entity's denormalised name/title, populated on
every create/update alongside the JSONB `data` payload:

| Service | Search column | Source file (model `search`) |
|---|---|---|
| organization-service | `name` | [`src/models/organizations.rs`](../../organization/organization-service-rust-crate/src/models/organizations.rs) |
| care-pathway-service | `name` | [`src/models/care_pathways.rs`](../../care-pathway/care-pathway-service-rust-crate/src/models/care_pathways.rs) |
| case-service | `title` | [`src/models/cases.rs`](../../case/case-service-rust-crate/src/models/cases.rs) |

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

> **Known drift:** care-pathway-service and case-service apply
> `escape_like`; **organization-service does not** — its `search` builds
> the pattern as a raw `format!("%{q}%")`, so a `%`/`_` in an
> organization query is still treated as a wildcard. Closing that gap
> (port `escape_like` into
> [`organization-service/src/models/organizations.rs`](../../organization/organization-service-rust-crate/src/models/organizations.rs))
> is a tracked clean-up.

### 1.4 Why a denormalised column, not the JSONB payload

The full record is stored as a single JSONB `data` column (the matcher
type, verbatim). `ILIKE` over deep JSON would be both awkward to express
and unindexable in the simple case. Denormalising the one human-facing
label (`name`/`title`) to a top-level column keeps list + search fast
and lets a future `pg_trgm` GIN index (see
[postgresql](../postgresql/index.md)) accelerate the substring scan
without touching the payload. Richer search across payload fields is the
Tantivy job (§2).

---

## 2. Full-text search via Tantivy [partial / planned]

The intended upgrade beyond `ILIKE` is an embedded **Tantivy** full-text
index (see the stack note in
[`agents/share/loco.md`](../../agents/share/loco.md) and the dependency
table in
[`agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md)).

### 2.1 Where it is wired today

| Service tier | Search backend | Notes |
|---|---|---|
| Older Axum services (person, and the original entity services with a `src/search/` module) | **Tantivy** [implemented] | `src/search/index.rs` declares the per-entity schema (indexed fields, analyzers), `src/search/mod.rs` is the `SearchEngine` wrapper, `src/search/query.rs` builds queries. |
| loco.rs services (organization, care-pathway, case) | **Postgres `ILIKE`** [implemented]; Tantivy [planned] | Deferred in each crate's spec §13 (e.g. care-pathway/case task **T-6**, "full-text / fuzzy search is deferred"). |

The reference Tantivy implementation lives in
[`person-service/src/search/index.rs`](../../person/person-service-rust-crate/src/search/index.rs):
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
- Until PostGIS is wired, geographic narrowing falls back to `ILIKE` on
  the denormalised place name / postal code.

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
search  ──►  candidate set        (ILIKE name/title today; Tantivy fuzzy/phonetic planned)
   │              │
   │              ▼
   │         matcher.rank / score  (per-component breakdown, 0.00–1.00)
   │              │
   ▼              ▼
 results     classify: certain / probable / possible / unlikely
```

- **Duplicate-check on demand** — `POST /<plural>/check-duplicates`
  pulls candidates (today: a capped scan / name search) and ranks them
  with the embedded matcher, returning scored hits without persisting.
- **Real-time on create** — high-confidence candidates surface as a
  `409 Conflict` with the matches (the entity services that wire this).
- **Batch dedup** — `POST /<plural>/deduplicate` scans the corpus and
  queues review-queue items.

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
- In the **Tantivy tier** (§2), phonetic variants are folded into the
  name query so phonetically-equal names enter the candidate set.
- In the **`ILIKE` tier** (§1) there is **no** phonetic expansion — pure
  substring only. Phonetic search arrives with Tantivy.

---

## 6. Pagination, masking, index/DB synchronisation

### 6.1 Pagination

- **`ILIKE` tier [implemented, fixed cap]:** server-side `LIMIT 50`,
  newest-first by id; no client `offset`/`limit`, no total count.
- **Tantivy tier [implemented in the Axum services]:** client `limit`
  (default 10, max 100) + `offset` via the `TopDocs` collector.

### 6.2 Result masking [planned for loco services]

Search may **mask sensitive fields** before returning results (coordinates,
telephone, email, postal address). The older Axum services expose
`mask_sensitive=true` on search and a `GET /<plural>/{id}/masked` view;
the loco services have **not** wired privacy/masking yet (deferred per
their spec §13). See [privacy](../../agents/share/privacy.md) for the
masking model, masked-view endpoint, and GDPR export.

### 6.3 Index ↔ DB synchronisation

- **`ILIKE` tier:** the search column lives **in the same table/row** as
  the record, written transactionally on create/update; soft-delete sets
  `deleted_at`, which the `IS NULL` filter honours. There is no separate
  index to keep in sync — consistency is free.
- **Tantivy tier:** the on-disk index is a **second store** that must be
  kept in step with Postgres. Convention: create/update re-indexes the
  document, delete removes it by id term, then the `IndexReader` reloads
  to observe the committed segments (`reload`/`ReloadPolicy::OnCommitWithDelay`).
  A bulk re-index can call `optimize` to settle segment merges. This is
  the cost the `ILIKE` approach avoids — and the reason the loco services
  start with `ILIKE`.

---

## 7. API surface

### 7.1 `ILIKE` services (organization, care-pathway, case) [implemented]

| Method | Path | Query params | Behaviour |
|---|---|---|---|
| GET | `/api/organizations/search` | `q` (required) | `ILIKE '%q%'` over `name`, active rows, cap 50, newest first → `[{pid, name}]` |
| GET | `/api/care-pathways/search` | `q` (required) | `ILIKE '%q%'` over `name`, active rows, cap 50 → `[{pid, name}]` |
| GET | `/api/cases/search` | `q` (required) | `ILIKE '%q%'` over `title`, active rows, cap 50 → `[{pid, title}]` |

- Blank/absent `q` → **`400`** (§1.2).
- No `limit` / `offset` / `fuzzy` / `phonetic` / `mask_sensitive` params
  yet — those belong to the Tantivy tier.
- Documented in each crate's hand-written OpenAPI (`src/openapi.rs`),
  surfaced at `/swagger-ui` + `/api-docs/openapi.json`.

### 7.2 Tantivy services (person and the original Axum entity services) [implemented]

| Method | Path | Query params |
|---|---|---|
| GET | `/api/persons/search` | `q`, `limit` (default 10, max 100), `offset`, `fuzzy` (bool), `phonetic` (bool), `mask_sensitive` (bool) |

Other Axum entity services mirror this shape under their own plural path.

### 7.3 Planned convergence

The target end-state is a single search contract across all entities:
`GET /<plural>/search?q=&limit=&offset=&fuzzy=&phonetic=&mask_sensitive=`,
backed by Tantivy, with the `ILIKE` implementation as the documented
fallback for crates that have not yet migrated. Until then, treat the
two tiers above as the source of truth per crate, and consult the
crate's own `spec.md §13` for its migration status.

---

## 8. Summary: implemented vs planned

| Capability | loco services (org / care-pathway / case) | Axum services (person, …) |
|---|---|---|
| Substring `ILIKE` name/title search | **[implemented]** | superseded by Tantivy |
| Active-rows-only, cap 50, blank-q `400` | **[implemented]** | — |
| `escape_like` wildcard neutralisation | **[implemented]** (org: **[planned]** — drift, §1.3) | n/a |
| Tantivy full-text | [planned] (spec §13 / T-6) | **[implemented]** |
| Fuzzy (edit-distance) | [planned] | **[implemented]** |
| Phonetic (Soundex) search | [planned] | **[implemented]** |
| Boolean AND/OR/NOT | [planned] | **[implemented]** |
| Client `limit`/`offset` pagination | [planned] | **[implemented]** |
| Result masking | [planned] | **[implemented]** |
| Geo-radius (PostGIS) | [planned] | [planned] |
| Search → matcher blocking for dedup | **[implemented]** (scan/name-search candidates) | **[implemented]** |
