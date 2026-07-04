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
| GET    | `/api/things/{id}/export` | GDPR data export      |
| GET    | `/api/things/{id}/masked` | Masked thing view     |
| GET    | `/api/things/{id}/audit`  | Audit logs            |
| GET    | `/api/audit/recent`       | Recent audit activity |
| GET    | `/api/audit/user`         | User audit logs       |

### Auth

Bearer tokens are PASETO `v4.public` (Ed25519) minted by the central
authentication-service and verified **offline** against its published
key set (`/.well-known/paseto-keys`) via the `authentication-verifier`
crate — no shared secret, no introspection call. Configure with
`THING_PASETO_KEYS` (key-set JSON), `THING_TOKEN_ISSUER`, and
`THING_TOKEN_AUDIENCE`. Handlers opt in by taking an `AuthUser`
argument (`src/api/rest/auth.rs`).
