## 8. Architecture

### 8.1 Trio composition

```
+--------------------------------------------------------------+
|              organization-front-end-with-svelte               |
|  SvelteKit 2 SPA (Svelte 5 runes, TS strict, no data grid)    |
|  routes: /  /new  /[pid]  /[pid]/edit                         |
|  lib/api: client.ts -> organizations.ts (repository)          |
+------------------------------+--------------------------------+
                               | HTTP, raw JSON (no envelope)
                               | PUBLIC_API_BASE_URL (default :5150)
+------------------------------v--------------------------------+
|               organization-service-with-loco                 |
|  loco.rs 0.16 app (Axum 0.8)                                  |
|  +----------------------+  +---------------------------+      |
|  | controllers/         |  | controllers/docs.rs       |      |
|  | organizations.rs     |  | /api-docs/openapi.json    |      |
|  | CRUD+search+match+   |  | /swagger-ui               |      |
|  | dup+audit+events     |  +---------------------------+      |
|  +----+--------+--------+                                     |
|       |        |     +-----------------+  +----------------+  |
|       |        +---->| models/audit_   |  | streaming.rs   |  |
|       |              | logs.rs         |  | in-mem ring    |  |
|       v              +-----------------+  | buffer (1000)  |  |
|  +----------------------+                 +----------------+  |
|  | models/              |                                     |
|  | organizations.rs     |--- embeds, calls directly --------+ |
|  +----------+-----------+                                   | |
+-------------|-----------------------------------------------|-+
              |                                               |
+-------------v---------------+   +---------------------------v--+
|  PostgreSQL (SeaORM 1.1)    |   |  organization-matcher        |
|  organizations  (JSONB data)|   |  pure library: MatchingEngine|
|  audit_logs     (snapshots) |   |  no IO, no unsafe, no panics |
+-----------------------------+   +------------------------------+
```

### 8.2 Dependency direction

Strictly one-way; the matcher is the leaf:

- `organization-front-end-with-svelte` → service REST API (HTTP only;
  no shared code — drift policy).
- `organization-service` → `organization-matcher` (Cargo path
  dependency; the matcher type is re-used as the DTO, called directly
  on deserialised payloads — **no adapter**).
- `organization-matcher` → nothing but `serde` + small string/Unicode
  helpers. It MUST NOT acquire IO, async, logging, or service
  dependencies.

### 8.3 Service (loco.rs) structure

```
organization-service-with-loco/
├── src/
│   ├── app.rs                      loco Hooks: registers organizations + docs routes
│   ├── bin/main.rs                 loco CLI entrypoint (`cargo loco start`)
│   ├── controllers/
│   │   ├── organizations.rs        CRUD + search + match + check-duplicates + audit + events
│   │   └── docs.rs                 OpenAPI JSON + Swagger UI
│   ├── models/
│   │   ├── organizations.rs        CRUD helpers over the JSONB payload (create/find_by_pid/search/list/…)
│   │   ├── audit_logs.rs           record / recent / for_entity
│   │   └── _entities/              SeaORM entities
│   ├── openapi.rs                  hand-written OpenAPI 3 document
│   └── streaming.rs                in-memory OrgEvent ring buffer (OnceLock global)
├── migration/src/                  m…_organizations, m…_audit_logs
└── config/                         development / production / test yaml (auto_migrate in dev)
```

Note: the loco scaffolding leftovers (`src/workers/downloader.rs`,
empty `src/data/` + `src/tasks/`) were removed 2026-06-13 (§13 T-12);
the app registers no background workers.

### 8.4 Data flows

**Create:** HTTP POST → name-required guard (`422` if blank) →
`OrgModel::create`
(serialize payload → INSERT) → audit `created` (best-effort, with
snapshot) → publish `Created` event → `{pid, name}` response.

**Check-duplicates:** HTTP POST → load active rows (cap 1 000) →
deserialise each `data` payload → `MatchingEngine::match_organizations`
per candidate → keep `is_match`, sort by score desc → response.

**Match:** HTTP POST `{query, candidates}` → `MatchingEngine::rank`
→ ranked results. Stateless; no DB access.

### 8.5 Deployment topology

**Today:** one service instance + one PostgreSQL database; front-end
served as static SPA assets; Swagger UI for exploration. Suitable for
development and pilot registries.

**Roadmap (§15, aspirational):** stateless service replicas behind a
load balancer per region; PostgreSQL with replication and
read-replicas; durable event bus replacing the in-memory buffer;
externalised search; PASETO v4.public tokens from the central
authentication service, verified offline against its published Ed25519
key (see
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
superseding RS256-JWT + JWKS). The service is already stateless **except**
for the in-memory event buffer — replacing it is the gating item for
horizontal scale-out.

### 8.6 Cross-service entity linking — organization is a link **target** (v1)

