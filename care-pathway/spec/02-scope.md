## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The DTO contract: the API request/response body **is**
  `care_pathway_matcher::CarePathway`, stored verbatim as JSONB
  (§5). There is no separate service model and no adapter.
- The service ↔ front-end wire contract: raw loco JSON (no
  envelope), TypeScript type mirroring (§5.4).
- Shared invariants that more than one subproject must uphold (§5.5).
- Entity-wide goals: national-health-system scale, multi-locale,
  auditability, healthcare-compliance posture (§7, §12).

### 2.2 In scope — per subproject

**care-pathway-service-with-loco** owns:

- Care-pathway CRUD with soft delete (`deleted_at`).
- `POST …/match` (rank an explicit candidate set, no persistence)
  and `POST …/check-duplicates` (match a query against stored
  pathways).
- One `care_pathways` table: `pid` + denormalised `name` + the full
  `CarePathway` JSONB `data`.
- loco.rs app structure, migrations, configuration.

**care-pathway-matcher-rust-crate** owns:

- Pure-library pairwise comparison: deterministic short-circuits
  (DOI / Wikidata / guideline-id / URI / UUID, same-provider pathway
  code, `same_as` URL overlap) + weighted probabilistic scoring
  (name, condition codes, pathway code, care setting, interventions,
  keywords) with per-component breakdown.
- The `CarePathway` domain type and its supporting enums — the
  entity's canonical DTO.
- Normalisation (`fold`, pathway-code, set folding), Soundex bonus,
  config presets (`strict` / `default` / `lenient`).

**care-pathway-front-end-with-svelte** owns:

- Operator routes: list (`/`), create (`/new`), detail + delete +
  check-duplicates (`/[pid]`), edit (`/[pid]/edit`).
- Its own copy of API types, client, and form primitives (drift
  between front-ends is accepted — repo decision 2026-06-02).

### 2.3 Out of scope (today) — MVP deferrals

The entity is an **MVP**: CRUD + matching only. Explicitly deferred,
tracked in §13 / §15 and the crate specs' §13:

- Full-text search (Tantivy) and the front-end search box.
- Event streaming and audit logging on CRUD.
- Privacy controls (masking, GDPR export, consent model).
- Record merge with link tracking and transferred-data snapshots.
- Real-time duplicate detection on create (`409 Conflict`).
- OpenAPI / Swagger, gRPC, richer validation (ICD / SNOMED code
  formats).
- Token verification middleware — PASETO v4 public, per
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (SSO provider exists in the [authentication entity](../../authentication/)).
- Request-level integration tests against PostgreSQL.
