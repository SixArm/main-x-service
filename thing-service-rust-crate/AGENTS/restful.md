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

## RESTful API endpoints

| Method | Path                      | Description           |
|--------|---------------------------|-----------------------|
| GET    | `/api/health`             | Health check          |
| POST   | `/api/things`             | Create thing          |
| GET    | `/api/things/{id}`        | Get thing             |
| PUT    | `/api/things/{id}`        | Update thing          |
| DELETE | `/api/things/{id}`        | Soft delete thing     |
| GET    | `/api/things/search`      | Search things         |
| POST   | `/api/things/match`       | Match things          |
| POST   | `/api/things/duplicates`  | Check for duplicates  |
| POST   | `/api/things/merge`       | Merge things          |
| POST   | `/api/things/deduplicate` | Batch deduplication   |
| GET    | `/api/things/{id}/export` | GDPR data export      |
| GET    | `/api/things/{id}/masked` | Masked thing view     |
| GET    | `/api/things/{id}/audit`  | Audit logs            |
| GET    | `/api/audit/recent`       | Recent audit activity |
| GET    | `/api/audit/user`         | User audit logs       |

## Web UI endpoints

| Method | Path                                  | Description                                                    |
|--------|---------------------------------------|----------------------------------------------------------------|
| GET    | `/`                                   | Home page                                                      |
| GET    | `/things`                             | Thing index                                                    |
| GET    | `/things/search/partial?q=…`          | HTMX search fragment                                           |
| GET    | `/static/css/lily.css`                | NHS UK theme that styles Lily HTML Headless components         |
| GET    | `/static/js/htmx.min.js`              | HTMX 2.0.4                                                     |
| GET    | `/static/js/alpine.min.js`            | Alpine 3.14.8                                                  |
