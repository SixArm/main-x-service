## 3. Stakeholders and Users

The Place entity serves a worldwide public governmental deployment.

| Stakeholder | Needs | Primary touchpoint |
|---|---|---|
| **Registry operators** (data stewards, clerks) | Create / search / correct place records; resolve duplicate candidates; merge confirmed duplicates; review queue triage | [place-front-end](../place-front-end-with-svelte/) operator UI |
| **Government agencies** (service delivery, licensing, civil protection) | Resolve a place by identifier (GLN, FIPS, GNIS, OSM ID); geo-radius lookups ("facilities within R km"); stable canonical IDs across systems | Service REST API |
| **Address / gazetteer authorities** (national mapping agencies, postal authorities) | Bulk import and reconciliation of authoritative place data; identifier cross-referencing | Service REST API + import pipelines (roadmap, [§15](15-roadmap.md)) |
| **Integrators** (peer Main X Index entities, third-party systems) | Stable REST contract, OpenAPI spec, event stream of CRUD / merge activity, eventually gRPC | Service REST API, Swagger UI, event streaming |
| **Auditors / regulators** (GDPR supervisory authorities, ISO assessors) | Complete who / what / when audit trail; data-subject export; consent records; masking of personal place data | Audit API, GDPR export endpoint |
| **Data subjects** (residents whose home addresses are place records) | Privacy: masking, consent, export, erasure via soft delete | Indirect — via operators and the privacy endpoints |
| **Developers / agents** (human and AI contributors) | Single source of truth per subproject; explainable matching; three-part-PR discipline | Per-subproject `spec/` + `AGENTS/`; this entity spec |
| **Operations / SRE** | Health checks, OTLP traces, Prometheus metrics, stateless horizontal scaling | Service `/api/health`, `/metrics.prom`, OTLP export |

### 3.1 Notes

- Operators authenticate via the central
  [authentication-service](../../authentication/authentication-service-rust-crate/)
  SSO once JWT enforcement lands (service spec §13 T-8; entity roadmap
  [§15](15-roadmap.md)). Today the API is unauthenticated — a known
  gap, tracked in [§13](13-tasks.md).
- Data-engineer consumers can also use
  [place-matcher](../place-matcher-rust-crate/) standalone, outside
  the service, for offline deduplication pipelines (matcher spec §1.3).
