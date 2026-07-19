# RESTful API reference — Thing Service

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

```rust
use thing_service::models::thing::Thing;
use thing_service::models::identifier::{ThingIdentifier, IdentifierType};
use thing_service::models::consent::{Consent, ConsentType, ConsentStatus};
```

### Matching

```rust
use thing_service::matching::scoring::{
    compute_match, MatchWeights, MatchResult, MatchConfidence,
};
use thing_service::matching::name::name_similarity;
use thing_service::matching::description::description_similarity;
use thing_service::matching::url::{url_similarity, url_list_similarity};
use thing_service::matching::identifier::{
    identifier_similarity, has_deterministic_match,
};
use thing_service::matching::phonetic::{soundex, soundex_match};
```


### Adapter to the canonical `thing-matcher` crate

The service embeds the sibling `thing-matcher` crate and re-exports it
from `src/matching/mod.rs` as `matcher_lib`. Pair it with
`adapter::to_matcher_thing` to score two service records through the
canonical algorithm:

```rust
use thing_service::matching::adapter::to_matcher_thing;
use thing_service::matching::matcher_lib::{MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_things(
    &to_matcher_thing(&a),
    &to_matcher_thing(&b),
);
// result.score: f64 in [0.0, 1.0]
// result.is_match: bool
// result.confidence: High | Medium | Low
// result.breakdown: per-field Option<f64>
```

