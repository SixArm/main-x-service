# Main X Index — example compose stacks

Three podman-compose stacks, in increasing scope, for local demos and
tutorials (DEP-1 in the repository root [`tasks.md`](../../tasks.md)).
None of these is `scripts/test-db.sh` — that is the per-crate **test**
database (`compose.test.yaml`, one Postgres per crate's own test suite);
these are **dev/demo** stacks that run the actual service binaries.

| File | What it brings up |
|---|---|
| [`single-service.yml`](single-service.yml) | One service (case-service) + its own Postgres — the minimal pattern, template for any other crate |
| [`full-family.yml`](full-family.yml) | All ten entity registries + authentication-service + link-graph-service, one shared Postgres (twelve databases) |
| [`enforced.yml`](enforced.yml) | An **override** on top of `full-family.yml`: `<ENTITY>_REQUIRE_AUTH` on everywhere, an ABAC policy mounted, PASETO key-fetch wired to authentication-service |
| [`authentication-dev.yml`](authentication-dev.yml) | authentication-service alone, in `LOCO_ENV=development` — the mode that logs a real magic-link URL to the console (SEC-A3) instead of emailing it, for anything that needs to complete a real passwordless sign-in without SMTP (person-front-end-with-svelte's live-integration suite is the reference consumer — `tests/integration/auth.setup.ts`) |

## Prerequisites

- Podman, not Docker (`agents/share/rust-loco-stack.md`). `podman
  compose` shells out to a compose provider (Homebrew's `docker-compose`
  on macOS) — the warning it prints on every invocation is expected.
- Run every command from **anywhere**; each file's `build.context` is
  `../..` resolved relative to the compose file itself (the repository
  root), not your current directory.
- **Build, then up — as two separate commands, always.** `up -d --build`
  was observed to hang indefinitely (no build subprocess, 0% CPU) under
  this compose provider on this repository's large build context.
  `build` followed by a plain `up -d` (which just reuses the image
  `build` already produced) does not hang. This is a compose-provider
  quirk, not a bug in these files or their Dockerfiles — see `tasks.md`
  DEP-1 for the full note.
- Building all twelve service images takes a while (each compiles in
  release mode); the single-service stack is far faster to try first.

## Quick start

```sh
# One service
podman compose -f examples/compose/single-service.yml build
podman compose -f examples/compose/single-service.yml up -d
curl http://localhost:8089/_health
podman compose -f examples/compose/single-service.yml down -v

# The full family (default-open auth, matching every crate's shipped default)
podman compose -f examples/compose/full-family.yml build
podman compose -f examples/compose/full-family.yml up -d
curl http://localhost:8081/api/health   # person
curl http://localhost:8091/_health      # authentication
podman compose -f examples/compose/full-family.yml down -v

# The enforced variant (ABAC guard on, same full family underneath)
podman compose -f examples/compose/full-family.yml -f examples/compose/enforced.yml build
podman compose -f examples/compose/full-family.yml -f examples/compose/enforced.yml up -d
curl http://localhost:8089/_health              # 200 — health stays public
curl http://localhost:8089/api/cases            # 401 — no bearer token
podman compose -f examples/compose/full-family.yml -f examples/compose/enforced.yml down -v

# authentication-service alone, in development mode (real magic-link
# sign-in without SMTP — see the table above)
podman compose -f examples/compose/authentication-dev.yml build
podman compose -f examples/compose/authentication-dev.yml up -d
curl -X POST http://localhost:5150/api/auth/signup \
  -H 'content-type: application/json' \
  -d '{"email":"you@example.test","name":"You","return_url":"http://localhost:4173"}'
podman compose -f examples/compose/authentication-dev.yml logs authentication-service | grep 'magic link issued'
podman compose -f examples/compose/authentication-dev.yml down -v
```

## Why a migrate-then-start step per service

Every crate's `config/production.yaml` sets `auto_migrate: false`
(link-graph is the one exception — see its comment in
`full-family.yml`), so a container that jumped straight to `start`
would serve against an empty schema. Each service therefore has a
one-shot `<service>-migrate` container (`command: ["db", "migrate",
"-e", "production"]`, same image, no port) that `depends_on:
condition: service_completed_successfully` gates the real service on.

## Why one shared Postgres in `full-family.yml`

Simplest to bring up and tear down for a demo; each service still owns
its own database and none shares tables (`init/10-databases.sh` creates
the twelve databases; `init/00-extensions.sh` enables the extensions
every migration expects, in `template1`, before those `CREATE DATABASE`
calls run, so every database inherits them). A real deployment would
more plausibly give each service its own Postgres instance, the way
every crate's own `compose.test.yaml` already does — this is a
demo-topology choice, not a claim about how to run this in production.

## What "enforced" honestly does and does not complete

See the header comment in [`enforced.yml`](enforced.yml). Turning on
`<ENTITY>_REQUIRE_AUTH` + mounting the ABAC policy + wiring the PASETO
key fetch is complete and works out of the box. Link-graph's
reconciliation pull against person/case's bulk `/links` endpoints is
**configured but not automatically authenticated**: those endpoints
require a real PASETO token with `access=admin` or `svc=true`, and
minting one needs a live pass through authentication-service's
passwordless magic-link flow — not something a static compose file can
script. The stack still works correctly without it (the read-model
updates from the event stream regardless; reconciliation is the
periodic integrity check on top, not the only path); the file explains
the one manual step to complete it.
