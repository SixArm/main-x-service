# Runbook: standing up a registry for the first time

**The shipped default is wide open.** Every entity registry in this
family boots with no authentication, no authorization, and (on most)
no keyed integrity — by design, so the family can ship and integrate
before any one deployment's policy exists (`security.md` §4). This
runbook is the ordered checklist for the day that stops being true: the
day a registry becomes reachable by a caller you do not fully trust.
It is OPS-1's general slice; the other four runbooks are the specialised
ones — [`integrity-activation.md`](integrity-activation.md) (the audit
chain / MAC, four services only), [`paseto-key-rotation.md`](paseto-key-rotation.md),
[`reconciliation-divergence.md`](reconciliation-divergence.md), and
[`event-bus-outage-replay.md`](event-bus-outage-replay.md).

Every step below has a **verification command**, not a "confirm the env
var is set" checkbox. Reading a config file tells you what you intended;
it does not tell you what the running process actually loaded — the
integrity runbook's `mac_absent not falling` trap and this runbook's own
§6 finding (a config file that renders fine in `development.yaml` and
crashes boot in `production.yaml`, found only by actually starting a
container) are both cases where the file looked right and the process
was wrong.

Applies to every entity registry (person, worker, place, thing, event,
course, organization, care-pathway, case, portfolio). Substitute the
entity prefix throughout — `PERSON_`, `WORKER_`, `PLACE_`, `THING_`,
`EVENT_`, `COURSE_`, `ORGANIZATION_`, `CARE_PATHWAY_`, `CASE_`,
`PROJECT_PORTFOLIO_MANAGEMENT_` — **except** where a crate's own
env vars break that pattern; portfolio's integrity controls are the one
family-wide example (`PORTFOLIO_INTEGRITY_MAC_KEY`, not
`PROJECT_PORTFOLIO_MANAGEMENT_INTEGRITY_MAC_KEY` — check your crate's own
`AGENTS.md`/`src/compliance/mac.rs` rather than assume the long prefix
everywhere).

## What you get if you do nothing

| Control | Default | Consequence |
|---|---|---|
| `<E>_REQUIRE_AUTH` | **off** | `/api/*` (and `/fhir/*` where mounted) are open to any caller; only `/_health`, `/_ping`, the docs, and `/metrics.prom` are *meant* to be public — with the flag off, everything is |
| ABAC policy | the built-in default | applies only once `<E>_REQUIRE_AUTH` is on; until then it is inert |
| `<E>_PASETO_KEYS_URL` | unset | the peer falls back to a static `<E>_PASETO_KEYS` env value, or (if that's unset too) trusts no token at all — every request is then unauthenticated regardless of `REQUIRE_AUTH` |
| `<E>_EVENT_TRANSPORT` | `memory` | events are lost on restart; no durable outbox |
| integrity / audit MAC (4 services only) | off | see `integrity-activation.md` — not duplicated here |

## Activation order, and why it is an order

1. **Mount an ABAC policy before turning on the gate**, not after.
   `<E>_ABAC_POLICY` (inline JSON) or `<E>_ABAC_POLICY_FILE`. Skipping
   this is not "no policy" — it silently falls back to the built-in
   default (`authorization-attributes.md` §5: any authenticated caller
   reads, `access=write` writes, `access=admin` adds delete/merge,
   `svc=true` does everything). Decide whether that default is what you
   want *before* the gate goes live, because turning the gate on with no
   policy configured does not fail loud — it just uses the default.
2. **`<E>_REQUIRE_AUTH=1`.** Second, not first: turning this on with no
   PASETO key source configured (step 3) makes every request 401,
   including your own health checks against `/api/*` — `/_health` stays
   exempt, everything else does not.
3. **A PASETO key source** — `<E>_PASETO_KEYS_URL` pointed at
   authentication-service's `/.well-known/paseto-keys` (preferred; the
   registry then polls for rotations on `<E>_PASETO_KEYS_REFRESH_SECS`,
   default 3600s, per `paseto-key-rotation.md`), or a static
   `<E>_PASETO_KEYS` env value where no live auth-service is reachable
   from this deployment. Order relative to step 2 does not matter — a
   registry boots and serves `401`/`403` correctly with the gate on and
   no key source at all; it just cannot verify anything, so nothing
   reads or writes are ever `allow`ed. Set this before or alongside step
   2, not as an afterthought once callers start reporting failures.
4. **`<E>_EVENT_TRANSPORT=outbox`** if you need durable events (a
   committed change without its event is the failure mode the outbox
   exists to prevent — `event-bus.md`). Optional; many deployments run
   `memory` indefinitely.
