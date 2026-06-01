# Loco

https://loco.rs/

## Stack

Use:

- Lily Design System HTML Headless (~/git/lilydesignsystem/lily-design-system/lily-design-system-html-headless)
- Tera templates
- HTMX JavaScript
- Alpine.js JavaScript
- Tantivy search engine
- OpenTelemetry metrics
- Prometheus metrics

## Background jobs 

https://loco.rs/docs/processing/workers/

- Use Postgres-backed background jobs
- NOT SQLite-backed background jobs (bg_sqlt)
- NOT Redis-backed background jobs (bg_redis)

Config:

```
queue:
  kind: Postgres
  uri: "TODO"
  dangerously_flush: false
  num_workers: 2
```
