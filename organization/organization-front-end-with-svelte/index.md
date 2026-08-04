# organization-front-end-with-svelte — documentation index

Operator UI for organization CRUD + matching, consuming the
[Organization Service](../organization-service-with-loco).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Flow

```text
/         ──>  GET  /api/organizations                 list
/new      ──>  POST /api/organizations  {Organization} create -> /[pid]
/[pid]    ──>  GET  /api/organizations/{pid}           detail
              POST /api/organizations/check-duplicates  -> scored matches (self excluded)
              DELETE /api/organizations/{pid}            soft-delete
/[pid]/edit ─> PUT  /api/organizations/{pid}            edit
/review   ──>  GET  /api/organizations/review-queue     stored review queue
              POST /api/organizations/deduplicate       batch scan (button-only)
              POST /api/organizations/review-queue/{id}/decision  drag-to-decide
/merge    ──>  GET  /api/organizations/{pid}  x2          optional side-by-side preview
              POST /api/organizations/merge              fold duplicate into survivor
              GET  /api/organizations/merges/recent      merge history
```

## Session / authentication (BFF)

```text
signed in ──> central auth-service magic-link ──> server-side cookie session
browser ──(__Host-mxi_session cookie, httpOnly)──> own SvelteKit server (BFF)
BFF ──(session → short-lived PASETO v4.public)──> organization service (server-side)
mutating requests: CSRF-protected; browser holds NO token (no localStorage)
```

The browser holds no token; the BFF exchanges the cookie session for a
short-lived PASETO v4.public bearer and calls the service server-side.
Service-side enforcement (`ORGANIZATION_REQUIRE_AUTH`) is off by default.
Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the `#access_token` fragment handoff decommissioned).
The runtime is the BFF: `src/lib/server/` + the `/api/proxy` server route
hold the session and inject the PASETO server-side.

## Worked example — the Organization payload

The create/edit body **is** the `organization_matcher::Organization`
shape, serialized snake_case. Identifiers are `{scheme, value}`; bare
schemes are strings, the `Custom` variant is `{ "Custom": "label" }`
(read-only in this UI — the form's dropdown edits only unit-variant
schemes):

```json
{
  "name": "Acme Corporation",
  "legal_name": "Acme Corporation Ltd.",
  "alternate_names": ["ACME", "Acme Inc"],
  "identifiers": [
    { "scheme": "Lei", "value": "529900T8BM49AURSDO55" },
    { "scheme": "Duns", "value": "150483782" },
    { "scheme": { "Custom": "internal-id" }, "value": "ORG-42" }
  ],
  "url": "https://acme.example",
  "same_as": ["https://www.wikidata.org/wiki/Q1"],
  "address": {
    "street_address": "1 High St",
    "locality": "London",
    "region": "England",
    "postal_code": "SW1A 1AA",
    "country": "GB"
  },
  "jurisdiction": "GB",
  "founding_date": "1971",
  "telephone": "+1 555 0100",
  "email": "ops@acme.example",
  "keywords": ["manufacturing", "widgets"]
}
```

`OrganizationForm` assembles this via `src/lib/api/build.ts`: blank
scalars become `null` (explicit clear), comma-list inputs split into
arrays, the address is attached only if at least one part is filled, and
empty identifier rows are dropped.
