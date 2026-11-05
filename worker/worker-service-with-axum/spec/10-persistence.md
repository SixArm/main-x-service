## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables

`workers`, `worker_names`, `worker_identifiers`, `worker_addresses`,
`worker_contacts`, `worker_links`, `organizations`,
`organization_addresses`, `organization_contacts`,
`organization_identifiers`, `worker_match_scores`, `entity_links`,
`worker_assessments`, `review_queue`, `event_outbox`, `audit_log`.

> `worker_links` holds **within-entity** links to other worker records
> (a matcher signal). `entity_links` (§10.3) holds **cross-service**
> outbound edges (never a matcher signal). They are separate tables by
> the §5.1 partition rule.

> **`workers.gender` casing.** The column carries a CHECK constraint
> admitting only `'male' | 'female' | 'other' | 'unknown'`. The
> repository writes the lowercased enum token (see §13 BUG-1); the read
> path lowercases before parsing, so a legacy PascalCase row still
> round-trips. `m20260723_000002_normalize_worker_gender_case` is the
> one-way data migration that repairs such rows on a deployment whose
> table was created without the constraint.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`.

### 10.3 `entity_links` (cross-service write side)

Worker's outbound cross-service edges (domain model §5.4). Per the shared
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md)
schema. The far endpoint is an opaque `EntityRef` URN in `to_ref` — there
is **no** foreign key across services; `from_pid` is a local FK to
`workers`.

```sql
CREATE TABLE entity_links (
    id           UUID PRIMARY KEY,
    from_pid     UUID NOT NULL,          -- this worker (FK, local)
    kind         TEXT NOT NULL,          -- 'same_identity' | 'employed_by'
    to_ref       TEXT NOT NULL,          -- EntityRef URN: 'person:…' | 'organization:…'
    role         TEXT,                   -- job title (employed_by)
    confidence   DOUBLE PRECISION,       -- 1.0 operator-asserted; <1 suggested
    provenance   TEXT NOT NULL,          -- operator | import | matcher_suggested
    valid_from   DATE,                   -- affiliation start (nullable)
    valid_to     DATE,                   -- affiliation end ("former …")
    created_at   TIMESTAMPTZ NOT NULL,
    deleted_at   TIMESTAMPTZ,            -- soft-delete (withdrawn edge)
    UNIQUE (from_pid, kind, to_ref, valid_from)   -- idempotent upsert key
);
```

### 10.4 Bulk import / export

Worker adopts the uniform bulk import/export contract — async `bg_pg`
jobs, JSONL/CSV/Parquet formats, idempotent upsert-by-key import,
keyless-row → duplicate-detection → review-queue routing, per-row error
report, and masked-by-default audited export — fixed verbatim in
[`bulk import / export`](../../../agents/share/bulk-import-export.md)
(the `bulk_jobs` table §3, the five endpoints §4, the formats §5, the
import semantics §6, the error contract §7, the export privacy/audit
posture §8). This section declares **only what Worker differs on**
(shared doc §10); nothing else is restated.

**Stable key(s) (upsert).** Per shared §6/§10, import upserts in place
when a declared stable key matches an existing record; otherwise the row
runs normal duplicate detection (§6 → review queue, `provenance =
import`). Worker's stable keys, in order:

1. **A scheme-scoped person-level identifier** — the `(identifier_type,
   system, value)` triple of any `Identifier` whose scheme is one the
   worker-matcher short-circuits on deterministically: one of the 42
   national person-identifier schemes (e.g. `SSN`, `NPI`) or `tax_id`
   (the `TAX`-type identifier surfaced by `effective_tax_id`). These are
   the *person-level* schemes only; **organisation-level codes (NHS ODS,
   GLN, employer/department codes) are NOT stable keys** — two workers at
   one practice share them, so upsert-by-key on them would merge
   colleagues (worker-matcher scope §2; matcher partition mirrors §5.1).
2. **The record `pid`** (the worker UUID) when a row carries one — an
   exact re-export → re-import round-trip.

A row with neither a known person-level identifier nor a `pid` is
**keyless**: it takes the duplicate-detection path, never a silent
create-on-collision.

**CSV column set + flattening** (per shared §5; JSONL remains the
lossless reference — prefer it when fidelity matters):

- **Scalar columns** (one each): `pid`, `active`, `name.use_type`,
  `name.family`, `name.given` (space-joined; JSON-encoded if any given
  name contains a space), `name.prefix`, `name.suffix`, `gender`,
  `birth_date`, `tax_id`, `marital_status`, `multiple_birth`,
  `deceased`, `deceased_datetime`, `managing_organization`,
  `created_at`, `updated_at`.
- **Single nested object → dotted columns** — the primary address:
  `address.line1`, `address.line2`, `address.city`, `address.state`,
  `address.postal_code`, `address.country`.
- **Arrays / arrays-of-objects → one JSON-encoded cell each**:
  `identifiers`, `additional_names`, `telecom`, `addresses` (the full
  list; the dotted `address.*` columns mirror only the primary),
  `documents`, `emergency_contacts`, `photo`, `links` (within-entity
  `WorkerLink`), and `entity_links` (cross-service edges, §10.3 —
  importable per shared §9, same `provenance = import` + the
  `UNIQUE (from_pid, kind, to_ref, valid_from)` upsert key).

**Export sensitivity.** Worker is **personal data** (demographics, DOB,
national identifiers, credentials, emergency contacts — compliance §12,
[privacy](../../../agents/share/privacy.md)). Beyond the shared default:

- Export is **masked by default**; the `masking_profile` matches the
  single-record read/masking rules. A **full / unmasked** bulk export
  requires **elevated authorisation** (HR-admin / credentialing-officer,
  not read-only; API §9 roles) and may never reveal more than the caller
  could read one record at a time.
- `include_soft_deleted` defaults `false` and is gated.
- **Every export is audited** — actor, filter, format, row count,
  masking profile, timestamp — written even for a zero-row export, since
  a bulk extract of worker personal data is itself a HIPAA / GDPR
  compliance event. The single-subject GDPR export is the `filter = one
  pid` special case of this machinery.

### 10.5 `worker_assessments` (aptitude / personality / psychometric / selection)

One row per administration of one instrument to one worker (domain model
§5.5). The per-scale outcomes ride in a `results` JSONB array rather
than a child table: an assessment is always read and written whole,
never queried field-by-field, so a child table would buy nothing and
cost a join on every read.

```sql
CREATE TABLE worker_assessments (
    id              UUID PRIMARY KEY,
    worker_id       UUID NOT NULL,        -- the assessed worker
    category        VARCHAR NOT NULL,     -- aptitude | personality | psychometric | selection
    instrument      VARCHAR NOT NULL,     -- the named test
    provider        VARCHAR,              -- publisher / administering provider
    status          VARCHAR NOT NULL DEFAULT 'scheduled',
    administered_on DATE,
    expires_on      DATE,                 -- results stop counting as current
    administered_by VARCHAR,
    notes           VARCHAR,              -- sensitive: redacted under `mask`
    results         JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ           -- soft-delete (withdrawn record)
);
CREATE INDEX worker_assessments_worker_idx
    ON worker_assessments (worker_id, administered_on DESC) WHERE deleted_at IS NULL;
CREATE INDEX worker_assessments_category_idx
    ON worker_assessments (category) WHERE deleted_at IS NULL;
```

`category` and `status` store the domain enums' lowercase wire tokens
(as the family does elsewhere), so the vocabulary can grow by data
migration rather than DDL. A stored token or `results` payload that has
drifted outside the domain vocabulary is surfaced as a `500` with a
message by `db::assessments::to_domain` — never a panic (security
invariant 2).

Migration: `m20260723_000001_create_worker_assessments`
(`migrations/2026072300000001_create_worker_assessments/{up,down}.sql`).
