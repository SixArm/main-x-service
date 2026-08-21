# FHIR R5 API — design

How the Main X Index family exposes its entity registries over **HL7 FHIR
R5**, so a FHIR-native client (EHR, care-coordination platform, analytics
pipeline) can read and write records using standard resources, bundles,
and search parameters instead of the bespoke REST DTOs. This is a design
document: it fixes the resource mapping, the endpoint surface, the wire
conventions (Bundle, `OperationOutcome`, `CapabilityStatement`,
search parameters), where FHIR sits in the loco-idiomatic service layout,
its auth/authorization posture, and the per-entity adoption, so each crate
adopts it without re-litigating. Only the per-entity **resource type** and
**field mapping** differ (§9).

It builds on the existing REST surface ([restful.md](restful.md)), reuses
the auth stack ([authentication-sessions.md](authentication-sessions.md),
[jwt-enforcement.md](jwt-enforcement.md)) and authorization
([authorization-attributes.md](authorization-attributes.md)), and honours
the same validation, masking ([privacy.md](privacy.md)), audit
([auditability.md](auditability.md)), and event
([event-bus.md](event-bus.md)) paths as the native API — FHIR is a second
**representation**, never a second write path that bypasses them.

## 1. Why change

Every service already speaks a bespoke snake_case JSON REST API. But the
entities this family registers — people, practitioners, places,
organizations, care pathways, devices — are exactly the things healthcare
and government systems exchange as **FHIR resources**. A FHIR-native
caller should not have to learn our DTO shapes, our identifier
conventions, or our search syntax; it should `GET /fhir/Patient/{id}` and
receive a conformant resource. The architecture doc
([architecture.md](architecture.md)) has always listed a "FHIR R5 API
(Axum)" tier next to REST; two services (person, worker) carry a partial,
**unmounted** prototype under `src/api/fhir/` with a non-standard
`resourceType`. This doc turns that aspiration into one uniform, mounted,
standards-faithful contract.

## 2. Goals & non-goals

**Goals**

- A **standards-faithful FHIR R5 representation** of each in-scope entity,
  at a stable `/fhir/<ResourceType>` surface, alongside (not replacing)
  the native REST API.
- **Bidirectional**: read (`GET`/search → resource/Bundle) and write
  (`POST`/`PUT` from a resource), the write path reusing the native
  validation, duplicate detection, event, and audit machinery.
- **Uniform** across services — one shape for interactions, Bundles,
  `OperationOutcome` errors, search-parameter parsing, and the
  `CapabilityStatement` — so a client learns it once.
- **Lossless where the model allows, explicit where it does not** — every
  drop of fidelity is a documented, `TODO`-marked gap, never silent.
- Reuses the family's **auth + ABAC** guard and **masking** rules: a FHIR
  read reveals no more than the equivalent REST read.

**Non-goals**

- **Full FHIR conformance / certification.** v1 is the core interactions
  and a handful of search parameters per resource, not the complete
  resource definition, every search modifier, `_include`/`_revinclude`,
  chained search, `_history`, conditional update, or `PATCH`.
- **FHIR as the system of record.** Postgres + the matcher DTO stay
  canonical; FHIR is a boundary representation, converted at the edge.
- **A FHIR resource for entities that have none.** Portfolio and
  authentication are out of scope (§9); course is a best-effort
  non-standard shape, clearly labelled.
- **FHIR-native matching/merge/search internals.** Duplicate detection and
  merge remain native operations; FHIR search maps onto the existing
  search/list path.
- **A shared FHIR runtime crate.** Only this contract is shared; the
  resource structs + conversions are **copied per project** (drift-accepted,
  same posture as the front-ends and `mxi-events`). See §12.

## 3. Resource mapping (the one shared decision)

Each in-scope entity maps to exactly one primary FHIR R5 resource type.
The mapping is fixed here; adding a resource later is a new row plus a
per-service conversion.