Field-routing rules are documented inline in
[`src/matching/adapter.rs`](../src/matching/adapter.rs) and pinned by
[`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs).

### Validation

```rust
use thing_service::validation::{validate_thing, normalize_thing, ValidationError};
```

### Privacy

```rust
use thing_service::privacy::{mask_thing, gdpr_export};
```

## Usage examples

### Create and validate a thing

```rust
let mut thing = Thing::new("Pride and Prejudice");
thing.description = Some("A novel of manners by Jane Austen.".into());
thing.additional_type = Some("https://schema.org/Book".into());
thing.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
thing.same_as = vec![
    "https://www.wikidata.org/wiki/Q170583".into(),
    "https://openlibrary.org/works/OL1394865W".into(),
];
thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

let errors = validate_thing(&thing);
assert!(errors.is_empty());

normalize_thing(&mut thing);
```

### Match two things

```rust
let a = Thing::new("Pride and Prejudice");
let b = Thing::new("Prde and Prejudice"); // typo

let result = compute_match(&a, &b, &MatchWeights::default());
println!("Score: {:.2}", result.score);            // 0.95+
println!("Confidence: {:?}", result.confidence);
println!("Name score: {:.2}", result.breakdown.name_score);
```

### Privacy masking

```rust
let mut thing = Thing::new("Private Diary");
thing.owner = Some("Jane Doe".into());
thing.identifiers = vec![ThingIdentifier::serial_number("SN-1234567890")];

let masked = mask_thing(&thing);
// owner: "[owner withheld]"
// identifier value: "****7890"

let export = gdpr_export(&thing); // Full JSON for data portability
```


### Prometheus metrics

| Method | Path             | Description                                                                  |
| ------ | ---------------- | ---------------------------------------------------------------------------- |
| GET    | `/metrics.prom`  | Prometheus text-exposition format (`text/plain; version=0.0.4`) for scraping |

The canonical `/metrics` endpoint serves the HTML performance dashboard;
configure your scraper with `metrics_path: /metrics.prom`. The metric
inventory (entity-CRUD counters, HTTP request counter, latency
histograms) is in [`src/metrics.rs`](../src/metrics.rs).

## API versioning

API URLs are version-free; select the representation version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).

## RESTful API endpoints

| Method | Path                      | Description           |
|--------|---------------------------|-----------------------|
| GET    | `/api/health`             | Health check          |
| GET    | `/api/whoami`             | Echo the verified bearer-token claims (`401` without a valid token) |
| POST   | `/api/things`             | Create thing          |
| GET    | `/api/things/{id}`        | Get thing             |
| PUT    | `/api/things/{id}`        | Update thing          |
| DELETE | `/api/things/{id}`        | Soft delete thing     |
| GET    | `/api/things/search`      | Search things         |
| POST   | `/api/things/match`       | Match things          |
| POST   | `/api/things/check-duplicates` | Check for duplicates |
| POST   | `/api/things/merge`       | Merge things          |
| POST   | `/api/things/deduplicate` | Batch deduplication   |
| GET    | `/api/things/review-queue` | Stored review queue (filter `status`, `limit`) |
| POST   | `/api/things/review-queue/{id}/decision` | Decide a pending review item (`confirmed` / `rejected`) |
| GET    | `/api/things/{id}/export` | GDPR data export      |
| GET    | `/api/things/{id}/masked` | Masked thing view     |
| GET    | `/api/things/{id}/audit`  | Audit logs            |
| GET    | `/api/audit/recent`       | Recent audit activity |
| GET    | `/api/audit/user`         | User audit logs       |

### FHIR R5 endpoints

HL7 FHIR R5 surface mapping things to the FHIR `Device` resource,
mounted by [`src/controllers/fhir.rs`](../src/controllers/fhir.rs)
(`routes()`, prefix `/fhir`):

| Method | Path                | Description               |
|--------|---------------------|---------------------------|
| GET    | `/fhir/metadata`    | CapabilityStatement       |
| POST   | `/fhir/Device`      | Create FHIR Device        |
| GET    | `/fhir/Device`      | Search FHIR Devices       |
| GET    | `/fhir/Device/{id}` | Get FHIR Device           |
| PUT    | `/fhir/Device/{id}` | Update FHIR Device        |
| DELETE | `/fhir/Device/{id}` | Delete FHIR Device (soft) |

### Auth

Bearer tokens are PASETO `v4.public` (Ed25519) minted by the central
authentication-service and verified **offline** against its published
key set (`/.well-known/paseto-keys`) via the `authentication-verifier`
crate — no shared secret, no introspection call. Configure with
`THING_PASETO_KEYS_URL` (boot-time HTTP fetch of the key set) or
`THING_PASETO_KEYS` (key-set JSON via env), plus `THING_TOKEN_ISSUER`
and `THING_TOKEN_AUDIENCE`. Handlers opt in by taking an `AuthUser`
argument (`src/api/rest/auth.rs`).

Key-set precedence (`state::boot_verifier`, run in
`App::after_routes` before the shared-store insert and the middleware
capture the state): when `THING_PASETO_KEYS_URL` is set (non-blank)
the key set is fetched over HTTP **once at boot** and, on success,
**wins** over `THING_PASETO_KEYS`; a fetch failure logs a warning and
falls back to the env path. Unset/blank ⇒ `THING_PASETO_KEYS`, else an
empty reject-all key set. The service **always boots**; there is no
refresh loop (key-rotation re-fetch is a roadmap item, spec §15).

Blanket enforcement (default **off**): when `THING_REQUIRE_AUTH` is
truthy (`1`/`true`/`yes`/`on`, case-insensitive; anything else
including unset/blank ⇒ off), the `auth::require_auth_mw` middleware
requires a valid bearer token on **every** `/api/*` route except the
public `/api/health`. Root-level `/_health`, `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` sit
outside the `/api` scope and stay public. The flag is read once at
`AppState` construction — restart the service to change it. Wired on
both router surfaces (`create_router` and the loco router in
`App::after_routes`). Unauthenticated requests to any protected path
return `401`; a valid token the ABAC policy denies (below) returns
`403`. Family contract:
[`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md).

#### Authorization (ABAC)

Inside the same guard (so only when `THING_REQUIRE_AUTH` is on), a
verified token is authorized by **attribute-based access control**
per `agents/share/authorization-attributes.md`: the request's action
is derived from the HTTP method plus the crate's destructive named
POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the policy over the token's `attrs` claim. Configure with
`THING_ABAC_POLICY` (inline JSON) or `THING_ABAC_POLICY_FILE` (path);
unset or unparsable ⇒ warn-log + the built-in default policy (any
authenticated subject reads; `access=write` writes; `access=admin`
adds DELETE/merge/deduplicate; `svc=true` does everything). Read once
at `AppState` construction — restart to change. `401` = missing/bad
credential; `403` = valid credential, policy denied (the body names
the deciding rule).
