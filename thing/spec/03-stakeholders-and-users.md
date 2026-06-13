## 3. Stakeholders and Users

A worldwide public governmental deployment, with millions of users
across many locales.

| Stakeholder | Interest |
|---|---|
| Registry operators | Day-to-day CRUD / search / dedup / merge through the operator UI; review-queue triage |
| Asset-owning agencies | Stable Thing IDs for the records they steward; deterministic DOI / ISBN / GTIN convergence; bulk on-boarding (roadmap §15) |
| Open-data / system integrators | REST API + OpenAPI contract; event stream for downstream sync; `same_as` / `additional_type` hooks |
| Auditors / regulators | Complete who / what / when audit trail; GDPR / UK DPA evidence; ISO/IEC 27001 / 42001 operational controls |
| Compliance / privacy officers | Masking, consent records, GDPR Article 15 export for things linked to individuals |
| Operations / DBA | PostgreSQL schema + migration discipline; health checks; observability |
| Other Main X Index entities | Cross-references via `thing_id`; the catch-all registry when no dedicated entity fits |
| Developers and AI agents | Three living specs + this umbrella; SDD discipline; bridge tests pinning the integration contract |

### Primary user journeys

1. **Operator registers an asset** — front-end `/things/new` →
   `POST /api/things` → 409 duplicate candidates surfaced inline.
2. **Operator resolves a duplicate** — `/things/match` and
   `/things/merge` → service merge with snapshot + audit + event.
3. **Integrator deduplicates a feed** — embeds `thing-matcher`
   directly, or calls `POST /api/things/deduplicate`.
4. **Auditor reviews activity** — `/things/[id]/audit` and
   `GET /api/audit/recent`.
