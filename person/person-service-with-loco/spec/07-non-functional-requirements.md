## 7. Non-Functional Requirements

| Attribute | Target |
|---|---|
| Scale | Millions of persons |
| Create latency (incl. dup-check + index + audit) | ≤ 50 ms p50 |
| Read | ≤ 5 ms p50 |
| Search | ≤ 100 ms p50 |
| Match | ≤ 500 ms p99 |
| Throughput | ≥ 1 000 req/sec single instance |
| Availability | HADR; stateless app tier; PostgreSQL replication |
| Fault tolerance | Graceful shutdown; connection pooling; health checks; non-root containers |
| Observability | OTLP traces / metrics / logs; `traceparent` per request; JSON logs in production |
| Background jobs | Loco `BackgroundQueue` backed by **PostgreSQL** (`bg_pg`) — same database as application data; no external broker |
| Event bus | Durable **transactional outbox** (Phase 2, [event-bus.md](../../../agents/share/event-bus.md) §3) selected by `PERSON_EVENT_TRANSPORT` (`memory` default = in-memory publish; `outbox` = one `event_outbox` row written **in the same transaction** as each entity write, so no committed change loses its event and vice versa). The **Phase-3 relay** (T-21) drains unpublished rows to an `EventSink` and stamps `published_at`, enforcing the `PERSON_EVENT_RETENTION_DAYS` (default `7`) outbox row TTL; it runs only when the transport is `outbox` **and** `PERSON_EVENT_RELAY` is truthy (poll interval `PERSON_EVENT_RELAY_INTERVAL_SECS`, default `5`). The default sink logs each event (no broker); a real Fluvio `EventSink` (behind a `fluvio` feature) = roadmap. |
| Security | Offline PASETO v4.public bearer verification against the auth-service published Ed25519 keys; key set from `PERSON_PASETO_KEYS` (JSON) or fetched once at boot from `PERSON_PASETO_KEYS_URL` (fetched set wins; fetch failure warns and falls back to the env path — the service **always boots**); blanket `/api/*` enforcement middleware (T-1b), **default-off** behind `PERSON_REQUIRE_AUTH` with a public allow-list (health, OpenAPI/Swagger, metrics), with ABAC authorization inside the guard (shared `authentication-verifier` engine over the token's `attrs` claim; `PERSON_ABAC_POLICY`/`_FILE`, else the built-in default policy — read allow / mutation deny; `401` = bad credential, `403` = policy denied); TLS at the edge |

