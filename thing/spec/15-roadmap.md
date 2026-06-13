## 15. Roadmap

The path from today's single-region MVP trio to the worldwide public
governmental deployment described in §1 and §7. Items here are
aspirational until they land as §13 tasks (entity or crate level).

### 15.1 Security — SSO everywhere

- JWT enforcement on every service endpoint, verified offline against
  the [authentication entity](../../authentication/)'s JWKS; editor /
  read-only / service roles; rate limiting; security headers.
- Front-end sign-in via the central magic-link flow; user attribution
  flows into the audit trail.
- Tracked as entity T-6 / service T-4 / front-end v0.3.

### 15.2 Durable event bus

- Replace the in-memory publisher with a production Fluvio (or
  equivalent) publisher behind the existing `EventProducer` trait;
  add consumers so peer entities and analytics ingest Thing events.
- Tracked as service T-1.

### 15.3 Multi-region replication

- PostgreSQL cross-region replication with regional read replicas;
  active/active app tier; region-aware routing; backup + DR runbook;
  infrastructure as code (OpenTofu) and Kubernetes (Helm, HPA,
  probes) per service §15.

### 15.4 Externalised search

- Move the per-instance Tantivy index to a shared / replicated search
  tier so horizontally-scaled instances serve one consistent index;
  PVC-backed as an interim step.

### 15.5 Bulk import and federation

- Wikidata + OpenLibrary import pipelines (service §15); agency bulk
  on-boarding with batch dedup against the live index; schema.org
  sub-type registry for `additional_type`.

### 15.6 Localisation

- Operator UI localised across the locale set in
  [`agents/share/locales.md`](../../agents/share/locales.md);
  localisable API error messages; locale-aware collation in search.

### 15.7 Matching evolution

- `ThingMatcher` trait so ML / embedding scorers can plug in
  (service T-2 / T-5, `pg_vector`); retire-or-keep decision on the
  in-service 5-component scorer in favour of the embedded canonical
  engine (§16 OQ-1); unified confidence vocabulary (entity T-8).

### 15.8 API completion

- gRPC promoted from stub to working Tonic server for
  high-throughput government integrators (service T-3); front-end
  surfaces for the remaining endpoints (entity T-7).
