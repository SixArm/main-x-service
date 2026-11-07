# API request collections

One `.http` file per service — the main endpoints, ready to run against a
local `examples/compose/full-family.yml` stack. Written in the plain-text
syntax both the VS Code **REST Client** extension and the **JetBrains HTTP
Client** understand: `@variable = value` for variables, `###` between
requests, `#`/`//` for comments, `{{variable}}` interpolation, and
(where a client supports it) `# @name requestName` +
`{{requestName.response.body.$.field}}` to chain a value from one
response into a later request.

## Files

| File | Service | Port |
|---|---|---|
| `00-auth-handshake.http` | walkthrough: magic-link → session → PASETO token → bearer call on a peer | 8091 (+ any peer) |
| `person.http` | person-service | 8081 |
| `worker.http` | worker-service | 8082 |
| `place.http` | place-service | 8083 |
| `thing.http` | thing-service | 8084 |
| `event.http` | event-service | 8085 |
| `course.http` | course-service | 8086 |
| `organization.http` | organization-service | 8087 |
| `care-pathway.http` | care-pathway-service | 8088 |
| `case.http` | case-service | 8089 |
| `portfolio.http` | project-portfolio-management-service | 8090 |
| `authentication.http` | authentication-service (the central SSO provider) | 8091 |
| `link-graph.http` | link-graph-service (read-only cross-service aggregator) | 8092 |

Each file covers CRUD plus that service's signature capabilities — search,
match, duplicate-check, merge, review-queue, cross-service links, bulk
import/export, FHIR — not every route it exposes. See each crate's own
`spec/index.md` for the complete surface.

## Running the stack

```sh
podman compose -f examples/compose/full-family.yml build
podman compose -f examples/compose/full-family.yml up -d
# wait for every *-service container to report healthy:
podman ps --format '{{.Names}}\t{{.Status}}'
```

Run `build` and `up` as **two separate commands** — `up -d --build` is
known to hang on this repository's large build context (see the header
comment in `examples/compose/full-family.yml`). Building all twelve
release images takes several minutes; each service's `*-migrate`
container must exit `0` before its `*-service` container starts, so give
it time rather than assuming a `curl` failure means something is wrong.

Tear down when done:

```sh
podman compose -f examples/compose/full-family.yml down -v
```

## Variables

Every file starts with a `@baseUrl` pointing at that service's mapped
host port (see the table above — the internal container port is `8080`
for the six older "person-style" crates and `5150`/`5160` for the newer
controller-style ones; the compose file remaps everything to `808x`/`809x`
on the host, so you never need to know the internal port).

Requests that need an id (a created record's `pid`, a review-queue item
id, an edge id, …) declare a `@placeholder-looking-like-a-uuid` variable
near the top — replace it with a real id from a prior response, or (in a
client that supports chaining) reference the earlier named request's
`response.body` directly.

## `<ENTITY>_REQUIRE_AUTH` — off by default

`examples/compose/full-family.yml` deliberately leaves every
`<ENTITY>_REQUIRE_AUTH` flag at its default (**off**) — see
[`agents/share/security.md`](../../agents/share/security.md) §4. That
means every request in every file below runs **unauthenticated** against
the stock compose file, `Authorization` headers included in the examples
are inert extra headers, not a live requirement, and this matches what
you'll see the first time you bring the stack up. `examples/compose/`
also ships an `enforced.yml` override that turns the flags on; layer it
on top of `full-family.yml` to exercise the commented-out `Authorization:
Bearer` lines for real.

## The auth handshake

`00-auth-handshake.http` walks the full chain from
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
§5: request a magic link, retrieve it in dev mode (no real mailbox —
the token is logged, see that file's comments), verify it to establish a
cookie session, exchange the session for a short-lived PASETO
`v4.public` token via `POST /api/auth/token` (CSRF-protected), then use
that token as a `Bearer` credential on a call to a peer entity service.
Run its requests in order — each later request depends on a cookie or
token a prior one produced.

## Response envelope — reads two ways across the family

Every service answers the same way ([`spec/restful`](../../spec/restful/index.md)
§1.3): a success is bare JSON, the resource or list object itself with no
wrapper, and a non-2xx is an RFC 9457 problem, `application/problem+json`
with `type`, `title`, `status`, a lowercase `code`, `detail` where there is
more to say, and `errors` for structured context. A `500` has no `detail`.
FHIR errors stay `OperationOutcome`.

One documented exception within a person-style crate: worker's
assessment endpoints (`POST/GET .../assessments`) bypass the envelope
entirely and return the resource bare — see the GOTCHA comment in
`worker.http`.

## Pagination and API versioning

List/search endpoints on the four newer controller-style services
(organization, care-pathway, case, portfolio) take `?limit=&offset=` and
report `X-Total-Count` / `X-Limit` / `X-Offset` response headers per
[`agents/share/restful.md`](../../agents/share/restful.md). The six
older crates take the same query params but report the total inside the
JSON body instead (no pagination headers there — confirmed per-crate, not
assumed).

Every file includes at least one request carrying
`Accepts-version: 1.0` to demonstrate the header-based versioning scheme
in [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md)
— omitting the header is equally valid and gets you the current version
by default.