| Service | Entity | FHIR R5 resource | Fidelity | Notes |
|---|---|---|---|---|
| person | Person | **Patient** (primary) + **Person** (alias endpoint) | high | `Patient` is the clinical core; `Person` kept for the demographic/non-clinical view. Existing prototype used `Person` — reconciled to standard. |
| worker | Worker | **Practitioner** | high | replaces the prototype's non-standard `resourceType: "Worker"`. |
| place | Place | **Location** | high | address → `Location.address`, geo → `Location.position`. |
| organization | Organization | **Organization** | high | reference implementation (§10). |
| care-pathway | Care pathway | **PlanDefinition** | medium | clinical pathway template ⇒ `PlanDefinition`; `CarePlan` (an instantiated pathway) is a roadmap add. |
| thing | Thing | **Device** | medium | generic asset ⇒ `Device`; `Substance`/`Medication` are out of v1 scope. |
| event | Event | **Appointment** (default) + **Encounter** (roadmap) | low / best-effort | schema.org/Event ↔ FHIR is a loose fit; time window → `Appointment.start`/`.end`, participants → `Appointment.participant`. Documented as best-effort. |
| case | Case | **Task** (default) + **CarePlan** (roadmap) | low / best-effort | governmental case ⇒ `Task` (a tracked unit of work with status/priority/subject). **`subject_of` sensitivity applies** (§8, cross-service-linking §10). |
| course | Course | **Basic** (`code` = `course`) — non-standard | best-effort | no FHIR R5 resource models an educational course; wrapped as a `Basic` resource with a documented profile. Clearly labelled non-standard. |
| portfolio | Portfolio/Project/Product/Program | — | — | **out of scope**: no meaningful FHIR resource. |
| authentication | User (SSO) | — | — | **out of scope**: an auth provider, not a clinical/registry entity. |

- **`Patient` vs `Person`** for the person service: `Patient` is the
  primary, clinically-expected resource; the `/fhir/Person` endpoint is
  retained as a thin alias for the non-clinical demographic view. Both
  convert from the same domain `Person`.
- **Fidelity column** sets caller expectations up front: `high` = a
  natural, near-complete mapping; `medium` = the core fields map but the
  resource has clinical structure we don't populate; `low`/`best-effort` =
  a deliberate, documented approximation.

## 4. Endpoint surface (uniform per resource)

Each in-scope service mounts its resource at `/fhir/<ResourceType>` with
the FHIR **RESTful interactions** below. `<ResourceType>` is the §3 type
(e.g. `/fhir/Patient`, `/fhir/Organization`).

| Interaction | Method + path | Success | Body |
|---|---|---|---|
| read | `GET /fhir/<Type>/{id}` | 200 | the resource |
| create | `POST /fhir/<Type>` | 201 (+`Location`) | the created resource |
| update | `PUT /fhir/<Type>/{id}` | 200 | the updated resource |
| delete | `DELETE /fhir/<Type>/{id}` | 204 | — (soft-delete, as native) |
| search | `GET /fhir/<Type>?<params>` | 200 | a **searchset `Bundle`** |
| capabilities | `GET /fhir/metadata` | 200 | the `CapabilityStatement` (§7) |

- **`{id}`** is the record's public UUID (`pid`) — the same id the native
  API and the `EntityRef` ([cross-service-linking.md](cross-service-linking.md))
  use, so a FHIR `id` and a `person:<uuid>` ref are trivially
  inter-convertible.
- **Content type** `application/fhir+json` on responses (accept
  `application/fhir+json` and `application/json` on requests). `_format`
  is honoured for JSON only; XML is out of scope.
- **`_id` correspondence.** `POST` with no `id` mints a `pid`; `PUT`
  targets an existing `pid` (upsert-on-`PUT` — conditional create — is
  roadmap).
- The FHIR routes are **additive**: the native `/api/<plural>` surface is
  unchanged and remains the richer API (match, merge, deduplicate, audit,
  privacy export). FHIR covers CRUD + search only.

## 5. Errors — `OperationOutcome`

Every non-2xx FHIR response body is a FHIR **`OperationOutcome`** resource
(never the native `{success,data,error}` envelope), so a FHIR client can
parse failures uniformly:

```jsonc
{ "resourceType": "OperationOutcome",
  "issue": [ { "severity": "error", "code": "invalid",
               "diagnostics": "Patient.name: at least one name is required" } ] }
```

Status → `issue.code` mapping (fixed family-wide):

| HTTP | `issue.code` | When |
|---|---|---|
| 400 | `invalid` / `structure` | malformed FHIR / unparseable body |
| 401 | `login` | missing/invalid credential (guard, §8) |
| 403 | `forbidden` | valid credential, policy denied (§8) |
| 404 | `not-found` | unknown `{id}` (or soft-deleted) |
| 409 | `conflict` / `duplicate` | duplicate detected on create (native 409) |
| 422 | `processing` / `business-rule` | validation failure (native 422 reasons) |
| 500 | `exception` | database / search / internal |

