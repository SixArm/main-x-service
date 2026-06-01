# Thing Service — Index

Generic registry for arbitrary discrete objects — books, papers,
software, digital assets, devices, products. Domain model aligned
with [schema.org/Thing](https://schema.org/Thing); identifiers via
the schema.org [`PropertyValue`](https://schema.org/PropertyValue)
shape (DOI, ISBN, ISSN, GTIN, SKU, MPN, SerialNumber, URI, UUID,
Custom). Probabilistic + deterministic matching, real-time and batch
deduplication, GDPR Article 15 export, audit trail.

This page is a **navigation aid with worked examples**. For canonical
behaviour, read [`spec.md`](spec.md).

## Documentation map

| File | Role |
|------|------|
| [`spec.md`](spec.md) | **Single source of truth.** What the system does, how it is built, NFRs, tasks (§13), open questions (§16). |
| [`README.md`](README.md) / [`CLAUDE.md`](CLAUDE.md) | User-facing intro — must stay consistent with the spec. |
| [`AGENTS.md`](AGENTS.md) | Agent-facing entry point — `AGENTS/*` directory + shared docs. |
| [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md) | The SDD discipline this crate practises. |
| [`AGENTS/models.md`](AGENTS/models.md) | Field-by-field domain model reference. |
| [`AGENTS/matching.md`](AGENTS/matching.md) | Match weights, components, deterministic rules, Soundex. |
| [`AGENTS/restful.md`](AGENTS/restful.md) | Endpoint catalogue + library API. |
| [`AGENTS/testing.md`](AGENTS/testing.md) | Unit / integration / benchmark layout. |
| [`agents/share/*`](../agents/share/) | Project-wide cross-crate references. |

## Quick start

```bash
# REST API
cargo run --release

# Web UI
cargo run --bin web                    # → http://0.0.0.0:5150
PORT=5180 cargo run --bin web

# Tests
cargo test --lib                       # unit (~100)
cargo test --tests                     # integration_*
cargo bench                            # Criterion
```

## URL surface (REST)

| Method | Path | Notes |
|---|---|---|
| GET | `/api/health` | Liveness |
| POST | `/api/things` | Create — `409` on detected duplicate |
| GET | `/api/things/{id}` | Read |
| PUT | `/api/things/{id}` | Update |
| DELETE | `/api/things/{id}` | Soft delete |
| GET | `/api/things/search` | Full-text + fuzzy |
| POST | `/api/things/match` | Score against candidates |
| POST | `/api/things/duplicates` | Real-time dup check |
| POST | `/api/things/merge` | Merge survivor + duplicate |
| POST | `/api/things/deduplicate` | Batch dedup scan |
| GET | `/api/things/{id}/masked` | Privacy view |
| GET | `/api/things/{id}/export` | GDPR Art. 15 export |
| GET | `/api/things/{id}/audit` | Per-record audit |
| GET | `/api/audit/recent` | System-wide recent audit |
| GET | `/api/audit/user` | Per-user audit |

This crate does **not** expose a FHIR R5 surface. See
[`spec.md §9`](spec.md#9-api-surface).

## Worked examples

### Create a thing (book)

```bash
curl -X POST http://localhost:8080/api/things \
  -H 'content-type: application/json' \
  -d '{
    "name": "Pride and Prejudice",
    "alternate_names": ["First Impressions"],
    "description": "A novel of manners by Jane Austen.",
    "additional_type": "https://schema.org/Book",
    "url": "https://en.wikipedia.org/wiki/Pride_and_Prejudice",
    "images": ["https://example.com/cover.jpg"],
    "main_entity_of_page": "https://en.wikipedia.org/wiki/Pride_and_Prejudice",
    "owner": "Penguin Random House",
    "same_as": [
      "https://www.wikidata.org/wiki/Q170583",
      "https://openlibrary.org/works/OL1394865W"
    ],
    "identifiers": [
      { "property_id": "Isbn", "value": "9780141439518" }
    ]
  }'
```

If an existing Thing matches deterministically (same ISBN) or
heuristically (similar name + URL), the response is `409 Conflict`
with the candidate matches and per-component breakdown.

### Create a thing (paper)

```bash
curl -X POST http://localhost:8080/api/things \
  -H 'content-type: application/json' \
  -d '{
    "name": "Attention Is All You Need",
    "additional_type": "https://schema.org/ScholarlyArticle",
    "identifiers": [
      { "property_id": "Doi", "value": "10.48550/arXiv.1706.03762" }
    ]
  }'
```

### Check for duplicates

```bash
curl -X POST http://localhost:8080/api/things/duplicates \
  -H 'content-type: application/json' \
  -d '{
    "name": "Pride and Prejudice",
    "identifiers": [
      { "property_id": "Isbn", "value": "9780141439518" }
    ]
  }'
```

### Search

```bash
curl "http://localhost:8080/api/things/search?\
q=Pride+and+Prejudice&limit=10&offset=0&fuzzy=true&mask_sensitive=true"
```

| Parameter | Meaning |
|---|---|
| `q` | Free-text against name / alternate_names / description / identifier values / URL / same_as |
| `limit` / `offset` | Pagination (limit ≤ 100) |
| `fuzzy` | Enable Tantivy fuzzy matching |
| `mask_sensitive` | Apply per-field masking to results |

### Match against existing records

```bash
curl -X POST http://localhost:8080/api/things/match \
  -H 'content-type: application/json' \
  -d '{
    "name": "Prde and Prejudice",
    "identifiers": [
      { "property_id": "Isbn", "value": "9780141439518" }
    ],
    "threshold": 0.7
  }'
```

A deterministic identifier match (same ISBN) short-circuits to
`score = 1.00, confidence = Certain` regardless of other components.

### Merge

```bash
curl -X POST http://localhost:8080/api/things/merge \
  -H 'content-type: application/json' \
  -d '{
    "main_thing_id": "11111111-1111-1111-1111-111111111111",
    "duplicate_thing_id": "22222222-2222-2222-2222-222222222222",
    "merge_reason": "Confirmed duplicate (same ISBN, different listings)"
  }'
```

### Batch deduplication

```bash
curl -X POST http://localhost:8080/api/things/deduplicate \
  -H 'content-type: application/json' \
  -d '{
    "threshold": 0.70,
    "auto_merge_threshold": 0.95,
    "max_candidates": 50
  }'
```

### GDPR Article 15 export

```bash
curl "http://localhost:8080/api/things/{id}/export"
```

### Masked view

```bash
curl "http://localhost:8080/api/things/{id}/masked"
```

Returns the Thing with `owner` → `"[owner withheld]"`, identifier
`value` → `"****<last 4 chars>"`, and per-identifier `url` cleared.
`property_id` is preserved.

## Library API examples

### Validate and normalise

```rust
use thing_service::models::thing::Thing;
use thing_service::models::identifier::ThingIdentifier;
use thing_service::validation::{validate_thing, normalize_thing};

let mut thing = Thing::new("Pride and Prejudice");
thing.description = Some("A novel of manners by Jane Austen.".into());
thing.additional_type = Some("https://schema.org/Book".into());
thing.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
thing.same_as = vec![
    "https://www.wikidata.org/wiki/Q170583".into(),
    "https://openlibrary.org/works/OL1394865W".into(),
];
thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

let errs = validate_thing(&thing);
assert!(errs.is_empty(), "validation failed: {errs:?}");

normalize_thing(&mut thing);    // URL schemes lowercased; lists deduped
```

### Match two things

```rust
use thing_service::matching::scoring::{compute_match, MatchWeights};

let a = Thing::new("Pride and Prejudice");
let b = {
    let mut t = Thing::new("Stolz und Vorurteil");      // translated title
    t.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    t
};
let a = {
    let mut t = a;
    t.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    t
};

let result = compute_match(&a, &b, &MatchWeights::default());
println!("score={:.2} confidence={:?}", result.score, result.confidence);
//                                          → 1.00, Certain
println!("deterministic={}", result.breakdown.deterministic_match);
//                                          → true
```

### Match via the canonical `thing-matcher` bridge

Use the sibling `thing-matcher` crate as the reference algorithm. The
service re-exports it as `matcher_lib`, and `adapter::to_matcher_thing`
projects the service's domain model into the matcher's input shape
(including national-identifier routing by FHIR `system` URI, address
field renaming, and identifier-scheme dispatch).

```rust,no_run
use thing_service::matching::adapter::to_matcher_thing;
use thing_service::matching::matcher_lib::{Confidence, MatchConfig, MatchingEngine};
use thing_service::models::*;

let mut a = Thing::new("Pride and Prejudice");
a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
let mut b = Thing::new("Stolz und Vorurteil"); // German translation
b.identifiers = vec![ThingIdentifier::isbn("9780141439518")]; // same ISBN

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));

assert!(result.is_match, "near-duplicate should classify as match");
assert_eq!(result.confidence, Confidence::High);
println!("score   = {:.3}", result.score);
println!("conf    = {:?}", result.confidence);
// `result.breakdown` carries per-field Option<f64> for an auditable trail.
```

End-to-end pinning lives in
[`tests/duplicate_detection.rs`](tests/duplicate_detection.rs); the
adapter source is [`src/matching/adapter.rs`](src/matching/adapter.rs).


### Privacy mask + GDPR export

```rust
use thing_service::privacy::{mask_thing, gdpr_export};

let mut thing = Thing::new("Private Diary");
thing.owner = Some("Jane Doe".into());
thing.identifiers = vec![ThingIdentifier::serial_number("SN-1234567890")];

let masked = mask_thing(&thing);
// owner: "[owner withheld]"
// identifier value: "****7890"

let export = gdpr_export(&thing);
```

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | _required_ |
| `DATABASE_MIN_CONNECTIONS` / `DATABASE_MAX_CONNECTIONS` | Pool sizes | `2` / `10` |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `PORT` | Web UI port | `5150` |
| `SEARCH_INDEX_PATH` | Tantivy index directory | `./search_index` |
| `MATCHING_THRESHOLD` | Default match cutoff | `0.7` |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://localhost:4317` |
| `OTLP_SERVICE_NAME` | OTel `service.name` | `thing-service` |
| `RUST_LOG` | `tracing-subscriber` filter | `info,thing_service=info` |

## Project layout

```
src/
├── lib.rs              # Library root
├── models/             # Thing, ThingIdentifier, IdentifierType, Consent
├── matching/           # name, description, url, identifier, phonetic, scoring
├── validation/         # validate_thing, normalize_thing
├── privacy/            # mask_thing, gdpr_export
├── api/                # REST + gRPC (stub)
├── web/                # Loco app + Tera views + Axum web router
└── bin/web.rs          # cargo run --bin web

assets/views/           # Tera templates (HTMX + Alpine + Lily)
assets/static/          # lily.css, htmx.min.js, alpine.min.js
config/                 # development.yaml, test.yaml, production.yaml
migrations/             # SeaORM up.sql / down.sql pairs
tests/                  # integration_* (matching, validation, privacy, models, scoring, edge_cases)
benches/                # matching, validation, searching, database_reading/writing, privacy
AGENTS/                 # Reference documentation
```

## Key types

| Type | Module | Description |
|---|---|---|
| `Thing` | `models::thing` | Core entity (schema.org/Thing canonical properties) |
| `ThingIdentifier` | `models::identifier` | `PropertyValue` shape (`property_id`, `value`, `name?`, `url?`) |
| `IdentifierType` | `models::identifier` | Doi / Isbn / Issn / Gtin / Sku / Mpn / SerialNumber / Uri / Uuid / Custom |
| `Consent` | `models::consent` | GDPR consent record |
| `MatchResult` / `MatchBreakdown` | `matching::scoring` | Score + per-component detail |
| `MatchWeights` | `matching::scoring` | Configurable scoring weights |
| `MatchConfidence` | `matching::scoring` | Certain / Probable / Possible / Unlikely |
| `ValidationError` | `validation` | Field + message |

## Key functions

| Function | Module | Description |
|---|---|---|
| `compute_match` | `matching::scoring` | Match two things with weighted scoring |
| `name_similarity` | `matching::name` | Jaro-Winkler |
| `description_similarity` | `matching::description` | Jaro-Winkler |
| `url_similarity` | `matching::url` | Scheme/case-normalized host + path |
| `url_list_similarity` | `matching::url` | Best pair over two URL lists |
| `identifier_similarity` | `matching::identifier` | Exact `(property_id, value)` |
| `has_deterministic_match` | `matching::identifier` | Short-circuit detector |
| `soundex` / `soundex_match` | `matching::phonetic` | 4-char phonetic |
| `validate_thing` | `validation` | Required + format checks |
| `normalize_thing` | `validation` | URL scheme lowercase + dedup |
| `mask_thing` | `privacy` | Owner + identifier masking |
| `gdpr_export` | `privacy` | GDPR Article 15 export |

## Status & roadmap

- **Status** — see [`spec.md §14`](spec.md#14-implementation-status).
- **Tasks** — see [`spec.md §13`](spec.md#13-tasks).
- **Roadmap** — see [`spec.md §15`](spec.md#15-roadmap).
- **Open questions** — see [`spec.md §16`](spec.md#16-open-questions).

## Compliance

| Standard | Mechanism |
|---|---|
| GDPR Art. 15 | `/api/things/{id}/export` (for personal Things) |
| GDPR Art. 17 | Soft delete + consent revocation |
| ISO/IEC 27001 | Operational controls (deployment-side) |

## License

Dual-licensed: Apache-2.0 OR BSD-3-Clause OR GPL-2-or-later OR MIT.
