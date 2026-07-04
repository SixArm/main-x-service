## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept. Ordered roughly by intent — privacy leads because case data
is personal data.

- **Privacy and data-subject rights.** *(highest priority — §12.)*
  Per-field masking + a masked-view endpoint (`GET …/{pid}/masked`);
  GDPR data-subject export (`GET …/{pid}/export` and a subject-scoped
  export across cases sharing a `subjects` id); a GDPR-erasure path
  layered on soft delete; consent / lawful-basis records where
  applicable. See
  [`agents/share/privacy.md`](../../agents/share/privacy.md). (Seed:
  T-10.)
- **Bulk import / export.** *(governance-heavy — §12.)* Async,
  job-based bulk load and extract per the family contract
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md);
  the case-specific stable keys, CSV columns, and (prominently) the
  export sensitivity posture are declared in
  [case-service §8.7](../case-service-with-loco/spec/index.md). Because
  case data is personal/sensitive, export is masked by default, full /
  unmasked or soft-deleted export requires case-read authorisation, and
  every export is audited — a bulk extract must never reveal more than
  reading cases one at a time. (Seed: case-service §13 bulk task.)
- **Family parity — match / search / merge.** Bring the entity to the
  mature-sibling baseline
  ([`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md)):
  Tantivy full-text + fuzzy search, search-blocked duplicate candidates,
  real-time `409` duplicate detection on create, review queue, batch
  deduplicate scan. Merge with history + snapshots is already
  delivered. (Seeds: T-6, T-11.)
- **Auditability — durable bus.** Audit log + audit query API + event
  streaming on every CRUD/merge are delivered (in-process). Next: a
  **durable event bus** (replacing in-process publishing) so peer
  registries and analytics can subscribe across replicas. (Seed: T-12.)
- **Security — blanket SSO enforcement.** Offline token verification
  against the central auth-service's published key is delivered for
  `whoami` + `actor` stamping, with the credential already **PASETO v4
  public** (Ed25519) per
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (source of truth; supersedes the RS256-JWT + JWKS model); extend
  to **blanket `/api/*` enforcement**, role split between read-side
  integrators and caseworkers, paseto-keys-over-HTTP fetch at boot, and
  rate limiting. (Seed: T-7 follow-up.)
- **Front-end depth.** Search box, audit / event views, and a merge
  action from the duplicates list. (Seeds: T-11, T-8 follow-up.)
- **Cross-system case linkage.** Import/export of docket / external
  case id metadata so a court system, a benefits platform, and this
  registry can resolve the same matter; the registry-of-identities
  posture stays — no case workflow (§1.3).
- **Localization.** Operator UI in the
  [`agents/share/locales.md`](../../agents/share/locales.md) locale set;
  multilingual case titles via `alternate_titles` + `in_language`;
  cross-language duplicate linkage through deterministic identifiers.
- **Scale-out and operations.** Multi-replica deployment, PostgreSQL
  replication, JSONB GIN / side-table indexing for `subjects` (OQ-3),
  OTLP observability pipeline, Prometheus metrics, encryption at rest,
  backup / DR runbooks, container hardening.
- **gRPC.** Tonic stub once a high-throughput consumer exists. (The
  OpenAPI 3 doc + Swagger UI for the REST surface are already delivered
  — hand-written `src/openapi.rs`, not utoipa-derived.)
