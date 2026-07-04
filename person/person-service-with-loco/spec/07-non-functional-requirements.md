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
| Security | Offline PASETO v4.public bearer verification against the auth-service published Ed25519 keys; blanket `/api/*` enforcement middleware (T-1b), **default-off** behind `PERSON_REQUIRE_AUTH` with a public allow-list (health, OpenAPI/Swagger, metrics); roles/RBAC + boot-time key fetch are T-1c; TLS at the edge |

