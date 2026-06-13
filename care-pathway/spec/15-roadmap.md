## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept. Ordered roughly by intent.

- **Family parity — match / search / merge.** Bring the entity to
  the mature-sibling baseline
  ([`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md)):
  Tantivy full-text + fuzzy search, search-blocked duplicate
  candidates, real-time `409` duplicate detection on create, review
  queue, merge with link tracking and snapshots, batch deduplicate
  scan. (Seeds: T-6, T-8.)
- **Auditability.** Audit log + audit query API + event streaming on
  every CRUD/merge; then a **durable event bus** (replacing
  in-process publishing) so peer registries and analytics can
  subscribe. (Seed: T-3.)
- **Security.** JWT enforcement on `/api/*` verifying RS256 tokens
  against the central auth-service JWKS; role split between
  read-side integrators and registry operators; rate limiting.
  (Seed: T-7.)
- **FHIR PlanDefinition import/export.** Map `PlanDefinition.url` /
  `identifier` / `title` / `useContext` to `CarePathway` identifier
  metadata; import a `PlanDefinition` reference to register or link a
  pathway; export a registered pathway as a `PlanDefinition` stub
  carrying the registry `pid`. Registry-of-identities posture stays —
  no pathway logic (§8.5).
- **Condition-code-system services.** Integrate SNOMED CT / ICD
  terminology services for code validation (T-9), display names,
  and cross-system mapping (e.g. SNOMED ↔ ICD-10), so two pathways
  coded in different systems for the same condition can corroborate.
- **Localization.** Operator UI in the
  [`agents/share/locales.md`](../../agents/share/locales.md) locale
  set; multilingual pathway names via `alternate_names` +
  `in_language`; cross-language duplicate linkage through
  deterministic identifiers.
- **Scale-out and operations.** Multi-replica deployment, PostgreSQL
  replication, JSONB GIN / side-table indexing for condition codes
  (OQ-3), OTLP observability pipeline, Prometheus metrics, backup /
  DR runbooks, container hardening.
- **OpenAPI + gRPC.** utoipa/Swagger for the REST surface (T-9);
  Tonic stub once a high-throughput consumer exists.
