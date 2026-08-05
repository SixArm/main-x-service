# Environment variable reference

Every environment variable a Main X Index service reads at runtime, for
the twelve crates under `agents/share/overview.md`: the ten entity
registries (person, worker, place, thing, event, course, organization,
care-pathway, case, portfolio), authentication-service, and
link-graph-service. Grounded in `std::env::var` call sites and each
crate's `config/*.yaml` Tera templates as of 2026-08-03 — not in prior
docs, several of which had drifted (see §7).

This is a **reference**, not a tutorial. For how these variables fit
together operationally, see [`loco.md`](loco.md) (the config/Tera
mechanism), [`jwt-enforcement.md`](jwt-enforcement.md) (the activation
gate), [`authorization-attributes.md`](authorization-attributes.md)
(ABAC), [`authentication-sessions.md`](authentication-sessions.md)
(PASETO), [`event-bus.md`](event-bus.md) (the durable bus), and
[`cross-service-linking.md`](cross-service-linking.md) (link-graph).
`examples/compose/` shows a working set of these values end-to-end.

**Out of scope:** contact-relationship-management, content-management-
system, patient-flow, and workforce-planning-management (the four
consumer apps) follow the same `<ENTITY>_*` shape under their own
prefixes (`CRM_`, `CMS_`, `PATIENT_FLOW_`, `WPM_`) but are not swept
here — file a follow-up if this doc needs to cover them too.

## 1. Two independent config paths — read this first

**Every crate** (all twelve) boots through **loco's own Tera-templated
YAML** (`config/{development,production,test}.yaml`), which is where
`DATABASE_URL`, `PORT`, `HOST`, `JWT_SECRET`, and the `DB_*`/`SMTP_*`
variables in §2 are actually consumed (`{{ get_env(name="...",
default="...") }}` calls in the YAML, not a `std::env::var` in Rust —
easy for a naive grep to miss entirely).

**Six of them** — person, worker, place, thing, event, course, the
older "person-style" layout (`agents/share/architecture.md`) — **also**
run a second, independent `Config::from_env()` (`src/config/mod.rs`)
for their own `AppState`, with its **own** `SERVER_PORT`/`DATABASE_URL`
reads (§3). The two paths can disagree — e.g. loco's yaml binds the
Axum server on `PORT` while these six crates' own state-config also
reads `SERVER_PORT` for a value that, in the shipped Dockerfiles, only
matters to code paths outside the loco-served router. Don't assume
setting one env var reconfigures both surfaces in these six crates;
check which path the behaviour you're changing actually goes through.

## 2. Loco Tera-config variables (all twelve crates)

Read from `config/production.yaml` (the shape below is the same
default value only for entries the family agrees on — course and
link-graph both have documented one-off defaults, called out in the
table).

