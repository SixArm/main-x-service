# Authentication and ABAC: magic link, sessions, tokens, and policy

This tutorial pairs **authentication-service** (the family's central SSO —
passwordless magic-link login, Postgres cookie sessions, PASETO v4.public
token issuance) with **case-service** as the protected peer. Case is not
an arbitrary choice: per
[`agents/share/authorization-attributes.md`](../agents/share/authorization-attributes.md)
§9/§11/§12, case is the family's **reference implementation** for
record-level ABAC (`resource.status`-gated write denial), the `mask`
obligation on a plain `GET`, and policy **hot-reload** (`CASE_ABAC_POLICY_FILE`
watched every 15 s, no restart needed).

You will: sign in with a magic link retrieved from the dev console log
(no real mailbox), confirm a session cookie is set, exchange the session
for a short-lived PASETO v4.public bearer token, turn on
`CASE_REQUIRE_AUTH` and walk the full 401/403 matrix as a caller's `attrs`
progress from nothing to `access=write` to `access=admin`, write and
**hot-reload** an ABAC policy that denies writes to a closed case, and
finish with a side-by-side full-vs-masked read driven by one allow rule's
`mask` obligation.

This tutorial does **not** cover Podman (TUT-1 covered the container
path; both services here run directly via `cargo run --`, which TUT-2
found is what actually works in this environment — confirmed again
below) or the other eight entity services (only case is wired to a
policy file in this walkthrough).

## Prerequisites

