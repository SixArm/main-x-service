## 7. Non-Functional Requirements

| Attribute | Target |
|---|---|
| Scale | Millions of things, thousands of data sources |
| Create latency | ≤ 50 ms p50 |
| Read | ≤ 5 ms p50 |
| Search | ≤ 100 ms p50 |
| Match | ≤ 500 ms p99 |
| Throughput | ≥ 1 000 req/sec single instance |
| Availability | HADR; stateless app tier; PostgreSQL replication |
| Fault tolerance | Graceful shutdown; connection pooling; health checks; non-root containers |
| Observability | OTLP traces / metrics / logs; `traceparent` per request |
| Background jobs | Loco `BackgroundQueue` backed by **PostgreSQL** (`bg_pg`) — same database as application data; no external broker |
| Security | Offline PASETO v4.public bearer verification against the auth-service published Ed25519 keys — opt-in per handler, plus env-gated **blanket `/api/*` enforcement** (`THING_REQUIRE_AUTH`, default **off**; public allow-list `/api/health`) with ABAC authorization inside the guard (shared `authentication-verifier` engine over the token's `attrs` claim; `THING_ABAC_POLICY`/`_FILE`, else the built-in default policy — read allow / mutation deny; `401` = bad credential, `403` = policy denied). Key set: `THING_PASETO_KEYS_URL` set ⇒ fetched once over HTTP at boot (fetched set wins; fetch failure warn-logs and falls back — the service always boots); unset/blank ⇒ `THING_PASETO_KEYS` env key set, else empty reject-all. TLS at the edge |