| Variable | Default | Effect |
|---|---|---|
| `DATABASE_URL` | **none in production** (boot fails if unset); dev/test default to `postgres://localhost/<entity>_service_{development,test}` | `database.uri` (and `queue.uri`, where the crate uses the Postgres-backed job queue) |
| `PORT` | `8080` (person, worker, place, thing, event) · `8084` (course) · `5150` (organization, care-pathway, case, portfolio, authentication) · `5160` (link-graph) | `server.port` |
| `HOST` | `http://localhost` | `server.host` |
| `JWT_SECRET` | **none** in production for every crate except course (`default="unused-course-service-does-not-issue-tokens"`); link-graph has no `auth:` block at all, so it is not read there | `auth.jwt.secret` — a loco config-schema requirement, **not** how any of these crates actually authenticate (that's PASETO, §4; see [`jwt.md`](jwt.md)) |
| `DB_CONNECT_TIMEOUT` | `500` (ms) | Pool connect timeout — organization, care-pathway, case, portfolio, authentication, link-graph, course only; person/worker/place/thing/event hardcode this in yaml |
| `DB_IDLE_TIMEOUT` | `500` (ms) | Pool idle timeout — same six crates |
| `DB_MIN_CONNECTIONS` | `2` (course/link-graph: `1`) | Pool floor — same six |
| `DB_MAX_CONNECTIONS` | `20` (link-graph: `10`) | Pool ceiling — same six |
| `SMTP_HOST` / `SMTP_PORT` | `localhost` / `587` | Mailer transport — organization, care-pathway, case, portfolio, authentication only (person/worker/place/thing/event/course/link-graph disable mail or have no mailer block at all) |
| `SMTP_USER` / `SMTP_PASSWORD` | `""` / `""` | Mailer auth — same five crates. Must be **quoted** in the YAML (`"{{ get_env(...) }}"`) — an unset var renders empty, and an unquoted empty value after `key: ` is YAML `null`, not `""`, which fails loco's `SmtpAuth: String` schema at boot |

## 3. Person-family `Config::from_env()` (person, worker, place, thing, event, course only)

A second, independent config surface these six crates alone read,
via `dotenvy::dotenv()` best-effort plus `std::env::var` in
`src/config/mod.rs`. A malformed **typed** value here (e.g.
`SERVER_PORT=not-a-number`) is a boot error, not a silently-ignored
default.

| Variable | Default | Effect |
|---|---|---|
| `DATABASE_URL` | `postgres://localhost/<entity>_service` | `database.url` on this crate's own `AppState` — a second, separate consumer of the same var name as §2 |
| `DATABASE_MAX_CONNECTIONS` | `10` | Pool ceiling (this surface) |
| `DATABASE_MIN_CONNECTIONS` | `2` | Pool floor (this surface) |
| `SERVER_HOST` | `0.0.0.0` | Bind host (this surface) |
| `SERVER_PORT` | `8080` | Bind port (this surface — distinct from §2's `PORT`) |
| `GRPC_PORT` | `50051` | gRPC listener (person, worker, event only carry a live gRPC stub) |
| `SEARCH_INDEX_PATH` | `./data/search_index` | Tantivy index directory |
| `SEARCH_CACHE_SIZE_MB` | `512` | Tantivy cache budget |
| `MATCHING_THRESHOLD` | `0.85` (`0.7` in some crates' own docs — check the crate) | Probabilistic match cutoff |
| `OTLP_SERVICE_NAME` | `<entity>-service` | OTel `service.name`. **Live in `link-graph-service` only** (`src/observability.rs`, 2026-08-05 — the family's only working exporter); in the ten entity registries the resource is built and never exported, see `overview.md`'s observability note |
| `OTLP_ENDPOINT` | `http://localhost:4317` | OTLP/gRPC collector endpoint, same caveat. In link-graph this is **on by default** and export is disabled by setting it to the **empty string** — there is no separate activation flag |
| `RUST_LOG` | `info` | Log level — also read directly by `tracing_subscriber` itself in every crate, not just this config struct |
| `STREAMING_BROKER_URL` | `localhost:9003` | Legacy broker-URL field predating the durable-bus `FLUVIO_ENDPOINT` (§5) — check which your change actually needs |
| `STREAMING_TOPIC` | `<entity>-events` | Legacy topic field, same caveat |

## 4. Auth: PASETO verification + ABAC (all ten entity services + link-graph)

Every one of these is prefixed by the crate's own token (§6). Not
present on authentication-service, which **issues** rather than
verifies (its own vars are in §8).

| Pattern | Default | Effect |
|---|---|---|
| `<P>_REQUIRE_AUTH` | **off** (falsy/unset); truthy = `1`/`true`/`yes`/`on`, case-insensitive | Blanket bearer-auth enforcement. Read **once**, cached in a `OnceLock` — changing it needs a restart. [`jwt-enforcement.md`](jwt-enforcement.md) |
| `<P>_PASETO_KEYS` | `{"keys":[]}` (rejects every token) | Inline JSON PASETO public-key set |
| `<P>_PASETO_KEYS_URL` | unset ⇒ the env-built verifier stands | Fetch authentication-service's published key set at boot; **wins** over `_PASETO_KEYS` on success, falls back to it on failure (the service always boots) |
| `<P>_PASETO_KEYS_REFRESH_SECS` | `3600`; `0` disables the refresh loop | Key-rotation poll interval, only meaningful with `_PASETO_KEYS_URL` set |
| `<P>_TOKEN_ISSUER` | `authentication-service` | Expected `iss` claim |
| `<P>_TOKEN_AUDIENCE` | `main-x-service` | Expected `aud` claim |
| `<P>_ABAC_POLICY` | built-in default policy | Inline JSON policy (wins over `_FILE`) |
| `<P>_ABAC_POLICY_FILE` | unset ⇒ inline/built-in policy | Policy file path; also enables a 15 s mtime-poll hot-reload watcher |

[`authentication-sessions.md`](authentication-sessions.md) §5 and
[`authorization-attributes.md`](authorization-attributes.md) are the
design docs behind this table.

## 5. Durable event bus (all ten entity services; not link-graph, not authentication)

| Pattern | Default | Effect |
|---|---|---|
| `<P>_EVENT_TRANSPORT` | `memory` (unset or unrecognised) | `memory` vs `outbox` transport. `outbox` also makes the mutation's audit write fail-closed inside the same transaction |
| `<P>_EVENT_RELAY` | off | Run the outbox → sink relay loop |
| `<P>_EVENT_RELAY_INTERVAL_SECS` | `5`, floored at `1` | Relay poll interval |
| `<P>_EVENT_RETENTION_DAYS` | `7` | Outbox row retention (Phase-3 cleanup) |
| `<P>_FLUVIO_ENDPOINT` | unset/blank ⇒ `LoggingSink` (no real broker); **set without the crate's `fluvio` Cargo feature ⇒ refuses to start the relay** (logged error, not a silent no-broker fallback) | Real-broker relay sink |
| `<P>_EVENT_TOPIC` | `mxi.<entity>.events` — **except portfolio, which publishes to `mxi.plan.events`** (the entity's actual streaming/topic token is `plan`, not `portfolio` — see `overview.md`'s dual-ENTITY-constant note) | Publish topic |

[`event-bus.md`](event-bus.md) is the design doc.

## 6. Prefix table — and where it is NOT what you'd guess

| Crate | Prefix | Note |
|---|---|---|
| person | `PERSON_` | |
| worker | `WORKER_` | |
| place | `PLACE_` | |
| thing | `THING_` | |
| event | `EVENT_` | **Doubles up**: `EVENT_EVENT_TRANSPORT`, `EVENT_EVENT_RELAY`, `EVENT_EVENT_RELAY_INTERVAL_SECS`, `EVENT_EVENT_RETENTION_DAYS`, `EVENT_EVENT_TOPIC` are correct as written — the crate prefix `EVENT_` composes with the pattern's own `EVENT_*` name. Do not "fix" these to single `EVENT_` in a future edit. |
| course | `COURSE_` | |
| organization | `ORGANIZATION_` | |
| care-pathway | `CARE_PATHWAY_` | |
| case | `CASE_` | |
| portfolio | `PROJECT_PORTFOLIO_MANAGEMENT_` | **Except** the integrity-MAC family (§9), which uses `PORTFOLIO_INTEGRITY_MAC_*` — a real, standing inconsistency, not a typo to silently correct without checking `src/compliance/mac.rs`'s `KeyConfig::new(..., "PORTFOLIO")` call first. |
| authentication | *(unprefixed)* `TOKEN_*` / `AUTH_*` | See §8 |
| link-graph | `LINK_GRAPH_` | Per-entity vars additionally suffix an entity token (§7): `LINK_GRAPH_RECONCILE_URL_<ENTITY>`, `LINK_GRAPH_PROBE_URL_<ENTITY>`, `LINK_GRAPH_SUGGEST_URL_<ENTITY>` (T-31, only `PERSON`/`WORKER` in use). Those tokens are `PERSON`, `WORKER`, `ORGANIZATION`, `CASE`, `PLACE`, `THING`, `EVENT`, `COURSE`, `COURSEINSTANCE` (no underscore), `CARE_PATHWAY` (with one) — the uppercased `EntityType::as_str()` from the shared `entity-ref` crate, not the service's own compose/topic prefix. |

## 7. Link-graph — consumption, reconciliation, probing

Beyond §4's auth set (`LINK_GRAPH_REQUIRE_AUTH`, `_PASETO_KEYS[_URL]`,
`_ABAC_POLICY[_FILE]`, `_TOKEN_ISSUER`/`_AUDIENCE`):

| Variable | Default | Effect |
|---|---|---|
| `LINK_GRAPH_FLUVIO_ENDPOINT` | unset ⇒ no bus consumption at all | Broker the consumer reads all ten entity topics from |
| `LINK_GRAPH_PROCESSED_EVENTS_RETENTION_DAYS` | `7` | `processed_events` dedup-table retention |
| `LINK_GRAPH_PROCESSED_EVENTS_PURGE_INTERVAL_SECS` | `3600`, floored at `1` | Purge loop period |
| `LINK_GRAPH_RECONCILE_SECS` | `300`; must be `> 0` | Reconciliation pass period |
| `LINK_GRAPH_RECONCILE_URL_<ENTITY>` | unset ⇒ that entity is not reconciled | That entity's bulk `/links` pull URL. Only **person** and **case** have a live bulk endpoint to point this at today (`cross-service-linking.md` §11) |
| `LINK_GRAPH_RECONCILE_TOKEN` | unset ⇒ **any non-loopback `_RECONCILE_URL_*` is refused** (SEC-B7) | Bearer token sent on reconcile pulls — must be a real PASETO the target service's own guard accepts, not an arbitrary shared secret, once that service has `<P>_REQUIRE_AUTH` on |
| `LINK_GRAPH_PROBE_URL_<ENTITY>` | unset ⇒ that entity is not probed | By-id presence-probe URL template containing a literal `{id}` |
| `LINK_GRAPH_LAZY_VERIFY` | off | Verify-on-read for endpoints not yet covered by the durable bus |
| `LINK_GRAPH_SUGGEST_URL_PERSON` | unset ⇒ the suggestion job (T-31) does not start | Person's **collection base** URL (e.g. `http://host/api/persons`) — doubles as both the fetch source (`GET {url}?limit=&offset=`, person's database-backed list endpoint — corrected from an earlier `search?q=*` approach after a live check found the Tantivy index could drift from the database, see `spec/13-tasks.md` T-31) and the write target (`{url}/{id}/links`), since person is this job's sole write target |
| `LINK_GRAPH_SUGGEST_URL_WORKER` | unset ⇒ the suggestion job does not start (even with `_URL_PERSON` set) | Worker's collection base URL (e.g. `http://host/api/workers`), used **only** to fetch — never to write. Not part of OQ-9(c)'s literal pinned set; added because the job cannot produce a candidate without also reading worker's collection, named to match this section's own `_URL_<ENTITY>` convention |
| `LINK_GRAPH_SUGGEST_TOKEN` | unset ⇒ **any non-loopback `_SUGGEST_URL_*` is refused** (SEC-B7, same rule as `_RECONCILE_TOKEN`) | Bearer sent on every suggestion-job call (both fetches **and** the POST) — a **dedicated** token, not `LINK_GRAPH_RECONCILE_TOKEN`, since this job writes while reconcile only reads |
| `LINK_GRAPH_SUGGEST_SECS` | `3600`; must be `> 0` | Suggestion pass period — coarser than `_RECONCILE_SECS`'s 300s default because this job does real `O(pairs)` scoring work, not a cheap diff |
| `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` | `50`; unset/zero/unparseable falls back to the default | (T-33, OQ-9(d)) Same-block comparisons per person anchor within one pass — mirrors `BatchDeduplicationRequest::max_candidates`'s default and per-anchor `.take()` semantics exactly (`person-service-with-loco/src/models/review_queue.rs`) |
| `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` | `200`; unset/zero/unparseable falls back to the default | (T-33, OQ-9(d)) Caps how many suggestions one pass `POST`s; when the candidate count exceeds this, only the highest-confidence survivors are sent — the rest are simply found again next pass (the fetch is idempotent) |

[`cross-service-linking.md`](cross-service-linking.md) is the design doc.

## 8. authentication-service — the one crate with its own vocabulary

Unprefixed, because this crate issues tokens rather than verifying
siblings' — there is no `<P>_` pattern to fit into.

| Variable | Default | Effect |
|---|---|---|
| `TOKEN_PRIVATE_KEY_SEED` | falls through to `_FILE`, then a built-in dev seed | Inline base64url 32-byte Ed25519 signing seed |
| `TOKEN_PRIVATE_KEY_FILE` | — | File holding the same seed |
| `LOCO_ENV` / `RUST_ENV` | — | Either `== production` **refuses** the dev-seed fallback outright (SEC-A1 fail-closed) — a production deploy that forgets the real seed does not boot, rather than silently signing with a public value |
| `TOKEN_ADDITIONAL_PUBLIC_KEYS` | none | Comma-separated base64url verify-only public keys, for rotation overlap |
| `TOKEN_ISSUER` | `authentication-service` | Issued `iss` |
| `TOKEN_AUDIENCE` | `main-x-service` | Issued `aud` |
| `TOKEN_EXPIRATION` | `300` s | Issued-token lifetime |
| `AUTH_SESSION_IDLE_TTL_SECS` | `1800` (30 min); must be `> 0` | Sliding idle session TTL |
| `AUTH_SESSION_ABSOLUTE_TTL_SECS` | `43200` (12 h) | Hard session ceiling, never extended |
| `AUTH_ALLOWED_FRONTENDS` | empty ⇒ only `FRONTEND_URL` | Comma-separated exact-origin allow-list for a magic-link `return_url` (open-redirect guard) |
| `FRONTEND_URL` | `http://localhost:5173` | Default magic-link base URL |
| `AUTH_ALLOWED_ORIGINS` | unset ⇒ permissive in dev, **warns** (does not fail closed) in production | CSRF/origin backstop on `POST /token`; when set, a non-matching `Origin` is rejected outright (SEC-A10) |
| `AUTH_ATTRIBUTE_VOCABULARY` | unrestricted | Inline JSON allow-set of attribute keys→values for the operator attribute-assignment surfaces |
| `AUTH_ATTRIBUTE_VOCABULARY_FILE` | — | Path form; inline wins |

[`authentication-sessions.md`](authentication-sessions.md) is the
design doc for the token/session vars;
[`authorization-attributes.md`](authorization-attributes.md) §6 for
`AUTH_ATTRIBUTE_VOCABULARY*`.

## 9. Integrity MAC (all twelve crates)

Names are **constructed**, not literal — `integrity-mac`'s
`KeyConfig::{key_env,key_file_env,key_id_env,retired_keys_env}` format
`<PREFIX>_INTEGRITY_MAC_KEY[_FILE|_ID]` /
`<PREFIX>_INTEGRITY_MAC_KEYS_RETIRED` from each crate's own
`KeyConfig::new(..., "<PREFIX>")` call in its `src/compliance/mac.rs` —
invisible to a plain `env::var("...")` grep. `<PREFIX>` is the crate's
own §6 prefix with the trailing underscore stripped (portfolio is the
one exception: `PORTFOLIO`, not `PROJECT_PORTFOLIO_MANAGEMENT`).

| Variable | Default | Effect |
|---|---|---|
| `<PREFIX>_INTEGRITY_MAC_KEY` | unset ⇒ MAC disabled entirely | Inline hex root key |
| `<PREFIX>_INTEGRITY_MAC_KEY_FILE` | — | File holding the key; **takes precedence** over the inline var |
| `<PREFIX>_INTEGRITY_MAC_KEY_ID` | `k1` | Key id written into the `d1.<id>:` MAC prefix |
| `<PREFIX>_INTEGRITY_MAC_KEYS_RETIRED` | none | Retired keys as `id:hex,id:hex,…`, verify-only (rotation) |

See [`runbooks/integrity-activation.md`](runbooks/integrity-activation.md)
for the activation walkthrough and [`security.md`](security.md) for
why this is the one stored value an adversary holding just the
database cannot forge.

## 10. Per-crate additions beyond §2–§6

Not every crate has every capability (`overview.md`'s honest capability
matrix) — these vars only exist where the capability does.

**Tantivy search** (organization, care-pathway, case, portfolio):

| Variable | Default | Effect |
|---|---|---|
| `<P>_SEARCH_INDEX_PATH` | `data/search-index` | Index directory |
| `<P>_SEARCH_BOOT_REINDEX` | **on** (only `0`/`false`/`no`/`off` disables) | Rebuild the index at boot if it's missing/empty |

(person/worker/place/thing/event/course use the unprefixed
`SEARCH_INDEX_PATH` from §3 instead — a different mechanism, not a
missing prefix.)

**Bulk import/export** (person, care-pathway, organization, case — see
`overview.md`'s capability matrix for which):

| Variable | Default | Effect |
|---|---|---|
| `<P>_BULK_ARTIFACT_BACKEND` | `""` ⇒ `local`; unrecognised value ⇒ `local` + warn | `local` or `s3` (`s3` needs the crate's own `s3` Cargo feature) |
| `<P>_BULK_ARTIFACT_DIR` | `$TMPDIR/<entity>-bulk-artifacts` | Local artifact directory |
| `<P>_BULK_S3_BUCKET` | **required** when backend=`s3` (hard error otherwise) | S3 bucket |
| `<P>_BULK_S3_REGION` | `us-east-1` | S3 region |
| `<P>_BULK_S3_ENDPOINT` | unset ⇒ AWS default | Custom endpoint (MinIO/Ceph/R2) |
| `<P>_BULK_S3_FORCE_PATH_STYLE` | **on** when unset | Path-style addressing (self-hosted targets usually need this) |

Only person and care-pathway implement the S3 backend today;
organization and case have `_BULK_ARTIFACT_DIR` but no S3 vars at all
(local-filesystem-only, see `bulk-import-export.md` §11). See
[`bulk-import-export.md`](bulk-import-export.md).

**Read auditing** (person, worker, care-pathway, case):

| Variable | Default | Effect |
|---|---|---|
| `<P>_AUDIT_READS` | off | Record an audit row on read/disclosure, not just mutation |
| `<P>_AUDIT_FAIL_CLOSED` | off | Fail the read itself if its audit row can't be written |

**Portfolio-only** (no other crate has these — domain-specific to the
plan/task/goal/burndown surface):

| Variable | Default | Effect |
|---|---|---|
| `PROJECT_PORTFOLIO_MANAGEMENT_SNAPSHOT_HOURS` | `0` ⇒ off | Estate-snapshot capture period |
| `PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES` | unset/invalid ⇒ off + warn; must be ≥ the crate's own minimum period | Scheduled-action sweep period |
| `PROJECT_PORTFOLIO_MANAGEMENT_RISK_APPETITE` | unset ⇒ no declared appetite, no breach checks | Declared risk appetite |
| `PROJECT_PORTFOLIO_MANAGEMENT_SMART_SCORE_WEIGHTS` | built-in default weights; malformed ⇒ warn + defaults wholesale | Prioritisation scoring weights (basis-point map) |
| `PROJECT_PORTFOLIO_MANAGEMENT_STALL_DAYS` | `30`; must be `> 0` | Stall-detection threshold |
| `PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS` | unset ⇒ no caps | Per-column WIP limits on the plan board |

## 11. Defaults worth knowing are security-relevant

- `<P>_REQUIRE_AUTH`, `<P>_AUDIT_READS`, `<P>_AUDIT_FAIL_CLOSED` all
  default **off** — the shipped default is wide open (`security.md` §4).
- `<P>_SEARCH_BOOT_REINDEX` and `<P>_BULK_S3_FORCE_PATH_STYLE` both
  default **on** — the opposite direction, chosen for correctness over
  a quiet failure mode.
- `LINK_GRAPH_RECONCILE_TOKEN` unset makes any **non-loopback**
  `_RECONCILE_URL_*`/`_PROBE_URL_*` silently unusable rather than an
  unauthenticated pull (SEC-B7) — "configured but refused" is the
  correct reading of a warning log here, not a bug.
- `TOKEN_PRIVATE_KEY_SEED` unset is a **hard boot failure** in
  production (SEC-A1), the one variable in this whole reference where
  "unset" does not mean "falls back to a safe default."

## 12. Boot-fails-if-unset, in one place

- `DATABASE_URL` — every crate, production only (no Tera default).
- `JWT_SECRET` — every crate except course and link-graph, production
  only.
- `<P>_BULK_S3_BUCKET` — only when `<P>_BULK_ARTIFACT_BACKEND=s3`.
- `TOKEN_PRIVATE_KEY_SEED` (or `_FILE`) — authentication-service,
  production only (SEC-A1).
- Any person-family (§3) typed variable with a malformed value
  (`SERVER_PORT=not-a-number`, an out-of-range port, …) — these six
  crates raise a config error rather than silently using the default.