| Tool | Why | Tested with |
|---|---|---|
| [Podman](https://podman.io/) (not Docker) | only for the throwaway test Postgres instances | 6.0.2, with `podman machine` running |
| Rust (this repo pins `1.96.1` in [`rust-toolchain.toml`](../rust-toolchain.toml)) | builds and runs both services directly | `cargo` on `PATH` |
| `curl` + `python3` (for decoding PASETO payloads and pretty-printing JSON) | verifies everything live | whatever your OS ships |

## 1. Start two services at once

Both crates default to **port 5150** (loco's scaffold default) and to
**Postgres port 5432** for their throwaway test databases — running two
at once needs both moved for one of them.

Start each crate's own test Postgres, on two different host ports:

```sh
scripts/test-db.sh up authentication/authentication-service-with-loco
TEST_DB_PORT=5434 scripts/test-db.sh up case/case-service-with-loco
```

```
test-db: mxi-authentication-test-db ready
  DATABASE_URL=postgres://loco:loco@localhost:5432/authentication_service_test

test-db: mxi-case-test-db ready
  DATABASE_URL=postgres://loco:loco@localhost:5434/case_service_test
```

Migrate both (same `cargo run -- db migrate` pattern TUT-2 established —
`cargo loco db migrate` still doesn't work in this environment: no
`cargo-loco` shim is installed here, only the standalone `loco`
generator CLI, confirmed again for this tutorial):

```sh
cd authentication/authentication-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5432/authentication_service_test \
  cargo run -- db migrate
```

```
environment=development
Migration 'm20260728_000001_add_auth_event_mac' has been applied
```

```sh
cd case/case-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5434/case_service_test \
  cargo run -- db migrate
```

```
environment=development
Migration 'm20260803_000013_bulk_jobs' has been applied
```

Note `environment=development` in both — that matters in the next step.

### The port collision

`authentication-service`'s `config/development.yaml` fixes `server.port:
5150` with no templating, and so does case-service's. Rather than edit
either crate's tracked config, this tutorial uses loco's own documented
override mechanism: a `config/development.local.yaml` next to
`development.yaml`, deep-merged onto it at boot, and **already
gitignored** by this crate's own `.gitignore` (`**/config/*.local.yaml`)
— so it never needs to be staged or cleaned out of git:

```sh
cat > case/case-service-with-loco/config/development.local.yaml <<'EOF'
server:
  port: 5180
EOF
```

Start authentication-service (default port, default environment —
**development**, which matters for step 2):

```sh
cd authentication/authentication-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5432/authentication_service_test \
  cargo run -- start
```

```
environment: development
listening on http://localhost:5150
```

Start case-service in a second terminal, on its overridden port, with
auth **off** for now (the family-wide default — TUT-1 already covered
that baseline):

```sh
cd case/case-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5434/case_service_test \
  cargo run -- start
```

```
environment: development
listening on http://localhost:5180
```

Verify both are actually up:

```sh
curl -s http://localhost:5150/_health
curl -s http://localhost:5180/_health
```

```json
{"ok":true}
{"ok":true}
```

Create one case to use through the rest of this tutorial (unauthenticated
— `CASE_REQUIRE_AUTH` isn't on yet):

```sh
curl -s -X POST http://localhost:5180/api/cases \
  -H "Content-Type: application/json" -H "Accepts-version: 1.0" \
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
{"pid":"818d596b-90b9-45df-ad24-7dee386d6d18","title":"Housing benefit appeal"}
```

## 2. Magic-link sign-in — retrieved from the dev console log

Request a magic link the same way a real front-end would (no browser
needed — this is the raw handshake
[`examples/api/00-auth-handshake.http`](../examples/api/00-auth-handshake.http)
documents):

```sh
curl -s -X POST http://localhost:5150/api/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email":"demo@example.com","name":"Demo User"}'
```

```json
{}
```

Always `200 {}` regardless of outcome — a deliberate anti-enumeration
contract (`src/controllers/auth.rs`, SEC-A5/A6); it never reveals whether
the email was new. The magic link itself never appears in the HTTP
response. In this **locally-run, non-compose** setup the mechanism is
simpler than [EX-3](../examples/api/README.md) found for the shipped
compose stack: EX-3 had to spin up a **throwaway `LOCO_ENV=development`
container**, because the shipped `compose/full-family.yml` bakes
`LOCO_ENV=production` (SEC-A1's fail-closed default) into the image, and
in production the token is deliberately never logged (SEC-A3) — only
issuance is. Here, `cargo run -- start` with no `LOCO_ENV` set already
booted as `environment: development` (confirmed in step 1), which is
loco's own default when the flag is absent — so the token is right there
in the console log, no override needed:

```sh
grep "magic link issued" /tmp/auth-service.log | tail -1
```

```
magic link issued (dev: open the link, or GET /api/auth/magic-link/{token})
  email=demo@example.com locale=en
  magic_link=http://localhost:5173/verify?token=h8te7GMyNRKW7mqheTqjmFDo4FRXILwm
```

(`src/controllers/auth.rs::log_magic_link_url` gates this on
`Environment::Development` specifically — `Test` and any other named
environment stay silent too, only `development` logs the token.)

Consume it — `GET`, not `POST`, token in the path:

```sh
curl -s -i "http://localhost:5150/api/auth/magic-link/h8te7GMyNRKW7mqheTqjmFDo4FRXILwm"
```

```
HTTP/1.1 200 OK
content-type: application/json
set-cookie: __Host-mxi_session=0d56a60e-bbd6-468e-bc8a-fb6cd2cacbcb; HttpOnly; Secure; SameSite=Lax; Path=/
set-cookie: __Host-mxi_csrf=b3Bz6WaR0SEUc2ObD0tzCQw7MyvbUVmE; Secure; SameSite=Lax; Path=/

{"token":"v4.public.eyJ...","pid":"7d8a4a39-8967-4740-b890-9d51a0df188e","name":"Demo User","email":"demo@example.com","is_verified":true}
```

Two `Set-Cookie` headers, exactly as
[`authentication-sessions.md`](../agents/share/authentication-sessions.md)
§3 documents: `__Host-mxi_session` (`HttpOnly` — never readable from
browser JS) and `__Host-mxi_csrf` (readable — a BFF needs to echo it
back). The session is real and server-side from this point: this is the
"establish a session" step, not yet the cross-service token exchange.

## 3. Exchange the session for a PASETO v4.public token

The response body above already carries a working `LoginResponse.token`
(a real PASETO string — the doc-comment in `src/controllers/auth.rs`
calling it an "access token" is stale wording from before this design;
see `authentication-sessions.md`), but the **designed** cross-service
exchange is the explicit endpoint,
[`authentication-sessions.md`](../agents/share/authentication-sessions.md)
§5: a front-end's own server (the BFF) holds the session and calls this
whenever it needs a short-lived bearer for an outbound peer call.

```sh
curl -s -X POST http://localhost:5150/api/auth/token \
  -H "Cookie: __Host-mxi_session=0d56a60e-bbd6-468e-bc8a-fb6cd2cacbcb" \
  -H "X-CSRF-Token: b3Bz6WaR0SEUc2ObD0tzCQw7MyvbUVmE"
```

```json
{"token":"v4.public.eyJzdWIiOiI3ZDhhNGEzOS04OTY3LTQ3NDAtYjg5MC05ZDUxYTBkZjE4OGUiLCJlbWFpbCI6ImRlbW9AZXhhbXBsZS5jb20iLCJuYW1lIjoiRGVtbyBVc2VyIiwiaXNzIjoiYXV0aGVudGljYXRpb24tc2VydmljZSIsImF1ZCI6Im1haW4teC1zZXJ2aWNlIiwiZXhwIjoxNzg1ODMyODMxLCJpYXQiOjE3ODU4MzI1MzEsInNpZCI6IjBkNTZhNjBlLWJiZDYtNDY4ZS1iYzhhLWZiNmNkMmNhY2JjYiIsInNjb3BlIjpbXSwicm9sZXMiOltdfa...(signature+footer elided)"}
```

Both the cookie and the CSRF header are required (`src/csrf.rs`), or this
is a `403` (SEC-A10). Requires a fresh call after every attribute
assignment in this tutorial, since — see step 4 — assigning attributes
**revokes the session** (SEC-A8), so a stale session can't keep minting
tokens with old privileges.

Decoding the PASETO's JSON payload (v4.public is `payload+signature`,
base64url together — strip the trailing 64-byte Ed25519 signature before
parsing as JSON) shows the real claim shape:

```json
{"sub":"7d8a4a39-8967-4740-b890-9d51a0df188e","email":"demo@example.com",
 "name":"Demo User","iss":"authentication-service","aud":"main-x-service",
 "exp":1785832831,"iat":1785832531,
 "sid":"0d56a60e-bbd6-468e-bc8a-fb6cd2cacbcb","scope":[],"roles":[]}
```

`exp - iat = 300` — the ~5-minute lifetime
`authentication-sessions.md` §5 documents. Worth noting as a real,
live-verified detail: there is **no `attrs` key at all** in this payload
— not `"attrs":{}` — because this user has no attributes assigned yet.
`authorization-attributes.md` §3 documents this exactly ("absent claim ⇒
empty map"); this is what that looks like on the wire.

The published key set every peer verifies against, for reference (the
JWKS analogue, `/.well-known/paseto-keys`, no auth required):

```sh
curl -s http://localhost:5150/.well-known/paseto-keys
```

```json
{"keys":[{"crv":"Ed25519","kid":"ZbYGc9btiEvwHCwiLYKtoHQPKawzVdapJcgfF_R6J7g","kty":"OKP","use":"sig","x":"ebVWLo_mVPlAeLES6KmLp5AfhTrmlb7X4OORC60ElmQ"}]}
```

## 4. Turn on `CASE_REQUIRE_AUTH` — the 401/403 matrix

`CASE_REQUIRE_AUTH` is read **once at process boot**
(`agents/share/security.md` §4 — "the flag is read once at router
construction; changing it requires a restart"), so this needs a real
restart, not just an exported variable. Stop case-service (`Ctrl-C` or
kill the backgrounded process) and start it again with auth on, pointed
at authentication-service's published keys:

```sh
cd case/case-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5434/case_service_test \
  CASE_REQUIRE_AUTH=true \
  CASE_PASETO_KEYS_URL=http://localhost:5150/.well-known/paseto-keys \
  cargo run -- start
```

```
PASETO key set fetched over HTTP; fetched key set wins over the env key set
  url=http://localhost:5150/.well-known/paseto-keys keys=1
```

Confirms the boot-time fetch (`src/auth.rs::init`) actually reached the
running authentication-service and got one key back. No `CASE_ABAC_POLICY`
/ `_FILE` is set yet, so the **built-in default policy** applies
(`authorization-attributes.md` §5): `svc=true` ⇒ everything, `access=admin`
⇒ destructive+write, `access=write` ⇒ write, otherwise read-only.

**No token:**

```sh
curl -s -i http://localhost:5180/api/cases/818d596b-90b9-45df-ad24-7dee386d6d18
curl -s -i -X POST http://localhost:5180/api/cases -d '{"title":"x"}'
```

```
HTTP/1.1 401 Unauthorized
missing authorization header
```

for both — a plain-text body, not the usual JSON error envelope (worth
knowing if you're scripting against this).

**A token with no `attrs` at all** (the one minted in step 3 — `demo`
still has no attributes assigned):

```sh
TOKEN=v4.public.eyJ...   # from step 3
curl -s -i http://localhost:5180/api/cases/818d596b-90b9-45df-ad24-7dee386d6d18 \
  -H "Authorization: Bearer $TOKEN"
curl -s -i -X POST http://localhost:5180/api/cases \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Blank-attrs case","agency_id":"dwp","case_number":"BA-1"}'
```

```
GET  → HTTP/1.1 200 OK           (default-allow-read)
POST → HTTP/1.1 403 Forbidden     "default deny"
```

Exactly the §5 default decision: read allowed, every other action denied
absent an explicit rule.

### Minting tokens with specific `attrs`: the `user_attributes` CLI task

`authorization-attributes.md` §6 names two operator surfaces: an HTTP
admin API (gated on `access=admin` — unusable to *bootstrap* the very
first admin) and a **CLI task**, `user_attributes`, which writes directly
to `users.attributes` with no auth check of its own (it's a local
operator tool, not an HTTP endpoint). That's the one that actually works
here:

```sh
cd authentication/authentication-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5432/authentication_service_test \
  cargo run -- task user_attributes op:set email:demo@example.com key:access values:write
```

```
user 7d8a4a39-8967-4740-b890-9d51a0df188e <demo@example.com>
before:
(none — read-only under the default policy)
after:
  access = write
```

This **revokes every existing session for the user** (SEC-A8, so a live
session can't keep minting tokens carrying the old attributes) — a fresh
magic-link sign-in is required to get a session (and therefore a token)
that snapshots the new attributes:

```sh
curl -s -X POST http://localhost:5150/api/auth/magic-link \
  -H "Content-Type: application/json" -d '{"email":"demo@example.com"}'
grep "magic link issued" /tmp/auth-service.log | tail -1
# … verify the new token, exchange for a bearer, as in steps 2-3 …
```

Decoding the new token's payload now shows the claim:

```json
"attrs":{"access":["write"]}
```

**`access=write`:**

```sh
curl -s -i -X POST http://localhost:5180/api/cases \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Write-access case","agency_id":"dwp","case_number":"WA-1"}'
```

```
HTTP/1.1 200 OK
{"pid":"8ae78fa2-686e-445e-b32e-97a9647f2bfc","title":"Write-access case"}
```

```sh
curl -s -i -X PUT http://localhost:5180/api/cases/8ae78fa2-686e-445e-b32e-97a9647f2bfc \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Write-access case (amended)","agency_id":"dwp","case_number":"WA-1"}'
```

```
HTTP/1.1 200 OK
```

```sh
curl -s -i -X DELETE http://localhost:5180/api/cases/8ae78fa2-686e-445e-b32e-97a9647f2bfc \
  -H "Authorization: Bearer $TOKEN"
```

```
HTTP/1.1 403 Forbidden
default deny
```

`POST`/`PUT` derive `Action::Write`; `DELETE` derives `Action::Delete` — a
**distinct** action (`authorization-attributes.md` §2: "delete implies
destructive; a rule targeting destructive covers both" — but a rule
granting only `write` does **not** cover `delete`). The built-in default
policy's `access=write` rule lists `["write"]` only, so delete stays
denied even though create/update succeeded.

**`access=admin`** (same CLI task, then a fresh sign-in, same pattern):

```sh
cargo run -- task user_attributes op:set email:demo@example.com key:access values:admin
# … fresh magic-link sign-in + token exchange …
curl -s -i -X DELETE http://localhost:5180/api/cases/8ae78fa2-686e-445e-b32e-97a9647f2bfc \
  -H "Authorization: Bearer $TOKEN"
```

```
HTTP/1.1 200 OK
```

```sh
curl -s -i http://localhost:5180/api/cases/8ae78fa2-686e-445e-b32e-97a9647f2bfc \
  -H "Authorization: Bearer $TOKEN"
```

```
HTTP/1.1 404 Not Found
```

The full matrix, all four rows live-verified on one running pair of
services:

| Caller | `GET` | `POST`/`PUT` | `DELETE` |
|---|---|---|---|
| no token | 401 | 401 | 401 |
| token, no `attrs` | 200 (default-allow-read) | 403 | 403 |
| `access=write` | 200 | 200 | 403 |
| `access=admin` | 200 | 200 | 200 |

## 5. Write and hot-reload an ABAC policy

[`examples/policies/closed-case-write-deny.json`](../examples/policies/closed-case-write-deny.json)
(EX-2's cookbook) is the one built for this: an `access=admin` override
allow, then a `deny` on `write` when the loaded record's
`resource.status` is `closed`. Per the task brief, this tutorial points
`CASE_ABAC_POLICY_FILE` at a **copy**, not the repo's own example file,
so nothing here asks a reader to edit a tracked file:

```sh
mkdir -p /tmp/tut3-scratch
cp examples/policies/closed-case-write-deny.json /tmp/tut3-scratch/case-abac-policy.json
```

### A finding worth knowing before using this file: it denies *every* non-admin write, not just closed-case writes

Loading the cookbook file **exactly as shipped** and trying a plain
`access=write` `POST /api/cases` — on a case that doesn't even exist yet,
so `resource.status` is empty either way — live-verifies as `403 default
deny`, not `200`:

```sh
# CASE_ABAC_POLICY_FILE=/tmp/tut3-scratch/case-abac-policy.json (unmodified copy)
curl -s -i -X POST http://localhost:5180/api/cases \
  -H "Authorization: Bearer $WRITE_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Benefits review","agency_id":"dwp","case_number":"BR-1"}'
```

```
HTTP/1.1 403 Forbidden
default deny
```

The reason, reasoning it through against the engine
(`authentication-verifier`'s `evaluate_with_context`, confirmed by this
live result): a **configured** policy file *replaces* the built-in
default policy outright — it does not layer on top of it. This
particular cookbook file's two rules are `access=admin ⇒ allow
write+destructive` and `resource.status=closed ⇒ deny write`; neither
rule ever matches an `access=write` (non-admin) caller, so every write
falls through to the engine's fallback **default decision** (§5:
"anything other than read ⇒ deny"), whether or not the case is closed.
The cookbook file is written to be **composed** with a base grant, not
deployed standalone — its own `examples/policies/README.md` entry
doesn't say this explicitly, worth knowing if you reach for it directly.

This tutorial's copy adds the missing base grant as a third rule —
exactly what a deployment starting from the built-in default and adding
the closed-case override would write:

```sh
cat > /tmp/tut3-scratch/case-abac-policy.json <<'EOF'
{
  "rules": [
    { "effect": "allow", "actions": ["write", "destructive"], "when": { "access": ["admin"] } },
    { "effect": "deny",  "actions": ["write"], "when": { "resource.status": ["closed"] } },
    { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
  ]
}
EOF
```

**Rule order matters here**: the closed-deny must come *before* the
general write-allow, or the general allow would win first-match and the
closed-case restriction would never fire.

Restart case-service once more with this policy wired in:

```sh
cd case/case-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5434/case_service_test \
  CASE_REQUIRE_AUTH=true \
  CASE_PASETO_KEYS_URL=http://localhost:5150/.well-known/paseto-keys \
  CASE_ABAC_POLICY_FILE=/tmp/tut3-scratch/case-abac-policy.json \
  cargo run -- start
```

```
watching CASE_ABAC_POLICY_FILE for changes secs=15
```

A fresh `access=write` caller (a second demo user, `caseworker@example.com`,
kept distinct from the admin `demo@example.com` used above so the two
roles don't collide) can now create and update the case:

```sh
curl -s -X POST http://localhost:5180/api/cases \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Benefits review","agency_id":"dwp","case_number":"BR-1"}'
```

```json
{"pid":"4ef27633-8edc-4784-bd7d-736226762522","title":"Benefits review"}
```

### A second finding: `status` on the wire is `"Closed"`, not `"closed"`

The natural next step — transition the case toward closed — starts with
the body [`examples/api/case.http`](../examples/api/case.http)'s own
`PUT` example suggests, `"status": "in_progress"`. That example isn't
marked curl-verified the way its neighbours are, and it turns out not to
be right:

```sh
curl -s -X PUT http://localhost:5180/api/cases/4ef27633-8edc-4784-bd7d-736226762522 \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Benefits review","agency_id":"dwp","case_number":"BR-1","status":"open"}'
```

```json
{"error":"Bad Request"}
```

A generic `400`-shaped rejection, not the handler's own `422` validator
(which would name the problem) — this is Axum's `Json` extractor refusing
to deserialize the body at all, before the handler ever runs.
`case_matcher::CaseStatus` has **no** `#[serde(rename_all)]`, so its wire
representation is plain Rust-derive `Serialize`/`Deserialize` — the exact
variant name, `"Open"` / `"Closed"` / `"InProgress"` / etc.:

```sh
curl -s -i -X PUT http://localhost:5180/api/cases/4ef27633-8edc-4784-bd7d-736226762522 \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Benefits review","agency_id":"dwp","case_number":"BR-1","status":"Open"}'
```

```
HTTP/1.1 200 OK
```

This is unrelated to the **lowercase** tokens
`case_resource_attrs`/`status_token` derive internally for ABAC matching
(`resource.status: ["closed"]`, all lowercase) — those are a separate,
policy-facing vocabulary the service computes *from* the stored
`CaseStatus`, not what you send it. Not fixed here (out of scope — only
`tutorials/` and `tasks.md` are staged by this task), just documented so
the next reader doesn't lose time to it. Proceeding with the correct
casing:

```sh
curl -s -X PUT http://localhost:5180/api/cases/4ef27633-8edc-4784-bd7d-736226762522 \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Benefits review","agency_id":"dwp","case_number":"BR-1","status":"Closed"}'
```

```json
{"pid":"4ef27633-8edc-4784-bd7d-736226762522","title":"Benefits review"}
```

```sh
curl -s http://localhost:5180/api/cases/4ef27633-8edc-4784-bd7d-736226762522 \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" | python3 -m json.tool
```

```json
{"title":"Benefits review", "...":"...", "status":"Closed", "...":"..."}
```

Now the same caseworker's further `PUT` is evaluated against the
**stored** (now `Closed`) case, per `update`'s own doc comment ("uses the
*existing* stored case's attributes"):

```sh
curl -s -i -X PUT http://localhost:5180/api/cases/4ef27633-8edc-4784-bd7d-736226762522 \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Benefits review (amended again)","agency_id":"dwp","case_number":"BR-1","status":"Closed"}'
```

```
HTTP/1.1 403 Forbidden
{"error":"forbidden","description":"deny (rule 1)"}
```

`deny (rule 1)` — the second rule (0-indexed), the closed-case deny,
naming itself as the deciding rule exactly as
`authorization-attributes.md` §5 promises for a `403` body.

### Hot-reload: grant the override without restarting

Edit the **same** policy file — case-service is already watching it
(`spawn_policy_watcher`, `src/auth.rs`, 15 s mtime poll) — to grant the
override: move the general `access=write` allow rule **ahead of** the
closed-case deny, so it wins first-match regardless of status:

```sh
cat > /tmp/tut3-scratch/case-abac-policy.json <<'EOF'
{
  "rules": [
    { "effect": "allow", "actions": ["write", "destructive"], "when": { "access": ["admin"] } },
    { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } },
    { "effect": "deny",  "actions": ["write"], "when": { "resource.status": ["closed"] } }
  ]
}
EOF
```

Polling the same `PUT` every second, with **no restart of case-service**
in between:

```
t+0s -> 403
t+1s -> 403
t+2s -> 403
t+3s -> 403
t+4s -> 200
RESULT: override took effect ~4s after the file edit, no restart
```

The case-service log confirms the reload:

```
ABAC policy reloaded
```

Repeating the same measurement (edit → poll) a second time (reverting to
the closed-deny-wins order, then back) landed the reload at similar
short delays each time — **4-6 s observed in this run**, well under the
nominal 15 s poll interval `POLICY_WATCH_SECS` names, because the edit
happened to land shortly before the watcher's next tick each time. The
honest worst case is "just under 15 s" — a change made right after a
tick waits nearly the full interval for the next one.

## 6. The `mask` obligation

[`examples/policies/masked-read-obligation.json`](../examples/policies/masked-read-obligation.json):
full `read` for `dept=cardiology`, and a fallback `read` for **everyone
else** carrying the `mask` obligation. A second demo user gets the
`dept=cardiology` attribute (same CLI task as before, and same
sign-in-again-because-attribute-change-revoked-the-session pattern):

```sh
curl -s -X POST http://localhost:5150/api/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email":"cardiology@example.com","name":"Cardiology Reviewer"}'
cargo run -- task user_attributes op:set email:cardiology@example.com key:dept values:cardiology
# … magic-link sign-in + token exchange, as before, yields $CARDIOLOGY_TOKEN …
```

Replace the policy file's contents with the mask cookbook entry and
restart case-service (a clean restart here, rather than another
hot-reload edit, since this swaps topic entirely from the write-policy
demo):

```sh
cp examples/policies/masked-read-obligation.json /tmp/tut3-scratch/case-abac-policy.json
# restart case-service with the same CASE_REQUIRE_AUTH / CASE_PASETO_KEYS_URL,
# CASE_ABAC_POLICY_FILE now pointed at the mask policy
```

This policy grants **no write rule at all** — trying to create a case
under it 403s regardless of caller:

```sh
curl -s -i -X POST http://localhost:5180/api/cases \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Cardiac referral appeal","agency_id":"nhs","case_number":"CR-2026-0042","subjects":["person:11111111-1111-1111-1111-111111111111"],"keywords":["cardiology","referral"],"identifiers":[{"scheme":"Docket","value":"NHS-CR-0042"}]}'
```

```
HTTP/1.1 403 Forbidden
default deny
```

So a case worth masking needs creating **first** — this tutorial reuses
the hot-reload watcher from step 5 for exactly that: edit the *same*
policy file back to a write-granting version, wait for the reload, create
the case, then edit it back to the mask policy:

```sh
cat > /tmp/tut3-scratch/case-abac-policy.json <<'EOF'
{
  "rules": [
    { "effect": "allow", "actions": ["write", "destructive"], "when": { "access": ["admin"] } },
    { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
  ]
}
EOF
# poll the same POST every second until it stops 403ing — took 7s this run
curl -s -X POST http://localhost:5180/api/cases \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Cardiac referral appeal","agency_id":"nhs","case_number":"CR-2026-0042","subjects":["person:11111111-1111-1111-1111-111111111111"],"keywords":["cardiology","referral"],"identifiers":[{"scheme":"Docket","value":"NHS-CR-0042"}]}'
```

```json
{"pid":"3fbd8792-4278-4167-999e-012cef9979d5","title":"Cardiac referral appeal"}
```

```sh
cp examples/policies/masked-read-obligation.json /tmp/tut3-scratch/case-abac-policy.json
# poll a GET on the new case until case_number comes back null — took ~10s
# this run: a second real data point alongside step 5's ~4-6s, both comfortably
# inside the nominal 15s POLICY_WATCH_SECS ceiling
```

The same case, read by both — real side-by-side output:

```sh
curl -s http://localhost:5180/api/cases/3fbd8792-4278-4167-999e-012cef9979d5 \
  -H "Authorization: Bearer $CARDIOLOGY_TOKEN" | python3 -m json.tool
```

```json
{
    "title": "Cardiac referral appeal",
    "case_number": "CR-2026-0042",
    "agency_id": "nhs",
    "subjects": ["person:11111111-1111-1111-1111-111111111111"],
    "keywords": ["cardiology", "referral"],
    "identifiers": [{"scheme": "Docket", "value": "NHS-CR-0042"}],
    "same_as": []
}
```

```sh
curl -s http://localhost:5180/api/cases/3fbd8792-4278-4167-999e-012cef9979d5 \
  -H "Authorization: Bearer $CASEWORKER_TOKEN" | python3 -m json.tool
```

```json
{
    "title": "Cardiac referral appeal",
    "case_number": null,
    "agency_id": "nhs",
    "subjects": [],
    "keywords": ["cardiology", "referral"],
    "identifiers": [],
    "same_as": []
}
```

(fields shown trimmed to the interesting ones — full responses also carry
`alternate_titles`/`agency_name`/`case_type`/`status`/`priority`/
`opened_date`/`in_language`, unchanged either way). Exactly
`mask_case`'s documented redaction
(`case/case-service-with-loco/src/controllers/cases.rs`): `subjects`,
`identifiers`, and `same_as` zeroed, `case_number` nulled, everything
else — including `keywords`, deliberately less identifying — intact. Same
case, same endpoint, same HTTP status (`200`, not a separate masked
route): the **caseworker** token has `access=write` but **no `dept`**,
so it falls through cardiology's full-read rule straight into the
catch-all `read` rule (`"when": {}` matches any authenticated subject)
and gets the `mask` obligation. `access` tier is irrelevant to this
policy entirely — an `access=admin` caller with no `dept=cardiology`
would be masked here too, since this policy only ever inspects `dept`.

## 7. Tear down

```sh
# stop both cargo run -- start processes (Ctrl-C, or kill the backgrounded PIDs)

rm -f case/case-service-with-loco/config/development.local.yaml
rm -rf /tmp/tut3-scratch

scripts/test-db.sh down authentication/authentication-service-with-loco
TEST_DB_PORT=5434 scripts/test-db.sh down case/case-service-with-loco
```

`development.local.yaml` was already gitignored and never staged; this
just removes it from the working tree so a later `cargo run -- start`
(this tutorial or another) goes back to case-service's tracked default
port.

## What's next

- **TUT-4 — cross-service linking**: `subject_of` and `same_identity`
  writes, then querying the link-graph aggregator's `neighbors` /
  `single-view` / `freshness`, plus a break-and-reconcile demo.
- **TUT-5 — bulk import/export**: fixture import (dry-run, error report),
  idempotent re-import, masked vs. full export.
- **TUT-6 — event bus**: outbox rows, the relay, `/events/recent`.

See [`tasks.md`](../tasks.md) for their current status.
