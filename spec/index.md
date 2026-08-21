# main-x-service

Monorepo for web services: the **Main X Index** family — a federated
identity index, one entity per directory. Each entity directory holds
a front-end web app, a matcher (or verifier) library crate, a service
API crate, plus entity-level `spec/` and `agents/` umbrella docs.

| Entity | Front-end (Svelte) | Library crate | Service crate | Umbrella docs |
| ------ | ------------------ | ------------- | ------------- | ------------- |
| authentication | [authentication-front-end-with-svelte](../authentication/authentication-front-end-with-svelte/) | [authentication-verifier-rust-crate](../authentication/authentication-verifier-rust-crate/) | [authentication-service-with-loco](../authentication/authentication-service-with-loco/) | [spec](../authentication/spec/index.md) · [AGENTS](../authentication/agents/index.md) |
| care-pathway | [care-pathway-front-end-with-svelte](../care-pathway/care-pathway-front-end-with-svelte/) | [care-pathway-matcher-rust-crate](../care-pathway/care-pathway-matcher-rust-crate/) | [care-pathway-service-with-loco](../care-pathway/care-pathway-service-with-loco/) | [spec](../care-pathway/spec/index.md) · [AGENTS](../care-pathway/agents/index.md) |
| case | [case-front-end-with-svelte](../case/case-front-end-with-svelte/) | [case-matcher-rust-crate](../case/case-matcher-rust-crate/) | [case-service-with-loco](../case/case-service-with-loco/) | [spec](../case/spec/index.md) · [AGENTS](../case/agents/index.md) |
| course | [course-front-end-with-svelte](../course/course-front-end-with-svelte/) | [course-matcher-rust-crate](../course/course-matcher-rust-crate/) | [course-service-with-loco](../course/course-service-with-loco/) | [spec](../course/spec/index.md) · [AGENTS](../course/agents/index.md) |
| event | [event-front-end-with-svelte](../event/event-front-end-with-svelte/) | [event-matcher-rust-crate](../event/event-matcher-rust-crate/) | [event-service-with-loco](../event/event-service-with-loco/) | [spec](../event/spec/index.md) · [AGENTS](../event/agents/index.md) |
| organization | [organization-front-end-with-svelte](../organization/organization-front-end-with-svelte/) | [organization-matcher-rust-crate](../organization/organization-matcher-rust-crate/) | [organization-service-with-loco](../organization/organization-service-with-loco/) | [spec](../organization/spec/index.md) · [AGENTS](../organization/agents/index.md) |
| person | [person-front-end-with-svelte](../person/person-front-end-with-svelte/) | [person-matcher-rust-crate](../person/person-matcher-rust-crate/) | [person-service-with-loco](../person/person-service-with-loco/) | [spec](../person/spec/index.md) · [AGENTS](../person/agents/index.md) |
| place | [place-front-end-with-svelte](../place/place-front-end-with-svelte/) | [place-matcher-rust-crate](../place/place-matcher-rust-crate/) | [place-service-with-loco](../place/place-service-with-loco/) | [spec](../place/spec/index.md) · [AGENTS](../place/agents/index.md) |
| thing | [thing-front-end-with-svelte](../thing/thing-front-end-with-svelte/) | [thing-matcher-rust-crate](../thing/thing-matcher-rust-crate/) | [thing-service-with-loco](../thing/thing-service-with-loco/) | [spec](../thing/spec/index.md) · [AGENTS](../thing/agents/index.md) |
| worker | [worker-front-end-with-svelte](../worker/worker-front-end-with-svelte/) | [worker-matcher-rust-crate](../worker/worker-matcher-rust-crate/) | [worker-service-with-loco](../worker/worker-service-with-loco/) | [spec](../worker/spec/index.md) · [AGENTS](../worker/agents/index.md) |

## Monorepo-wide topic specs

Comprehensive, repo-grounded specs for each cross-cutting topic. Each is
the family-wide source of truth; the briefer versions under
[`../agents/share/`](../agents/share/index.md) remain as quick references.

### Foundations

| Topic | Description |
| ----- | ----------- |
| [architecture](architecture/index.md) | System architecture — the family shape, the two service generations (loco vs older Axum), layering, cross-service integration |
| [dataflow](dataflow/index.md) | Request/data flows — create, match, merge, search, read/masked/export, auth |
| [tech-stack](tech-stack/index.md) | Technology stack + hard constraints (Podman/Tokio/MiMalloc/PostgreSQL/chrono) and current drift |
| [agents-directory-name-is-lowercase.md](agents-directory-name-is-lowercase.md) | The agent reference directories are named `agents`, lowercase — the rule, why a case-only mismatch hides on macOS, and the CI stage behind it |
| [rust-msrv-n-minus-3.md](rust-msrv-n-minus-3.md) | Minimum Supported Rust Version — the N-3 policy, how it is declared and verified, and how to bump it |

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
| [authentication](authentication/index.md) | SSO — passwordless magic-link, Postgres cookie sessions, cross-service PASETO v4.public (published Ed25519 key; RS256/JWKS decommissioned), key rotation, rate limiter, blanket enforcement, BFF token handoff |
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