Native validation errors (the `422` reasons the single-create validators
already produce) are surfaced one-per-`issue`, so the FHIR error carries
the same detail as the REST error.

## 6. Search & Bundles

- **`GET /fhir/<Type>?<params>` returns a `searchset` `Bundle`**: `type:
  "searchset"`, `total`, and `entry[]` each wrapping a resource with its
  `fullUrl`. Empty result ⇒ an empty-`entry` Bundle, not a 404.
- **Search parameters** map onto the entity's existing search/list path.
  A small, per-resource **supported set** is declared in the
  `CapabilityStatement` (§7); common ones:
  - **Common**: `_id`, `_lastUpdated`, `_count` (page size, capped like
    the native `limit`), `identifier` (token, `system|value`).
  - **Patient/Person/Practitioner**: `name`, `family`, `given`,
    `birthdate`, `gender`.
  - **Location/Organization**: `name`, `address`, `address-city`,
    `address-postalcode`.
  - **Device**: `identifier`, `type`, `manufacturer`.
  - **Task/Appointment**: `status`, `subject`/`patient` (token),
    `date`.
- **Unsupported parameters** are **ignored** (v1), not an error — but every
  ignored parameter that would narrow results is a silent-widening risk, so
  the `CapabilityStatement` is the source of truth for what actually
  filters. (Rejecting unknown params via `Prefer: handling=strict` is a
  roadmap option.)
- **Paging** uses `_count` + an opaque `offset`-backed `next` link in the
  Bundle, mirroring the native offset/limit pager. Cursor paging is
  roadmap.

## 7. `CapabilityStatement`

`GET /fhir/metadata` returns a minimal but honest **`CapabilityStatement`**
declaring, for this service: `fhirVersion: "5.0.0"`, `format:
["application/fhir+json"]`, the one `rest.resource` it serves, its
supported `interaction`s (§4), and its supported `searchParam`s (§6). This
is the machine-readable statement of exactly what the service implements —
so a client discovers the (deliberately partial) surface rather than
guessing. It must stay in sync with the mounted routes (a test pins this).

## 8. Auth, authorization & privacy

FHIR endpoints are **not** a backdoor around the security posture:

- **Blanket guard.** `/fhir/*` sits behind the same
  `<ENTITY>_REQUIRE_AUTH` blanket middleware as `/api/*`
  ([jwt-enforcement.md](jwt-enforcement.md)); it is **not** on the public
  allow-list (which stays `/api/health`, `/_health`, `/_ping`, the docs,
  and `/metrics.prom`). `GET /fhir/metadata` is the one FHIR path that MAY
  be public (capability discovery), at the service's discretion.
- **ABAC.** The guard derives the action from the HTTP method exactly as
  for REST ([authorization-attributes.md](authorization-attributes.md)):
  FHIR `GET`/search ⇒ `read`, `POST`/`PUT` ⇒ `write`, `DELETE` ⇒
  `delete`/`destructive`. No new action vocabulary. Record-level checks
  (`evaluate_with_resource`) and **masking obligations** apply on FHIR
  reads just as on REST reads — a policy that grants a *masked* read
  returns a redacted resource.
- **Masking.** The FHIR representation honours the same field-masking as
  the native masked view: a caller entitled only to masked data gets a
  resource with the sensitive elements redacted, never the full resource.
- **`case ↔ person` sensitivity.** The case service's `Task` resource (and
  any `subject`/`about` reference it carries) inherits the elevated
  governance of the `subject_of` edge (cross-service-linking §10): access
  control + audit on both read and write, and masking of the subject
  reference for unauthorised callers.
- **Audit.** Every FHIR create/update/delete writes the same audit record
  and emits the same `created`/`updated`/`deleted` event as its native
  counterpart ([auditability.md](auditability.md)) — the representation
  differs, the side effects do not.

## 9. Per-entity adoption (what each service declares)

The contract above is identical for every in-scope service. Each service
spec adds one FHIR section + a §13 task declaring only what differs:

