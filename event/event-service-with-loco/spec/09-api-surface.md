## 9. API Surface

Complete endpoint reference: [`AGENTS/restful.md`](../AGENTS/restful.md).

| Tier | Surface |
|---|---|
| REST (Axum) | 15 endpoints under `/api/events/*` + `/api/audit/*` + `/api/health` |
| Auth (Axum) | `GET /api/whoami` — echo the verified PASETO bearer-token claims (`401` without a valid token) |
| FHIR R5 (Axum) | Live `Appointment` surface — `/fhir/Appointment{,/{id}}` (read/create/update/delete/search) + `GET /fhir/metadata` (`CapabilityStatement`); `application/fhir+json`, `OperationOutcome` errors, searchset Bundle (see §6.8) |
| gRPC (Tonic) | Stubbed |
| Web UI | None in this crate (backend-only loco service, no view tier). The operator UI is the sibling [`event-front-end-with-svelte`](../../event-front-end-with-svelte/spec/index.md). |
| Docs | Swagger UI at `/swagger-ui` |

Standard response envelope. `409` on duplicate-detected create; `422`
on validation failure.

Authentication is opt-in per handler: taking an `AuthUser` argument
requires a valid `Authorization: Bearer <paseto>` token, verified
offline (PASETO `v4.public`, Ed25519) against the auth-service
published key set (see §13 T-8).

Key-set configuration (issuer/audience from `EVENT_TOKEN_ISSUER` /
`EVENT_TOKEN_AUDIENCE`, defaults `authentication-service` /
`main-x-service`):

- `EVENT_PASETO_KEYS_URL` set (non-blank) — the key-set JSON is
  fetched over HTTP **once at boot** (async, in `after_routes`, before
  the routers/middleware capture the verifier) from the auth service
  (normally `/.well-known/paseto-keys`). A successful fetch **wins**
  over `EVENT_PASETO_KEYS`; a failed fetch warn-logs and falls back to
  the env path. No refresh loop (rotation re-fetch is roadmap, §15).
- Unset/blank — the key set comes from the `EVENT_PASETO_KEYS` env
  var; absent/unparseable ⇒ an empty reject-all key set.

Either way the service **always boots**.

Blanket enforcement is implemented **default-off**: when
`EVENT_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
case-insensitive), every `/api/*` **and** `/fhir/*` route requires a
valid bearer token except the public `/api/health` and
`GET /fhir/metadata`. Root-level `/_health`, `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` sit
outside the enforced scope and stay public. The flag is read once at
construction — restart to change. Family contract:
[jwt-enforcement](../../../agents/share/jwt-enforcement.md).

Authorization (ABAC, inside the same guard — so only when
`EVENT_REQUIRE_AUTH` is on): the request's action is derived from the
HTTP method plus this crate's destructive named POSTs
(`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the configured policy (`EVENT_ABAC_POLICY` inline JSON /
`EVENT_ABAC_POLICY_FILE`; unset/unparsable ⇒ built-in default: any
authenticated subject reads, `access=write` writes, `access=admin`
adds delete/merge/deduplicate, `svc=true` does everything) over the
token's `attrs` claim. A valid token the policy denies gets `403`
with the deciding rule; see
[authorization-attributes](../../../agents/share/authorization-attributes.md).

### 9.1 Bulk import / export

The async, job-based bulk contract is fixed family-wide in
[bulk import/export](../../../agents/share/bulk-import-export.md) (execution
model on `bg_pg`, the five endpoints, JSONL/CSV/Parquet codecs,
upsert-by-stable-key + dedupe-to-review, the per-row error report, and
export masking + audit). This section declares only the **event-specific**
bits; the shared doc is the source of truth for everything else.

The five endpoints (shared doc §4) mount under the event resource, under
the same `/api/events/*` prefix as the CRUD surface:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/events/import` | `202 {job_id}` — body: `format`, `dedupe_mode`, `dry_run`; file upload |
| `GET` | `/api/events/import/{id}` | Job status + counts + `errors_url` + `review_url` |
| `POST` | `/api/events/export` | `202 {job_id}` — body: `format`, `filter`, `fields`, `include_soft_deleted`, `masking_profile` |
| `GET` | `/api/events/export/{id}` | Job status + `download_url` |
| `GET` | `/api/events/bulk-jobs` | List (filter by `kind`/`status`); `GET .../{id}` for one |

This is distinct from the per-record iCalendar import/export of §13 T-7
(`POST /api/events/import.ics`, `GET /api/events/{id}.ics`): T-7 is a
single-record format converter, this is the bulk multi-row job machinery.

**Stable key(s) for upsert** (shared doc §6, §10). A row upserts in place
when it carries either:

- a **scheme-scoped event identifier** — the same `(scheme, value)` pair the
  matcher short-circuits on (event-matcher §5.1, §6.7: sharing any
  `(scheme, value)` pair is a deterministic match), drawn from the
  `EventIdScheme` set: `Wikidata`, `Eventbrite`, `Meetup`, `Ticketmaster`,
  `Songkick`, `Bandsintown`, `Facebook`, `Luma`, `GoogleCalendar`,
  `ICalendarUid` (RFC 5545 `UID`), or `Other(scheme)`. Cross-scheme
  identifiers never match — `(Eventbrite, "abc")` and `(Meetup, "abc")`
  denote different events; or
- the record **`pid`** (the event UUID `id`) when present in the row.

A row with neither runs the normal duplicate detection (§6 review queue),
routing likely duplicates to the review queue with `provenance = import`.

**CSV column set + flattening** (shared doc §5). CSV is the operator /
spreadsheet format and is lossy for deep nesting — steer fidelity-sensitive
loads to **JSONL** (the lossless reference). Flat columns:

- **scalar** (one column each): `id`, `name`, `description`,
  `disambiguating_description`, `url`, `start_date`, `end_date`,
  `door_time`, `duration`, `previous_start_date`, `time_zone`, `all_day`,
  `event_status`, `event_attendance_mode`, `event_type`,
  `typical_age_range`, `is_accessible_for_free`, `maximum_attendee_capacity`,
  `maximum_physical_attendee_capacity`, `maximum_virtual_attendee_capacity`,
  `remaining_attendee_capacity`, `super_event`, `active`, `created_at`,
  `updated_at`;
- **single nested object** → dotted columns: the primary `Location` when it
  is a `Place` (`location.name`, `location.latitude`, `location.longitude`,
  `location.url`, and the nested address `location.address.line1`,
  `location.address.line2`, `location.address.city`,
  `location.address.state`, `location.address.postal_code`,
  `location.address.country`);
- **arrays / arrays-of-objects** → a single **JSON-encoded cell** each:
  `alternate_names`, `image`, `same_as`, `keywords`, `in_language`,
  `identifiers` (the typed `Vec<Identifier>` / scheme-scoped `event_ids`),
  `location` (the full `Vec<Location>` union beyond the primary),
  `organizers`, `performers`, `attendees`, `sponsors`, `funders`,
  `contributors`, `about`, `works`, `sub_events`, `offers`, and `links`
  (the `EventLink` relationships).

**Export sensitivity** (shared doc §8). An event is generally
low-sensitivity reference data, so the default `masking_profile` is light
and full export needs no elevated authorisation in the common case. The
exception is party contact data (`organizers` / `attendees` / … carry
`email`) and clinical events (`EventType::Encounter`, `EncounterId`
identifiers), which are personal data and follow the standard masked
default with elevated-authorisation-for-full. `include_soft_deleted`
defaults `false` and is gated. **Every export is audited** (actor, filter,
format, row count, masking profile, timestamp — written even for a zero-row
export), per the shared contract.

