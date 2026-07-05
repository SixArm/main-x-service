# Scope

> Part of the [Loco edition specification](index.md). Project-level
> scope: [root scope](../../spec/scope.md).

## In scope

- A **JSON API** that tracks folders by **NHS Number** across cabinets,
  with the same domain shape as the Svelte sibling (cabinets, folders,
  move events).
- Folder CRUD: list/search/show/create + per-folder history.
- A `POST /api/moves` endpoint that records a move (writing both the
  Main Event Service audit log and the Main Thing Service cabinet
  pointer).
- A `POST /api/places` endpoint that proxies through to the Main Place
  Service.
- A free-text move audit log at `GET /api/moves`.
- Server-side validation (Modulus 11 NHS check, required fields).
- Aggregate stats at `GET /api/stats`; liveness at `GET /healthz`.

## Explicitly out of scope

- **Any built-in user interface.** Browser rendering, HTML, CSS,
  client-side JavaScript, design systems. Front-ends are separate
  projects that consume this API.
- Authentication, ABAC, single sign-on, smartcard, CIS2.
- Mailer, background workers, queues — Loco supports them but they're
  not needed here.
- Real-time websockets / SSE.
- Multi-tenancy.
- Anything that would push the application towards SaMD classification.