5. **Integrity / audit** (care-pathway, case, person, worker, and
   portfolio's narrower row-integrity-only MAC) — its own runbook,
   `integrity-activation.md`. Do this after auth is live, for the same
   reason that runbook gives: an open `/audit/*` surface before auth is
   on hands an attacker the trail they'd want to inspect before editing.
6. **Any optional background loop your crate carries** — a scheduler
   sweep (`<E>_SCHEDULER_MINUTES`), a Prometheus gauge refresh
   (`<E>_FLOW_METRICS_SECS` on portfolio), the Fluvio relay
   (`<E>_FLUVIO_ENDPOINT`, requires the crate built with its `fluvio`
   feature — an endpoint set without the feature refuses to start the
   relay rather than silently falling back). Check your crate's own
   `AGENTS.md` for which of these it actually has; not every registry
   carries all of them, and this runbook does not enumerate them
   per-crate.

Every step above is read **once at process boot** (or, for the PASETO
key set and ABAC policy where a crate has adopted the `Reloadable*`
pattern, on a timer/file-watch — check whether yours has; not all have)
— a step you change without restarting has not taken effect no matter
how long you wait, unless you confirmed hot-reload applies.

## Verifying each step actually took effect

| Step | Command | Expected before | Expected after |
|---|---|---|---|
| Gate off (baseline) | `curl -s -o /dev/null -w '%{http_code}' http://<host>/api/<plural>` | `200` (open) | — |
| Gate on | same command, no `Authorization` header | — | `401` |
| Health stays public | `curl -s -o /dev/null -w '%{http_code}' http://<host>/_health` | `200` | `200` — unchanged either way |
| Metrics stays public | `curl -s -o /dev/null -w '%{http_code}' http://<host>/metrics.prom` | `200` | `200` — unchanged either way |
| A valid token is accepted | `curl -H "Authorization: Bearer <token>" http://<host>/api/<plural>/whoami` | `401` (no token concept without the gate meaning anything) | verified claims body, not `401`/`403` |
| ABAC policy loaded, not the default | attempt a mutating call with a token carrying only `access=write` against a rule your policy denies but the *default* policy would allow (or vice versa) | — | the response matches **your** policy's decision, not the built-in default's |
| PASETO keys resolved | grep boot log for `PASETO key set fetched over HTTP` (URL path) or confirm no `PASETO key-set refresh failed` lines appear later | — | present at boot; absent on every refresh tick |

The auth + gate checks above were run against a real container for
`project-portfolio-management-service` while writing this runbook (not
merely read from source): `GET /api/plans` returned `200` with the gate
off and no token, then `401` with `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH=1`
set and no token, while `/_health` and `/metrics.prom` stayed `200` in
both cases — exactly as the table above states.

## What "verified against a fresh container" found

Writing this runbook meant actually building and booting a registry
from a clean image rather than reading its `AGENTS.md`'s prior "verified
end-to-end" claim as still current. Two real, previously-undiscovered
defects turned up this way, both fixed in the same pass as this doc
(not left as a "found but not fixed" note):

1. **`config/production.yaml`'s dead `auth:` block crashed boot with no
   `JWT_SECRET` set**, in **every loco-idiomatic registry**
   (organization, care-pathway, case, portfolio) — `{{
   get_env(name="JWT_SECRET") }}` had no `default`, and this family
   issues PASETO v4.public, never JWT (`jwt.md`; `security.md` §7
   already removed loco's `auth` Cargo feature from every crate for the
   same reason). loco's own `Config.auth` field is `Option<Auth>` —
   safe to omit the block entirely, which is the fix, verified against
   a real container + real Postgres for all four crates named above.
   **The trap in fixing this**: a YAML `#` comment does not stop Tera
   from expanding a literal `{{ }}` template expression written inside
   it — an explanatory comment that requotes the broken line to explain
   it reproduces the exact crash it is describing.
2. **Two Dockerfiles had silently fallen out of sync with their own
   crate.** `project-portfolio-management-service`'s and
   `case-service`'s images never `COPY`'d `benches/`, and Cargo refuses
   to parse the manifest at all for a `[[bench]]`-declared crate without
   it — the same trap `place-service`'s own container fix already
   documented, not novel to this pass, just not yet rolled to these two.
   `care-pathway-service`'s image never `COPY`'d `link/entity-ref-rust-crate`
   at all, a sibling path dependency added 2026-08-24 (the `continues_as`
   journey-link write side) — after that crate's Dockerfile was last
   verified (2026-08-03), so the two silently diverged. All three are
   exactly why this runbook's own standard is "verified against a fresh
   container, not read" rather than "should still be true."

## Who owns each knob

**The deploying operator, every time.** There is no vendor-side
configuration in this family — every knob above is an environment
variable the operator sets, per the family's stated posture
(`overview.md`, `configuration.md`). A registry that ships pre-activated
for one deployment and open for another is not a supported mode; if you
need a different default for your fleet, bake it into your own image or
deployment manifest, not into the crate.
