# Getting started

By the end of this tutorial you will have **one Main X Index service —
case-service — running in Podman with its own Postgres**, and **its
operator front-end running against it in a browser**, where you create a
case with `curl` and then see it in the UI. This is the smallest possible
slice of the family; for how this one service fits into the other nine
(plus authentication and link-graph), see
[`agents/share/architecture.md`](../agents/share/architecture.md).

Two more things this tutorial deliberately does *not* cover: authentication
(the compose stack you're about to run has `CASE_REQUIRE_AUTH` off, its
default — see [TUT-3](#whats-next) below) and the other nine services (see
[TUT-2](#whats-next)).

## Prerequisites

| Tool | Why | Tested with |
|---|---|---|
| [Podman](https://podman.io/) (not Docker — see [`agents/share/rust-loco-stack.md`](../agents/share/rust-loco-stack.md)) | builds and runs the service + its Postgres | 6.0.2, with `podman machine` running |
| A `podman compose` provider | `podman compose` shells out to one; Homebrew's `docker-compose` on macOS | `docker-compose` 5.3.1 (prints a one-line notice on every invocation — expected) |
| [Node.js](https://nodejs.org/) 20+ and [pnpm](https://pnpm.io/) | runs the front-end dev server | Node v26.5.1, pnpm 11.0.8 |
| `curl` | verifies the backend directly | whatever your OS ships |

The service itself is Rust (this repo pins `1.96.1` in
[`rust-toolchain.toml`](../rust-toolchain.toml)), but you do not need Rust
installed for this tutorial — the container build compiles it for you.

Building the container image compiles the crate in release mode and took
about **3 minutes** on the machine this was verified on. If you're short on
disk space, note that a Rust release build inside a container can use
several GB of scratch space; free some up first if `podman compose build`
fails with "no space left on device".

## 1. Build and start the backend

The family ships a ready-made single-service compose file:
[`examples/compose/single-service.yml`](../examples/compose/single-service.yml).
It picks **case-service** as the representative example (see the comment
at the top of that file for why: case is the reference implementation for
several cross-cutting features, so it's the most representative single
service to read first) and publishes it on host port **8089**, with its
own Postgres reachable only inside the compose network (no host port).

Run these from the repository root, **as two separate commands**:

```sh
podman compose -f examples/compose/single-service.yml build
podman compose -f examples/compose/single-service.yml up -d
```

> **Why two commands, not `up -d --build`?** The compose file's own header
> comment documents a real, observed problem: `up -d --build` hangs
> indefinitely on this repository's build context under at least one
> host's compose provider (no build subprocess, 0% CPU) — see `tasks.md`
> DEP-1 for the full note. A plain `build` followed by a plain `up -d`
> (which just reuses the image `build` already produced) does not hang.
> This is a compose-provider quirk, not a bug in the compose file.

`up -d` starts three things: the Postgres container, a one-shot
`case-migrate` container that applies migrations and exits, and the
`case-service` container itself (which waits for the migration to finish
successfully before it starts).

## 2. Verify the backend is actually up

Don't take "started" on faith — check it.

```sh
curl http://localhost:8089/_health
```

```json
{"ok":true}
```

Now make a real API call — create a case. This body is copied verbatim
from [`examples/api/case.http`](../examples/api/case.http), which is
already verified against a running case-service:

```sh
curl -X POST http://localhost:8089/api/cases \
  -H "Content-Type: application/json" \
  -H "Accepts-version: 1.0" \
  -d '{
  "title": "Housing benefit appeal",
  "agency_id": "dwp",
  "case_number": "HB-2024-0007",
  "subjects": ["person:abc"],
  "keywords": ["housing", "benefit"],
  "identifiers": [{ "scheme": "Docket", "value": "CV-2024-001234" }]
}'
```

```json
{"pid":"7dbcb006-ffed-4548-ba99-c120bea1adc7","title":"Housing benefit appeal"}
```

(`Accepts-version: 1.0` is this family's header-based API version
negotiation — see
[`agents/share/api-versioning.md`](../agents/share/api-versioning.md); it's
optional here since `1.0` is also the default.)

Copy your own response's `pid` and fetch it back:

```sh
curl http://localhost:8089/api/cases/7dbcb006-ffed-4548-ba99-c120bea1adc7
```

```json
{"title":"Housing benefit appeal","alternate_titles":[],"case_number":"HB-2024-0007","agency_id":"dwp","agency_name":null,"case_type":null,"status":null,"priority":null,"opened_date":null,"subjects":["person:abc"],"keywords":["housing","benefit"],"identifiers":[{"scheme":"Docket","value":"CV-2024-001234"}],"same_as":[],"in_language":[]}
```

And the list endpoint, which reports pagination in response headers rather
than the body (see [`agents/share/restful.md`](../agents/share/restful.md)):

```sh
curl -i "http://localhost:8089/api/cases?limit=25&offset=0"
```

```
HTTP/1.1 200 OK
content-type: application/json
x-total-count: 1
x-limit: 25
x-offset: 0
...

[{"pid":"7dbcb006-ffed-4548-ba99-c120bea1adc7","title":"Housing benefit appeal"}]
```

The backend is genuinely up, migrated, and holding your case.

## 3. Run the front-end

The sibling operator UI lives at
[`case/case-front-end-with-svelte/`](../case/case-front-end-with-svelte/) —
a SvelteKit SPA. The browser never talks to case-service directly: the
SvelteKit server acts as a **Backend-For-Frontend (BFF)** and proxies
entity-API calls, so no token ever reaches browser JavaScript (see
[`agents/share/authentication-sessions.md`](../agents/share/authentication-sessions.md)
§6 for why).

```sh
cd case/case-front-end-with-svelte
pnpm install
pnpm check
```

`pnpm check` should report `0 ERRORS 0 WARNINGS`.

Point the BFF at the compose stack's published port (**8089**, not the
in-repo dev default of 5150) and start the dev server:

```sh
cat > .env <<'EOF'
CASE_API_URL=http://localhost:8089
AUTH_API_URL=http://localhost:8089
EOF
pnpm dev
```

> **Gotcha found while verifying this tutorial:** this front-end's own
> `.env.example` names the wrong variables (`PUBLIC_API_BASE_URL`,
> `VITE_AUTH_FRONTEND_URL` — leftovers from an earlier design). The
> variables the code actually reads
> (`src/lib/server/config.ts`) are `CASE_API_URL` and `AUTH_API_URL`,
> which is also what the front-end's own `README.md` table says. Use the
> `.env` above, not `.env.example`.

`pnpm dev` prints a `Local:` URL — open it:

```
http://localhost:5173/
```

In the browser:

1. Open **http://localhost:5173/** (or **/cases** for the data-grid view)
   — you should see the "Housing benefit appeal" case you created with
   `curl` above, because both `curl` and the browser are hitting the same
   backend on port 8089.
2. Click **New** (or go to `/new`) and create a second case through the
   form.
3. Click into a case's detail page and try **check-duplicates** — it
   posts the current record and lists any stored matches with their
   scores (this family's duplicate-detection machinery; see
   [`agents/share/match-search-merge.md`](../agents/share/match-search-merge.md)).

Signing in is not required for any of this — the compose stack runs with
`CASE_REQUIRE_AUTH` off (the family-wide default), so the BFF proxy
forwards every request unauthenticated and case-service accepts it.

If you'd rather confirm this from the command line instead of a browser,
the BFF proxy is real and reachable directly:

```sh
curl http://localhost:5173/api/proxy/api/cases
```

```json
[{"pid":"7dbcb006-ffed-4548-ba99-c120bea1adc7","title":"Housing benefit appeal"}]
```

— the same case, fetched through the front-end's own server rather than
case-service directly, proving the BFF wiring works end to end.

When you're done looking around, stop the dev server (`Ctrl-C`, or if you
backgrounded it, kill that process).

## 4. Tear down

```sh
# stop the front-end dev server (Ctrl-C if running in the foreground)

# stop and remove the backend containers, network, and volume
podman compose -f examples/compose/single-service.yml down -v
```

`down -v` also drops the Postgres volume, so the next `up -d` starts from
an empty database — exactly what you want for re-running this tutorial
from a clean slate.

## What's next

This tutorial covered the smallest useful slice: one service, one UI, one
record. The rest of the planned tutorial set (not yet written) goes
further:

- **TUT-2 — identity lifecycle**: create → 409-duplicate → check-duplicates
  → match → merge → audit trail, via `curl` and the UI.
- **TUT-3 — authentication & ABAC**: magic-link sign-in, session cookie,
  `POST /token`, a protected call, the 401/403 matrix, writing and
  hot-reloading an ABAC policy, and the `mask` obligation.
- **TUT-4 — cross-service linking**: `subject_of` and `same_identity`
  writes, then querying the link-graph aggregator's `neighbors` /
  `single-view` / `freshness`, plus a break-and-reconcile demo.
- **TUT-5 — bulk import/export**: fixture import (dry-run, error report),
  idempotent re-import, masked vs. full export.
- **TUT-6 — event bus**: outbox rows, the relay, `/events/recent`.

See [`tasks.md`](../tasks.md) for their current status.
