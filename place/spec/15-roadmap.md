## 15. Roadmap

The path from today's single-node trio to a worldwide public
governmental deployment. Ordered by dependency, not by date. Items
already queued in a crate spec are cited rather than duplicated.

### 15.1 Security first

- **SSO / JWT enforcement** across the trio — verify RS256 JWTs
  offline against the [authentication entity](../../authentication/)'s
  JWKS; editor / curator / read-only / service roles; rate limiting,
  security headers (service spec §13 T-8; entity E-5). Prerequisite
  for regulator-grade user attribution in the audit trail
  ([§12.4](12-compliance.md)).

### 15.2 Durability and scale

- **Durable event bus** — Fluvio production publisher + consumers
  (service spec §13 T-3; E-6), so place events outlive restarts and
  feed peer entities.
- **PostGIS-backed geo queries** — `geometry(Point, 4326)` column,
  GiST index, `ST_DWithin` radius search (service spec §13 T-1; E-7);
  recursive CTEs for hierarchy depth (T-2).
- **Multi-region replication** — PostgreSQL streaming replication;
  read replicas per region, single write region; Tantivy index on
  PVCs with a rebuild story; Kubernetes (Helm, HPA, probes) and
  OpenTofu IaC (service spec §15). Data-residency split per
  jurisdiction is an open question ([§16](16-open-questions.md)).
- **gRPC** — promote the Tonic stub for high-throughput
  machine-to-machine callers (service spec §13 T-4).

### 15.3 Authoritative data integration

- **Address-authority / gazetteer integrations** — bulk import +
  reconciliation pipelines for national gazetteers (e.g. GNIS for the
  US, OS Open Names for the UK, IGN for France, GeoNames globally) on
  the pattern of the OSM import pipeline (service spec §13 T-5), with
  idempotency keyed on authority identifiers.
- **Reverse geocoding + GeoJSON export** — service spec §13 T-6 / T-7;
  unlocks "what administrative area contains this coordinate" for
  agency callers and standards-friendly exchange.

### 15.4 Worldwide operator experience

- **Localization** — operator UI in the locales of
  [`agents/share/locales.md`](../../agents/share/locales.md) (E-10);
  locale-aware address-form layouts; international street-vocabulary
  normalisation in the matcher (E-11).
- **Operator completeness** — masked-view toggle, GDPR-export
  download, dedup-scan results UI, review-queue triage screen
  (front-end spec §13 T-18–T-20 and successors).

### 15.5 Production hardening

- Security audit + penetration test, GDPR validation, DR runbook,
  backup / restore drills, CI/CD pipeline (service spec §15); Grafana
  dashboards + alerting on the existing OTLP / Prometheus surface.