Cross-service typed edges between records in *different* services are
defined family-wide in
[`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
(the hybrid topology: services own outbound edges locally, a standalone
read-model aggregator builds the queryable graph). In the v1 edge-kind
registry (that doc §9) every edge touching an organization points
**inbound** at it — `person → organization` (`works_at` / `member_of`)
and `worker → organization` (`employed_by`) — so the organization is
always the `to` side. Consequences for this service:

- **No `entity_links` write-side table in v1.** No outbound edge
  *originates* from an organization yet, so this service stores no
  cross-service edges and exposes no `/links` write surface. (Contrast
  person / worker, which get the table in the backbone phase — that doc
  §11.)
- **It participates as a target two ways.** (a) Its existing
  `created` / `deleted` / `merged` events feed the aggregator's
  `entity_presence` verification oracle and its merge-repointing handler
  (that doc §5, §5.3): a deleted organization flips inbound edges to
  `dangling`; a merge repoints inbound edges from `merged_from` to the
  survivor centrally. (b) Each organization record is addressable as an
  `EntityRef` URN `organization:<pid>` (that doc §3), so inbound edges
  resolve to it.
- **The inverse edges are read-model only.** The materialised inverses
  (`has_member`, `employs`) live in the aggregator's bidirectional
  `edges` table, **not** in this service. The organization service stores
  no representation of who links to it.
- **Roadmap.** If operators later need to assert affiliations *from* the
  organization side (the org originating an outbound edge), that is a new
  registry row + write-side table — tracked in §15.

### 8.7 Bulk import / export

The family-wide contract — async `bg_pg` jobs, the `bulk_jobs` table, the
five endpoints, JSONL/CSV/Parquet, idempotent upsert-by-key, the per-row
error report, and the export privacy/audit posture — is fixed once in
[`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md)
and is **not** restated here. Per that doc §10, the organization entity
declares only what differs.

**Stable key(s) (upsert key, import §6).** A row upserts in place when it
carries a **deterministic, globally-unique scheme-scoped identifier** that
the matcher already short-circuits on (§5.2, class "Deterministic"):
`Lei`, `Duns`, `Iso6523`, `Gln`, `Wikidata`, `Ror`, `Isni`, `Vat`. Each is
matched **within its scheme** as an `(scheme, value)` pair. The record
`pid` is also a stable key when present (a re-export → re-import round-trip
upserts by `pid`). Jurisdiction-scoped `TaxId` upserts only together with a
matching `jurisdiction` (R-1); classification codes (`Naics`, `IsicV4`,
`Sic`) and `Custom(_)` are **never** upsert keys. A keyless row (no
deterministic identifier, no `pid`) runs normal duplicate detection
(`check-duplicates` path) and routes likely duplicates to the review queue
with `provenance = import` (shared doc §6).

**CSV column set + flattening (§5 of the shared doc).** JSONL is the
lossless reference (one `Organization` wire record per line). The CSV
projection is:

| CSV column(s) | Source field | Encoding |
|---|---|---|
| `pid` | `pid` (read-only; export + upsert key) | scalar |
| `name` | `name` (required) | scalar |
| `legal_name` | `legal_name` | scalar |
| `url` | `url` | scalar |
| `jurisdiction` | `jurisdiction` (ISO 3166) | scalar |
| `founding_date` | `founding_date` (ISO-8601) | scalar |
| `telephone` | `telephone` | scalar |
| `email` | `email` | scalar |
| `address.street_address`, `address.locality`, `address.region`, `address.postal_code`, `address.country` | `address` (single nested object) | **dotted columns** |
| `identifiers` | `identifiers[]` (`{scheme, value}`) | **JSON-encoded cell** |
| `alternate_names` | `alternate_names[]` | **JSON-encoded cell** |
| `same_as` | `same_as[]` | **JSON-encoded cell** |
| `keywords` | `keywords[]` | **JSON-encoded cell** |
| `tags` | `tags[]` | **JSON-encoded cell** |
| `relationships` | `relationships[]` (`{relation, organization_id}`) | **JSON-encoded cell** |

CSV is lossy for the JSON-encoded arrays (deep nesting collapses to a
string cell); fidelity-sensitive loads use JSONL. Parquet uses the same
logical schema, export-first in v1.

**Export sensitivity.** Organization data is **largely public / low–medium
sensitivity** (register identity is open; §12). The default
`masking_profile` is therefore **light**: the only personal-data risk is
the contact fields and sole-trader records, so masked export protects
`telephone` / `email` (and sole-trader records) exactly as the §13 T-5
single-record masking does — full/unmasked export of those requires
elevated authorisation, and `include_soft_deleted` is gated. There is no
organization-specific escalation beyond the family default. Every export is
audited (actor, filter, format, row count, masking profile) per the shared
contract, even a zero-row export.
