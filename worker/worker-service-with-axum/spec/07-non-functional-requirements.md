## 7. Non-Functional Requirements

| Attribute | Target |
|---|---|
| Scale | Millions of workers, thousands of organisations |
| Create latency | ≤ 50 ms p50 |
| Read | ≤ 5 ms p50 |
| Search | ≤ 100 ms p50 |
| Match | ≤ 500 ms p99 |
| Throughput | ≥ 1 000 req/sec single instance |
| Availability | HADR; stateless app tier; PostgreSQL replication |
| Fault tolerance | Graceful shutdown; connection pooling; health checks; non-root containers |
| Security | Offline PASETO v4.public bearer verification against the auth-service published Ed25519 keys; key set from `WORKER_PASETO_KEYS` (JSON) or fetched once at boot from `WORKER_PASETO_KEYS_URL` (fetched set wins; fetch failure warns and falls back to the env path — the service **always boots**); blanket `/api/*` enforcement implemented **default-off** behind `WORKER_REQUIRE_AUTH` (read at router construction — restart to change; public allow-list: health, OpenAPI/Swagger, metrics) with ABAC authorization inside the guard (shared `authentication-verifier` engine over the token's `attrs` claim; `WORKER_ABAC_POLICY`/`_FILE`, else the built-in default policy — read allow / mutation deny; `401` = bad credential, `403` = policy denied); TLS at the edge |
| Observability | OTLP traces / metrics / logs; `traceparent` per request |
| Background jobs | Loco `BackgroundQueue` backed by **PostgreSQL** (`bg_pg`) — same database as application data; no external broker |
| Event bus | Durable event bus per [`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §7. Transport selected by `WORKER_EVENT_TRANSPORT` (`memory` \| `outbox`, default `memory`). `outbox` (Phase 2) writes one `event_outbox` row **inside each write's transaction** (exactly-once relative to the DB). Phase 3 (`src/relay.rs`): a background relay loop drains `event_outbox` → `EventSink` → `mark_published` and periodically purges old rows, **gated by `WORKER_EVENT_TRANSPORT=outbox` AND `WORKER_EVENT_RELAY`** (truthy `1`/`true`/`yes`/`on`) — off by default ⇒ no loop. `WORKER_EVENT_RELAY_INTERVAL_SECS` (default `5`, floored at 1) is the drain-tick interval; `WORKER_EVENT_RETENTION_DAYS` (default `7`) is the outbox-row TTL, **enforced** by the relay's periodic `purge_published` when the relay runs. Consumers dedupe on the envelope `event_id` (at-least-once delivery). |

