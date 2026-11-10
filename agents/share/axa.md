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

Services take it from crates.io, as `axa = "0.10"`. The request-level test
helpers are a separate crate, `axa-test = "0.1"`, taken as a dev-dependency:
import `axa_test::prelude::*` in a test, since `axa::prelude` does not bring in
`request`, `boot_test`, `TestApp`, or `TestServer`.

## Errors

Every error axa answers is an RFC 9457 problem, `application/problem+json`, with
`type`, `title`, and `status`, plus `detail` where there is more to say and the `code`
and `errors` extension members. Successes stay bare JSON. The older services build
their own errors the same way with `ApiProblem` (`spec/restful` §1.3), and FHIR errors
stay `OperationOutcome`. A client reads the reason from `detail`, and `title` when a `500`
has none.

An error with no `content-type`, or `text/plain`, becomes a problem with no `detail`,
so a handler returning `(StatusCode, "why")` loses the text: return an `Error`. A
violated unique or foreign key constraint answers `409`, not `500`.

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

## Production database

In production `DATABASE_URL` must set `sslmode` to `require`, `verify-ca`, or
`verify-full`, or the service refuses to start. `require` encrypts without checking
who answers, so use `verify-full` across a network. A loopback address or socket is
exempt. `DB_ALLOW_INSECURE=true` is the way out for a network that protects its own
traffic, such as one host's private bridge in `examples/compose`.

## Destructive commands

`db down`, `db reset`, and `db truncate` run only in development or test, with
`AXA_DESTROY_DATA=true`. `axa::cli::parse` refuses them before the service does
anything, and `--environment` cannot lift that.

## Extractors

Take `Json`, `Path`, and `Query` from `axa::extract`, not axum, in REST
handlers. A request they cannot read is answered with a problem whose
`detail` says what was wrong and quotes nothing the caller sent; axum's
own rejection reaches the caller with no `detail`. `axa::extract::Json`
is a response type too. `axa::prelude` still exports axum's, so import
these explicitly.

## Request limits

Each service layers `axa::ratelimit` through its `src/client_limits.rs`,
after its auth guard and inside `axa::middleware::apply`. Both limits are off
unless set: `RATE_LIMIT_PER_MINUTE` counts a client's requests, and
`LOCKOUT_AFTER_FAILURES` counts the `401`s it earns in fifteen minutes. A
client is the address axa resolved, so set `TRUSTED_PROXIES` behind a proxy,
or every client shares one limit. Each process counts for itself, so this
slows a noisy client and does not stop a distributed attack.

## Paging

List and search page by signed cursor through `axa::cursor_pagination`, the
`cursor-pagination` feature: `?limit=&cursor=`, `Link: rel="next"`, no total, and
a `422` for a bad limit or cursor. Each service signs with its `CURSOR_KEY`
through `src/cursor_key.rs`. Its `escape_like` escapes user text for `LIKE`.
Offset paging, `axa::offset_pagination`, is insecure and not used. See
[restful.md](restful.md) §Pagination.

## Router order

`App::router` builds the routes with `into_router`, adds the service's own layers in
`after_routes`, and calls `axa::middleware::apply` last. Anything layered after `apply`
sits outside it, so a response from that layer has no request id, security headers,
or CORS headers, and gets no panic or timeout protection. A guard's refusal sends only
`invalid token` or `forbidden`, and the reason goes to the log.

## Swagger UI

Serve it through `axa::openapi`, behind the `openapi` feature. The page names one exact
version with integrity hashes and sends `PAGE_CSP`, because axa's production default of
`default-src 'none'` would otherwise blank it.

## Limits

Keep the service-wide body limit small. A route that needs more, such as a bulk
import, sets its own with `axa::middleware::body_limit`, and its own timeout
through `Http::timeout_overrides`.
