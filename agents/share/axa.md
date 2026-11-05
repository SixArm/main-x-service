# axa

https://github.com/publicprojects-io/axa

The Main X Index services are plain Axum, using
[axa](https://github.com/publicprojects-io/axa) for the plumbing they share:
routes, middleware, health checks, errors, configuration, logging, graceful
shutdown, the command line, background jobs, and test helpers. They are
backend-only, with no view or template tier.

axa is a library, not a framework. Each service's `src/app.rs` holds ordinary
functions, and its `main.rs` calls them. axa's own specification is
[`spec/index.md`](https://github.com/publicprojects-io/axa/blob/main/spec/index.md).

Services take it as a pinned git dependency.

## Stack

Use:

- Tantivy search engine
- OpenTelemetry metrics
- Prometheus metrics
- Podman containerization with Debian 13 slim

## Configuration

Settings come from the environment, with each service's defaults stated in code
in `src/app.rs`. There are no config files. The variables are listed in the
`axa::config` rustdoc, and [configuration.md](configuration.md) covers the
service-specific ones.

The environment is `APP_ENV`. `LOCO_ENV` is still read.

## Background jobs

Jobs run on a Postgres queue, the `axa_queue` table, using `FOR UPDATE SKIP
LOCKED`. There is no external broker, and no SQLite.

`start` runs only the server, so a service with jobs must run `start --all`, or
`start --worker` separately. Its image uses `start --all`, and it warns at boot
when nothing here serves the queue.

A job gets one attempt by default, because a retry repeats its work. Raise
`max_attempts` only for jobs written to be repeated. Finished rows are deleted
after seven days.

## Limits

Keep the service-wide body limit small. A route that needs more, such as a bulk
import, sets its own with `axa::middleware::body_limit`, and its own timeout
through `Http::timeout_overrides`.
