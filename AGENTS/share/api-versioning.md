# API versioning — design

How the Main X Index family versions its HTTP APIs: **the version is not in
the URL**. URLs are stable and version-free (`/api/persons`, not
`/api/v1/persons`); the API version is negotiated through a custom request
header. This is a design document: it fixes the header name, the value
format, the default, the negotiation/rejection rules, and the rollout, so
each crate adopts it without re-litigating. It applies to the native REST
surface; the FHIR surface ([fhir.md](fhir.md)) carries its own
`fhirVersion` in the `CapabilityStatement` and is unaffected.

## 1. Why change

Some services had baked `v1` into their paths (`/api/v1/...`); most never
did (`/api/...`). URL versioning clutters every path with versioning
information, couples clients to a path shape, and makes "the same
resource" live at two URLs across a version bump. The family standardises
on **header versioning**: the URL names the resource, a header names the
representation version. This declutters the URI and lets a version change
be a header change, not a re-rooted API.

```
curl -H "Accepts-version: 1.0" https://www.example.com/api/products
```

## 2. Goals & non-goals

**Goals**

- **No version segment in any API URL.** `/api/<plural>` and its
  sub-paths, family-wide. (`/api/v1/*` is removed.)
- **Version via a custom request header**, `Accepts-version`, value a
  dotted version like `1.0`.
- **Backward-lenient default**: a request with no `Accepts-version` header
  gets the current (latest) version — nothing breaks for existing callers.
- **Explicit and observable**: the resolved version is echoed in a
  response header, and an explicitly-unsupported requested version is a
  clean `406`.
- One uniform rule across the family; only the *set of supported versions*
  can differ later (today it is exactly `{1.0}`).

**Non-goals**

- **Multiple concurrent versions right now.** There is exactly one
  version, `1.0`. The mechanism exists so a future `2.0` is a header
  negotiation, not a URL fork.
- **Versioning the FHIR surface this way** — FHIR advertises `fhirVersion`
  (`5.0.0`) in its `CapabilityStatement`; `/fhir/*` ignores
  `Accepts-version`.
- **Media-type / `Accept: application/vnd.…` versioning** — the family
  picks the simpler custom-header form the example above shows.
- **Query-string or URL versioning** (rejected — the whole point).

## 3. The contract

### Header

- **Request**: `Accepts-version: <version>` (e.g. `Accepts-version: 1.0`).
  Optional. Case-insensitive header name (HTTP headers are
  case-insensitive); the value is a trimmed dotted string.
- **Response**: every API response echoes `Accepts-version: <resolved>`
  so a client can see which version served it.

### Supported versions & default

- **`SUPPORTED_API_VERSIONS`** — the closed set a service accepts. Today:
  `["1.0"]`.
- **`CURRENT_API_VERSION`** — the latest, used when the request omits the
  header. Today: `"1.0"`.
- A bare major (`1`) is accepted as an alias for that major's current
  minor (`1.0`) — lenient matching, so `Accepts-version: 1` works.

### Negotiation (per request)

```
no Accepts-version header            → serve CURRENT_API_VERSION (1.0)
Accepts-version present & supported  → serve that version
Accepts-version present & unsupported→ 406 Not Acceptable
```

- `406` body names the requested and supported versions (JSON on the
  native API; a FHIR `OperationOutcome` is **not** used here — this is the
  native surface).
- Safe and unsafe methods alike are negotiated; the header is orthogonal
  to auth (§5) and to the action/ABAC derivation.

## 4. Where it lives (implementation shape)

A small, pure helper + a thin edge:

- **Pure core** (unit-testable, no I/O): `resolve_version(header:
  Option<&str>) -> Result<&'static str /*resolved*/, /*unsupported*/>`
  over `SUPPORTED_API_VERSIONS` / `CURRENT_API_VERSION`. Trims, lowercases
  the compare, applies the bare-major alias.
- **Edge**: a middleware layered on the API router that runs
  `resolve_version` on `Accepts-version`, returns `406` on the error, and
  on success sets the `Accepts-version` response header. It is orthogonal
  to the auth guard and may be composed in the same layer stack.
- **Loco-idiomatic services** put the helper in `src/version.rs` and layer
  the middleware in `app.rs` (next to the auth layer). **api/rest-layout
  services** (event, worker, …) put it beside `src/api/rest/auth.rs` and
  layer it in `create_router` + `after_routes`, exactly like the auth
  middleware.
- The version middleware is **additive and near-noop**: with no header and
  one supported version, it only stamps the response header.

## 5. Relationship to auth & FHIR

- **Auth** ([jwt-enforcement.md](jwt-enforcement.md),
  [authorization-attributes.md](authorization-attributes.md)) is
  unchanged; the blanket guard's public allow-lists lose their `/v1`
  segment (`/api/v1/health` → `/api/health`) along with the routes. The
  version middleware does not authenticate; the auth middleware does not
  version.
- **FHIR** ([fhir.md](fhir.md)) is exempt (§2). `/fhir/*` paths were
  already version-free.

## 6. Rollout

1. **Contract.** This doc; index wiring.
2. **Reference.** `event` — drop `/api/v1` → `/api` (router nests, the auth
   `API_PREFIX`/public/destructive-suffix constants, handlers, state,
   tests) and add the `Accepts-version` helper + middleware. The copy
   source for the rest.
3. **De-version the other versioned services.** `worker`, `portfolio`
   (routes, auth, metrics/openapi/streaming path strings, tests); fix
   `person`'s stale `/api/v1` test URIs (its routes were already
   unversioned); the `case-folder` consumer app.
4. **Front-ends.** `event` + `portfolio` SvelteKit clients (base path,
   API client, proxy route, tests) drop `/api/v1`; the BFF/proxy sends
   `Accepts-version: 1.0`.
5. **Docs.** Sweep specs / AGENTS / READMEs that cite `/api/v1`.
6. **Later services adopt the header helper** when next touched — they
   already have version-free URLs, so this is additive.

## 7. Open questions

- **Enforce vs. advertise.** v1 rejects an explicitly-unsupported version
  with `406`; an alternative is to ignore the header entirely until a
  second version exists. (Lean: negotiate now so the contract is real and
  tested before it is needed.)
- **Deprecation signalling** — when `2.0` lands, add a `Deprecation` /
  `Sunset` response header for `1.0`? (Defer until a second version
  exists.)
- **Per-endpoint version skew** — a single family-wide version vs.
  per-resource versions. (Lean: one family-wide version; revisit only if a
  single resource needs to bump alone.)
