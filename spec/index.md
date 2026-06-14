# main-x-service

Monorepo for web services: the **Main X Index** family — a federated
identity index, one entity per directory. Each entity directory holds
a front-end web app, a matcher (or verifier) library crate, a service
API crate, plus entity-level `spec/` and `AGENTS/` umbrella docs.

| Entity | Front-end (Svelte) | Library crate | Service crate | Umbrella docs |
| ------ | ------------------ | ------------- | ------------- | ------------- |
| authentication | [authentication-front-end-with-svelte](../authentication/authentication-front-end-with-svelte/) | [authentication-verifier-rust-crate](../authentication/authentication-verifier-rust-crate/) | [authentication-service-rust-crate](../authentication/authentication-service-rust-crate/) | [spec](../authentication/spec/index.md) · [AGENTS](../authentication/AGENTS/index.md) |
| care-pathway | [care-pathway-front-end-with-svelte](../care-pathway/care-pathway-front-end-with-svelte/) | [care-pathway-matcher-rust-crate](../care-pathway/care-pathway-matcher-rust-crate/) | [care-pathway-service-rust-crate](../care-pathway/care-pathway-service-rust-crate/) | [spec](../care-pathway/spec/index.md) · [AGENTS](../care-pathway/AGENTS/index.md) |
| case | [case-front-end-with-svelte](../case/case-front-end-with-svelte/) | [case-matcher-rust-crate](../case/case-matcher-rust-crate/) | [case-service-rust-crate](../case/case-service-rust-crate/) | [spec](../case/spec/index.md) · [AGENTS](../case/AGENTS/index.md) |
| course | [course-front-end-with-svelte](../course/course-front-end-with-svelte/) | [course-matcher-rust-crate](../course/course-matcher-rust-crate/) | [course-service-rust-crate](../course/course-service-rust-crate/) | [spec](../course/spec/index.md) · [AGENTS](../course/AGENTS/index.md) |
| event | [event-front-end-with-svelte](../event/event-front-end-with-svelte/) | [event-matcher-rust-crate](../event/event-matcher-rust-crate/) | [event-service-rust-crate](../event/event-service-rust-crate/) | [spec](../event/spec/index.md) · [AGENTS](../event/AGENTS/index.md) |
| organization | [organization-front-end-with-svelte](../organization/organization-front-end-with-svelte/) | [organization-matcher-rust-crate](../organization/organization-matcher-rust-crate/) | [organization-service-rust-crate](../organization/organization-service-rust-crate/) | [spec](../organization/spec/index.md) · [AGENTS](../organization/AGENTS/index.md) |
| person | [person-front-end-with-svelte](../person/person-front-end-with-svelte/) | [person-matcher-rust-crate](../person/person-matcher-rust-crate/) | [person-service-rust-crate](../person/person-service-rust-crate/) | [spec](../person/spec/index.md) · [AGENTS](../person/AGENTS/index.md) |
| place | [place-front-end-with-svelte](../place/place-front-end-with-svelte/) | [place-matcher-rust-crate](../place/place-matcher-rust-crate/) | [place-service-rust-crate](../place/place-service-rust-crate/) | [spec](../place/spec/index.md) · [AGENTS](../place/AGENTS/index.md) |
| thing | [thing-front-end-with-svelte](../thing/thing-front-end-with-svelte/) | [thing-matcher-rust-crate](../thing/thing-matcher-rust-crate/) | [thing-service-rust-crate](../thing/thing-service-rust-crate/) | [spec](../thing/spec/index.md) · [AGENTS](../thing/AGENTS/index.md) |
| worker | [worker-front-end-with-svelte](../worker/worker-front-end-with-svelte/) | [worker-matcher-rust-crate](../worker/worker-matcher-rust-crate/) | [worker-service-rust-crate](../worker/worker-service-rust-crate/) | [spec](../worker/spec/index.md) · [AGENTS](../worker/AGENTS/index.md) |

## Monorepo-wide topic specs

Comprehensive, repo-grounded specs for each cross-cutting topic. Each is
the family-wide source of truth; the briefer versions under
[`../agents/share/`](../agents/share/index.md) remain as quick references.

### Foundations

| Topic | Description |
| ----- | ----------- |
| [architecture](architecture/index.md) | System architecture — the family shape, the two service generations (loco vs older Axum), layering, cross-service integration |
| [dataflow](dataflow/index.md) | Request/data flows — create, match, merge, search, read/masked/export, auth |
| [tech-stack](tech-stack/index.md) | Technology stack + hard constraints (Podman/Tokio/MiMalloc/PostgreSQL/jiff) and current drift |

### Data & persistence

| Topic | Description |
| ----- | ----------- |
| [postgresql](postgresql/index.md) | PostgreSQL — version, per-service DBs, pooling, the two persistence styles, extensions, JSONB, locks, migrations, ops |
| [data-modeling.md](data-modeling.md) | Data-modeling rules (SQL-first, child tables, discriminators, JSONB policy) |
| [data.md](data.md) | Data conventions |

### Domain capabilities

| Topic | Description |
| ----- | ----------- |
| [matching](matching/index.md) | Record matching — probabilistic + deterministic algorithms, short-circuits, confidence bands, matcher crates + bridge |
| [search](search/index.md) | Search — Postgres `ILIKE` (today) + Tantivy full-text/fuzzy/phonetic (planned), geo-radius, blocking |
| [merge](merge/index.md) | Record merge — fold duplicate into main, history, soft-delete, events, review queue |
| [validation](validation/index.md) | Data quality & validation — 422 contract, checksum validators (GLN, NHS, Verhoeff, …), normalization |

### Cross-cutting concerns

| Topic | Description |
| ----- | ----------- |
| [authentication](authentication/index.md) | SSO — passwordless magic-link, RS256 JWT + JWKS, key rotation, sessions, rate limiter, blanket enforcement, token handoff |
| [restful](restful/index.md) | REST API conventions — routes, status codes, OpenAPI/Swagger, auth, FHIR/gRPC, pagination, metrics |
| [auditability](auditability/index.md) | Audit log + change-event stream — who/what/when, the actor, event taxonomy |
| [event-streaming](event-streaming/index.md) | Event stream — the in-memory Phase 1 seam + the durable outbox→Fluvio design |
| [privacy](privacy/index.md) | Privacy — masking, GDPR export/erasure, consent, anti-enumeration, per-entity sensitivity |
| [observability](observability/index.md) | Observability — tracing, OpenTelemetry/OTLP, Prometheus `/metrics.prom` |
| [availability](availability/index.md) | Availability — stateless design, pooling, health checks, Podman, horizontal scaling |
| [locales](locales/index.md) | i18n/l10n — the authentication email/UI catalog pattern, supported languages, BCP-47, RTL |
| [testing](testing/index.md) | Testing strategy — un-gated unit, DB-gated request, bridge tests, front-end vitest/Playwright, CI |
| [compliance](compliance/index.md) | Regulatory compliance — target regimes, obligation→control map, data-sensitivity, ISO 42001, gaps |

### Pointers

| Document | Description |
| -------- | ----------- |
| [../AGENTS.md](../AGENTS.md) | Root agent guide — subproject directory and shared reference docs |
| [../agents/share/index.md](../agents/share/index.md) | Shared reference docs (the briefer versions of the topics above) |
