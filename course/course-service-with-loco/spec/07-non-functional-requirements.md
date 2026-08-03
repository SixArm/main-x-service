## 7. Non-Functional Requirements

- **Throughput:** ≥1000 req/s sustained on a single 4-core host for `GET /courses/{id}`.
- **Latency p95:** `GET /courses/{id}` ≤ 25 ms; `GET /courses/search` ≤ 100 ms; `POST /courses/match` ≤ 500 ms.
- **Bundle size (binary):** < 30 MB stripped.
- **Memory:** ≤ 256 MB resident for 1M courses + 5M instances indexed.
- **Search consistency:** a `POST /courses` MUST be observable via `GET /courses/search` immediately on subsequent requests (`SearchEngine::reload()` after every commit, matching person-service).

### 7.1 Authentication / authorization configuration

Read once at `AppState` construction (`App::after_routes`); restart to change. All optional — the service always boots and enforcement is **off by default**.

| Variable | Default | Notes |
|---|---|---|
| `COURSE_REQUIRE_AUTH` | off | Truthy (`1`/`true`/`yes`/`on`, case-insensitive) turns on the blanket `/api/*` + `/fhir/*` bearer-token guard. Anything else (incl. unset) ⇒ off. |
| `COURSE_PASETO_KEYS_URL` | — | URL of the auth service's `/.well-known/paseto-keys`; fetched once at boot. On success wins over `COURSE_PASETO_KEYS`; fetch failure logs a warning and falls back to the env path. |
| `COURSE_PASETO_KEYS` | empty (reject-all) | Ed25519 key-set JSON (OKP/Ed25519 form) via env. Used when the URL is unset/blank or its fetch fails. |
| `COURSE_TOKEN_ISSUER` | `authentication-service` | Expected `iss` claim. |
| `COURSE_TOKEN_AUDIENCE` | `main-x-service` | Expected `aud` claim. |
| `COURSE_ABAC_POLICY` | built-in default | Inline ABAC policy JSON. Unparsable ⇒ warn-log + built-in default policy. |
| `COURSE_ABAC_POLICY_FILE` | built-in default | Path to an ABAC policy JSON file (used when `COURSE_ABAC_POLICY` is unset). Unreadable/unparsable ⇒ warn-log + built-in default policy. |

### 7.2 Durable event bus configuration (T-21)

Transactional-outbox transport for the durable event bus ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md)). Read once at `AppState` construction; restart to change. **Default `memory`** ⇒ behaviour-neutral (the in-memory `CourseEvent` publish is unchanged; the outbox is additive).

| Variable | Default | Notes |
|---|---|---|
| `COURSE_EVENT_TRANSPORT` | `memory` | `outbox` ⇒ write one `course_outbox` row **inside** each Course write's transaction (create/update/soft-delete/merge), so a committed change always has its event. `memory` and any unrecognised value ⇒ in-memory publish only, no outbox writes. |
| `COURSE_EVENT_RELAY` | `false` | Phase-3 relay worker toggle. Truthy (`1`/`true`/`yes`/`on`) spawns the background loop that drains unpublished `course_outbox` rows to the sink, stamps `published_at`, and purges old published rows. Only runs when the transport is `outbox`; off by default. |
| `COURSE_EVENT_RELAY_INTERVAL_SECS` | `5` | Relay poll interval in seconds (floored at 1). |
| `COURSE_EVENT_RETENTION_DAYS` | `7` | Outbox row TTL, **enforced** by the Phase-3 relay's retention sweep (published rows older than this are purged). |
| `COURSE_FLUVIO_ENDPOINT` | — (unset ⇒ `LoggingSink`) | The real broker's SC address (e.g. `127.0.0.1:9103`). Set ⇒ the relay selects `FluvioSink` (behind this crate's `fluvio` cargo feature) instead of `LoggingSink`. Set **without** the feature compiled in ⇒ the relay refuses to start at all (logged at `error`), never a silent `LoggingSink` fallback. |
| `COURSE_EVENT_TOPIC` | `mxi.course.events` | The Fluvio topic `FluvioSink` publishes to, per `agents/share/event-bus.md` §7's `mxi.<entity>.events` convention. |

Merge emits a `merged` outbox row for the survivor (carrying the duplicate's pid in `merged_from`) plus a `deleted` row for the duplicate, atomically in one transaction. The Phase-3 relay worker (`src/relay.rs`) drains `course_outbox` (`Model::unpublished` / `mark_published`) to an `EventSink` — the default `LoggingSink` is the no-broker dev/CI sink — stamps `published_at`, and purges published rows past the retention window; it is spawned in `App::after_routes` and is a no-op unless `COURSE_EVENT_TRANSPORT=outbox` and `COURSE_EVENT_RELAY` are set. **The real-broker `FluvioSink` (BUS-3) has landed**, behind this crate's own `fluvio` cargo feature (off by default — a default build's dependency tree is unchanged); see §13 T-23. The only remaining Phase-3 follow-up is wiring the `CourseInstance` sub-resource onto the outbox.

