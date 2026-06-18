## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The DTO contract: `organization_matcher::Organization` is the API
  body, the JSONB-persisted payload, and the matching input (§5).
- The service ↔ front-end wire contract: raw loco JSON, no response
  envelope; snake_case field names (§5.4, §9).
- Shared invariants that more than one subproject must uphold (§5.5).
- Entity-wide goals: register scale, multi-jurisdiction,
  auditability, privacy compliance (§7, §12).

### 2.2 In scope — per subproject

**organization-service-with-loco** owns:

- Organization CRUD with soft delete (`deleted_at` stamp) — loco.rs
  controllers over one `organizations` table.
- Name search (PostgreSQL `ILIKE`, capped 50).
- `/match` (rank an explicit candidate set) and `/check-duplicates`
  (match a query against stored records) via the embedded matcher.
- Audit log (`audit_logs` table: action + snapshot per CRUD).
- Event streaming (in-memory ring buffer, MVP of the family's layer).
- Hand-written OpenAPI 3 document + Swagger UI.

**organization-matcher-rust-crate** owns:

- Pure-library pairwise comparison: three deterministic short-circuit
  rules (R-0 deterministic identifier, R-1 same-jurisdiction tax ID,
  R-2 `same_as` URL overlap) + weighted probabilistic scoring with
  per-component breakdown.
- Probabilistic components: legal-suffix-aware name, postal address,
  URL/domain, jurisdiction, founding date, keywords.
- Normalisation (`fold`, `legal_name`, `domain`, `fold_set`),
  Soundex, `MatchConfig` presets (`strict` / default / `lenient`).

**organization-front-end-with-svelte** owns:

- Operator routes: list, create, detail + delete + check-duplicates,
  edit.
- Its own copy of API types, client, and form (drift between
  front-ends is accepted — repo decision 2026-06-02). Deliberately
  dependency-light: no data grid, no design system.

### 2.3 Out of scope (today)

The entity is an **MVP plus recent additions** (name search,
OpenAPI/Swagger, audit log, event streaming landed; see §14). Still
deferred, tracked in §13 / §15:

- Privacy layer: per-field masking, GDPR export endpoint, consent
  model (service crate spec §13).
- Record merging with link tracking; duplicate review queue; batch
  deduplication.
- Tantivy full-text / fuzzy search (today: `ILIKE` name search only).
- Real-time `409` duplicate detection on create (today: explicit
  `/check-duplicates` only — open question, §16).
- PASETO v4.public verification middleware (auth-service published
  Ed25519 key) and front-end auth wiring — see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (supersedes the prior RS256-JWT + JWKS model).
- Durable event bus (today: in-memory ring buffer, capacity 1 000).
- Richer validation (identifier formats, URL, country codes), gRPC,
  multi-region deployment, register-feed ingestion.
