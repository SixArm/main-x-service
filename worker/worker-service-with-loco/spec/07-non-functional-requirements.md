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
| Security | Offline PASETO v4.public bearer verification against the auth-service published Ed25519 keys; key set from `WORKER_PASETO_KEYS` (JSON) or fetched once at boot from `WORKER_PASETO_KEYS_URL` (fetched set wins; fetch failure warns and falls back to the env path — the service **always boots**); blanket `/api/*` enforcement implemented **default-off** behind `WORKER_REQUIRE_AUTH` (read at router construction — restart to change; public allow-list: health, OpenAPI/Swagger, metrics; T-1b remainder: RBAC roles); TLS at the edge |
| Observability | OTLP traces / metrics / logs; `traceparent` per request |
| Background jobs | Loco `BackgroundQueue` backed by **PostgreSQL** (`bg_pg`) — same database as application data; no external broker |

