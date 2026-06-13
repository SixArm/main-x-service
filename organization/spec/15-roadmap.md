## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept. Direction of travel: from registry MVP to a
population-scale public-register service.

- **Match–search–merge parity with the mature entities.** Tantivy
  full-text / fuzzy / phonetic search; real-time duplicate detection
  on create (`409` + candidates — pending OQ-3); duplicate review
  queue with `Pending` / `Confirmed` / `Rejected` / `AutoMerged`;
  batch deduplication; record merge with link tracking and
  transferred-data snapshots. Reference:
  [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md).
- **Security: JWT enforcement.** Verify RS256 JWTs offline against
  the [authentication entity](../../authentication/)'s JWKS;
  attributable audit `actor`; front-end sign-in + bearer wiring;
  role-based authorisation for write vs read.
- **Privacy layer.** Per-field masking honouring the §12
  open-register / protected-contact split; GDPR Article 15 export;
  consent records where applicable (sole traders).
- **Durable event bus.** Swap the in-memory ring buffer for a broker
  (Fluvio / Kafka / NATS — family decision pending) behind the
  existing publish call; event consumers for downstream register
  sync. This is the gating item for multi-replica deployment.
- **Multi-region scale.** Stateless replicas behind load balancers;
  PostgreSQL replication + read replicas; regional deployments with
  cross-region replication for the worldwide-governmental-system
  goal; load tests at register volumes (millions of records).
- **Register-feed integrations.** Scheduled ingestion and
  reconciliation against authoritative sources: GLEIF golden copy
  (LEI), national company registers (e.g. Companies House, national
  business registers), ROR for research organizations, Wikidata
  cross-references. Deterministic identifiers (§5.2) make these
  feeds idempotent upserts.
- **Localization.** Operator UI localized to the
  [`agents/share/locales.md`](../../agents/share/locales.md) set;
  configurable / extensible legal-suffix lists per jurisdiction
  (matcher §23); locale-aware address rendering.
- **Validation depth.** Identifier format checks (LEI check digits,
  GLN check digit, VAT national prefixes — matcher OQ), URL and
  ISO 3166 country validation, returning `422`.
- **Observability.** OTLP traces / metrics / logs per
  [`agents/share/observability.md`](../../agents/share/observability.md);
  match-score histograms; Grafana dashboards.
- **gRPC tier.** Tonic surface for high-throughput machine callers,
  mirroring REST CRUD + match, once the REST contract stabilises.
