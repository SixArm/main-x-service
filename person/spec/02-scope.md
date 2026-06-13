## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The service ↔ matcher DTO contract (the adapter projection, §5.3).
- The service ↔ front-end wire contract (response envelope, TypeScript
  type mirroring, §5.4).
- Shared invariants that more than one subproject must uphold (§5.5).
- Entity-wide goals: population scale, multi-locale, auditability,
  privacy compliance (§7, §12).

### 2.2 In scope — per subproject

**person-service-rust-crate** owns:

- Person CRUD with soft delete and full audit trail.
- Identifiers, identity documents, addresses, telecom, emergency
  contacts.
- Probabilistic + deterministic matching (in-service algorithms plus
  the embedded canonical matcher).
- Tantivy full-text / fuzzy / phonetic search.
- Real-time + explicit + batch duplicate detection, review queue,
  merge.
- Validation, normalisation, privacy masking, GDPR export, consent.
- REST API + FHIR R5 Person + gRPC stub; PostgreSQL via SeaORM.

**person-matcher-rust-crate** owns:

- Pure-library pairwise comparison: deterministic short-circuits +
  weighted probabilistic scoring with per-field breakdown.
- 42 national personal-identifier schemes, 9 passport-format
  validators, passport-book matching.
- Normalisation (NFKD diacritics, postcode, E.164 phone across 39
  jurisdictions), nickname tables, config presets
  (`strict` / `default` / `lenient`).

**person-front-end-with-svelte** owns:

- Operator routes: list/search, create with 409-duplicate surfacing,
  detail, edit, audit view, match check, merge.
- Its own copy of API types, client, and form primitives (drift
  between front-ends is accepted — repo decision 2026-06-02).

### 2.3 Out of scope (today)

- Authentication middleware in the service and sign-in in the
  front-end (roadmap §15; the SSO provider exists in the
  [authentication entity](../../authentication/)).
- Durable event bus — today the service publishes in-memory only.
- Consent-management UI, GDPR-export download UI, masked-view toggle
  (service endpoints exist; front-end tasks queued in its spec §13).
- Multi-region deployment, bulk import, externalized search (§15).
