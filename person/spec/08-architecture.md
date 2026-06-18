## 8. Architecture

### 8.1 The trio

```
+---------------------------------------------------------------+
|                     Operators (browsers)                       |
+-------------------------------+--------------------------------+
                                |
+-------------------------------v--------------------------------+
|              person-front-end-with-svelte (SPA)                 |
|  SvelteKit 2 + Svelte 5 runes + SVAR DataGrid + Lily Headless   |
|  src/lib/api/{types,client,persons}.ts  (envelope-aware fetch)  |
+-------------------------------+--------------------------------+
                                |  HTTP JSON  { success, data, error }
+-------------------------------v--------------------------------+
|               person-service-with-loco (registry)              |
|  REST /api/* (15 endpoints)  |  FHIR R5 /fhir/Person  |  gRPC   |
|                              |                        |  (stub) |
|  validation -> dup-detect -> repository -> index -> events ->   |
|  audit                                                          |
|       +--------------------------------------------------+      |
|       |  src/matching/  (in-service algorithms)           |      |
|       |  src/matching/adapter.rs --to_matcher_person()--> |      |
|       |  matcher_lib  ==  person-matcher (embedded)       |      |
|       +--------------------------------------------------+      |
+----------+--------------------+--------------------+-----------+
           |                    |                    |
 +---------v------+   +---------v--------+   +-------v---------+
 |  PostgreSQL    |   |  Tantivy index   |   |  Event stream   |
 |  (SeaORM,      |   |  (11 fields,     |   |  (in-memory;    |
 |  12+ tables)   |   |  local disk)     |   |  durable = §15) |
 +----------------+   +------------------+   +-----------------+

 person-matcher-rust-crate: pure library — no IO, no runtime, no DB.
 Also usable standalone by any other consumer.
```

### 8.2 Dependency direction

Strictly one-way:

```
front-end  ──HTTP──▶  service  ──Cargo path dep──▶  matcher
```

- The front-end depends on the service's REST contract only (FR-19).
- The service embeds the matcher; the matcher knows nothing about the
  service, the database, or HTTP (FR-20).
- Nothing depends on the front-end.

### 8.3 SSO integration

The Main X Index has one sign-on provider: the
[authentication entity](../../authentication/) (passwordless
magic-link; RS256 JWT issuance; JWKS endpoint for offline
verification). Integration plan (entity §13 E-1 / service §13 T-1):

1. Service: JWT-validator extractor on `/api/*`; verify RS256
   signatures against the cached JWKS; reject with `401`.
2. Front-end: redirect unauthenticated operators to the
   authentication front-end; attach the bearer token to `ApiClient`.
3. No password handling anywhere in the person entity — verification
   only.

### 8.4 Deployment topology

**Today (delivered):** single-node Compose — PostgreSQL + service
container (non-root, health-checked) + front-end static build; Tantivy
index on local disk.

**Governmental scale (roadmap — §15, not yet built):**

- Multiple stateless service replicas per region behind a load
  balancer; Kubernetes/Helm with HPA.
- PostgreSQL with cross-region replication; managed failover.
- Durable event bus (Fluvio/Kafka/NATS) replacing the in-memory
  publisher; consumers feed downstream agency systems.
- Search externalized from per-instance local disk so replicas share
  one index view.
- Front-end served from a CDN; per-region API endpoints.

### 8.5 Where the details live

- Service internals: [service spec §8](../person-service-with-loco/spec/08-architecture.md)
  (module layout, layering rules, `AppState`, data flows).
- Matcher internals: [matcher spec §9](../person-matcher-rust-crate/spec/09-architecture.md)
  and [§10](../person-matcher-rust-crate/spec/10-component-specifications.md).
- Front-end internals: [front-end spec §8](../person-front-end-with-svelte/spec/08-architecture.md).
- Index-wide layered view: [agents/share/architecture.md](../../agents/share/architecture.md).
