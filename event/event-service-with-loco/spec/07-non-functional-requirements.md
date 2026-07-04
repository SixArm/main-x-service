## 7. Non-Functional Requirements

| Attribute | Target |
|---|---|
| Scale | Millions of events |
| Create latency | ≤ 50 ms p50 |
| Read | ≤ 5 ms p50 |
| Search | ≤ 100 ms p50 |
| Match | ≤ 500 ms p99 |
| Throughput | ≥ 1 000 req/sec single instance |
| Availability | HADR; stateless app tier; PostgreSQL replication |
| Fault tolerance | Graceful shutdown; connection pooling; health checks; non-root containers |
| Observability | OTLP traces / metrics / logs; `traceparent` per request |
| Security | Offline PASETO v4.public bearer verification against the auth-service published Ed25519 keys (opt-in per handler; blanket enforcement is the open remainder of T-8); TLS at the edge |
| Background jobs | Loco `BackgroundQueue` backed by **PostgreSQL** (`bg_pg`) — same database as application data; no external broker |