1. **Resource type** — the §3 mapping (e.g. `Patient`, `Location`,
   `Device`), and any alias endpoint (person's `/fhir/Person`).
2. **Field mapping table** — domain field ↔ FHIR element, listing the
   populated elements and the **explicit fidelity gaps** (fields the
   resource defines that the domain has no source for, and domain fields
   the resource can't carry). Every gap is `TODO`-marked in code.
3. **Supported search parameters** — the per-resource subset (§6) that
   actually filters, reflected in the `CapabilityStatement`.
4. **Sensitivity** — any entity-specific masking/authorisation beyond the
   default (person, worker, case, and the `case↔person` reference).
5. **§13 task** — the code follow-up: the `fhir` module (resource structs,
   `to_fhir_*`/`from_fhir_*` conversions, `OperationOutcome`, Bundle,
   search-param parsing), the mounted controller (§10), the
   `CapabilityStatement`, OpenAPI/docs exposure, and tests (round-trip
   convert, each interaction, search→Bundle, `OperationOutcome` on error,
   `CapabilityStatement` matches routes, masked-read).

## 10. Where FHIR lives in the service layout

Two structural shapes exist in the family; the contract is identical, the
mounting differs:

**Loco-idiomatic services** (organization, care-pathway, case, place,
thing, event, course — `src/controllers/`): FHIR is a new controller
module.

```
src/
├── controllers/fhir.rs        loco `routes()` → /fhir/<Type>{,/{id}} + /fhir/metadata
├── fhir/
│   ├── mod.rs                 to_fhir_* / from_fhir_* over the stored DTO
│   ├── resources.rs           Fhir<Resource>, FhirOperationOutcome, FhirBundle, …
│   └── search.rs              search-param struct + mapping to the list/search path
```

Wire it in `app.rs::routes()` alongside the others:

```rust
AppRoutes::with_default_routes()
    .add_route(controllers::organizations::routes())
    .add_route(controllers::fhir::routes())   // <-- new
    .add_route(controllers::docs::routes())
    .add_route(controllers::metrics::routes())
```

The controller reads the same `AppContext`/model helpers the native
controller uses (the matcher-DTO-as-JSONB store), converting at the edge —
no separate persistence.

**Prototype-layout services** (person, worker — `src/api/`): the existing
`src/api/fhir/` module is **reconciled** to this contract (standard
`resourceType` per §3, `OperationOutcome` errors, `/fhir/metadata`) and —
critically — **actually mounted** (the current prototype defines handlers
but wires no routes). Their FHIR routes join their existing Axum router
and inherit the same blanket guard.

**Organization is the reference implementation** (§3 `high` fidelity, the
newer layout the other seven loco services share); it is built first and
copied.

## 11. Rollout

1. **Contract.** This doc; index wiring; per-service §13 FHIR tasks.
2. **Reference.** `organization` → `Organization`, full: the `fhir` module,
   mounted controller, `OperationOutcome`, searchset Bundle,
   `CapabilityStatement`, OpenAPI, tests. The copy source for the rest.
3. **High-fidelity loco services.** `place` → `Location`, `thing` →
   `Device`.
4. **Reconcile the prototypes.** `person` → `Patient` (+ `Person` alias),
   `worker` → `Practitioner`: standard `resourceType`, mount the routes,
   `OperationOutcome`, `/fhir/metadata`.
5. **Medium / best-effort.** `care-pathway` → `PlanDefinition`; then the
   best-effort mappings `event` → `Appointment`, `case` → `Task` (with §8
   governance), `course` → `Basic`.
6. **Hardening.** Per-resource search-param breadth, `_include`, `_history`,
   conditional create/update, `Prefer: handling=strict` — as real client
   demand appears.

## 12. Open questions

- **Shared `mxi-fhir` crate vs copy-per-project** for the resource structs
  + `OperationOutcome`/Bundle helpers. (Lean: copy until a second consumer
  needs an identical resource — same call as `mxi-events`/`EntityRef`.
  The common scaffolding — `OperationOutcome`, Bundle, `CapabilityStatement`
  builders, search-param base — is the tempting shared core if drift bites.)
- **`Patient` vs `Person`** as person's primary — this doc picks `Patient`
  primary + `Person` alias; confirm before person is reconciled.
- **course → `Basic`** vs no FHIR endpoint at all. (Lean: `Basic` so
  "all reasonable services" is honoured, clearly labelled non-standard;
  drop it if the non-standard shape misleads more than it helps.)
- **Unknown search params** — ignore (v1) vs reject under `Prefer:
  handling=strict`. (Lean: ignore now; add strict handling with the search
  hardening step.)
- **`_history` / versioning** — the domain keeps `updated_at` but not a
  full version history; FHIR `vread`/`_history` is deferred until an audit
  replay need appears.
