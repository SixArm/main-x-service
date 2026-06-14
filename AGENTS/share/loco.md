# Loco

https://loco.rs/

The Main X Index service crates use Loco's backend conventions
(config, hooks, workers, REST API) without its view/template tier —
they are backend-only services. There is no Tera, HTMX, Alpine, or
Lily Design System integration.

## Stack

Use:

- Tantivy search engine
- OpenTelemetry metrics
- Prometheus metrics
- Podman containerization with Debian 13 slim

## Background jobs

https://loco.rs/docs/processing/workers/

- Use Postgres-backed background jobs (`bg_pg`) — no external broker
- NOT SQLite-backed background jobs (`bg_sqlt`)

Config:

```
queue:
  kind: Postgres
  uri: "TODO"
  dangerously_flush: false
  num_workers: 2
```

## Cache

The loco **cache** layer is **in-memory** (loco feature `cache_inmem`,
enabled by default) — no external cache server. Jobs are Postgres-backed
(above) and the cache is in-process.

```
cache:
  kind: InMem
  max_capacity: 33554432 # 32MiB (default if not specified)
```
