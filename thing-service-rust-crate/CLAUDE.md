# Thing Service

The Thing Service is a generic registry for arbitrary discrete
objects — books, papers, digital assets, devices, products, software,
or any other "thing" — modelled one-to-one on
[schema.org/Thing](https://schema.org/Thing). It is the most general
entity in the Main X Index family; if your record doesn't fit one of
the more opinionated sibling crates (`person`, `worker`,
`event`), it goes here.

The Thing service shares its architecture with every other Main X
Index crate: REST + gRPC API on top of PostgreSQL, Tantivy, and an
event stream.

@agents/share/overview.md

## Features

### Thing identity management

- CRUD on Thing records with soft delete
- Schema.org/Thing canonical properties: `name`, `alternateName`,
  `description`, `disambiguatingDescription`, `additionalType`, `url`,
  `identifier`, `image`, `mainEntityOfPage`, `owner`, `sameAs`,
  `subjectOf`, `potentialAction`
- Typed identifiers via `PropertyValue` shape: DOI, ISBN, ISSN, GTIN,
  SKU, MPN, SerialNumber, URI, UUID, or `Custom(String)`
- Multiple `alternate_names`, multiple `images`, multiple `same_as`
  authoritative URLs (Wikipedia, Wikidata, …)
- `additional_type` points at the schema.org sub-type URL the record
  best fits (e.g. `https://schema.org/Book`)
- Automatic event publishing on every CRUD operation

### Thing matcher

- **Probabilistic** — weighted across name, identifier, description,
  url, sameAs (default weights: 0.40 / 0.30 / 0.10 / 0.10 / 0.10)
- **Deterministic** — match on any globally-unique identifier (DOI,
  ISBN, ISSN, GTIN, MPN, SerialNumber, UUID) short-circuits to `1.0`
- **Score breakdown** — per-component scores in every match response
- Algorithms: Jaro-Winkler, Soundex phonetic, scheme/case-normalized
  URL comparison

### Data quality & validation

- Required-field enforcement (`name`)
- URL format checks on `url`, `additional_type`, `main_entity_of_page`,
  `subject_of`, each `image`, and each `same_as` entry (must be
  http:// or https://)
- Identifier format checks per type:
  - **ISBN**: 10 or 13 digits (dashes / spaces tolerated; trailing `X` allowed for ISBN-10)
  - **ISSN**: 8 digits (dashes / spaces tolerated; trailing `X` allowed)
  - **DOI**: must start with `10.` and contain `/`
  - **GTIN**: 8, 12, 13, or 14 digits
  - **UUID**: parseable per RFC 4122
  - **URI**: must contain a scheme separator (`:`)
  - **Other** (SKU, MPN, SerialNumber, Custom): format unconstrained
- Empty `alternate_names` / identifier values rejected
- URL scheme normalization (scheme lowercased; host/path preserved)
- Dedupe on `alternate_names`, `same_as`, `images`
- Validation runs on create / update (returns `422`)

### Privacy

- Per-field masking:
  - `owner` → `"[owner withheld]"`
  - identifier `value` → `"****<last 4 chars>"`; per-identifier `url` cleared
  - `property_id` preserved
- GDPR Article 15 export at `GET /api/things/{id}/export`
- Consent model with `DataProcessing` / `DataSharing` / `Marketing` /
  `Research` types and `Active` / `Revoked` / `Expired` statuses

@AGENTS/index.md
@AGENTS/matching.md
@AGENTS/models.md
@AGENTS/restful.md
@AGENTS/testing.md

@agents/share/auditability.md
@agents/share/availability.md
@agents/share/match-search-merge.md
@agents/share/observability.md
@agents/share/privacy.md
@agents/share/restful.md
@agents/share/technology.md

## Quick start

### Local dev

**Prerequisites:** Rust 1.75+, PostgreSQL 15+ (optional for in-memory paths)

```bash
git clone https://github.com/sixarm/thing-service-rust-crate.git
cd thing-service-rust-crate

# REST API
cargo run --release

# Tests
cargo test --lib
```

## API examples

**Create a thing**

```bash
curl -X POST http://localhost:8080/api/things \
  -H "Content-Type: application/json" \
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

**Check for duplicates**

```bash
curl -X POST http://localhost:8080/api/things/check-duplicates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Pride and Prejudice",
    "identifiers": [
      { "property_id": "Isbn", "value": "9780141439518" }
    ]
  }'
```

**Search**

```bash
curl "http://localhost:8080/api/things/search?q=Pride+and+Prejudice&limit=10&offset=0&fuzzy=true&mask_sensitive=true"
```

**Match**

```bash
curl -X POST http://localhost:8080/api/things/match \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Prde and Prejudice",
    "identifiers": [
      { "property_id": "Isbn", "value": "9780141439518" }
    ],
    "threshold": 0.7
  }'
```

**Merge**

```bash
curl -X POST http://localhost:8080/api/things/merge \
  -H "Content-Type: application/json" \
  -d '{
    "main_thing_id": "uuid-main",
    "duplicate_thing_id": "uuid-dup",
    "merge_reason": "Confirmed duplicate"
  }'
```

**Batch deduplication**

```bash
curl -X POST http://localhost:8080/api/things/deduplicate \
  -H "Content-Type: application/json" \
  -d '{ "threshold": 0.7, "auto_merge_threshold": 0.95, "max_candidates": 50 }'
```

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | — |
| `DATABASE_MAX_CONNECTIONS` | Pool max | 10 |
| `DATABASE_MIN_CONNECTIONS` | Pool min | 2 |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `SEARCH_INDEX_PATH` | Tantivy index dir | `./search_index` |
| `MATCHING_THRESHOLD` | Default match threshold | `0.7` |
| `RUST_LOG` | Log filter | `info` |

## Testing

```bash
cargo test --lib                 # All unit tests
cargo test --tests               # All integration tests
cargo test --test integration_matching
cargo bench                      # Benchmarks
```

## Security & compliance

- Audit logging for every CRUD operation
- Soft delete (records never truly deleted)
- Non-root containers
- Environment-based secrets (no secrets in code/images)
- Configurable CORS
- Per-field privacy masking (owner, identifier values)
- GDPR data export endpoint
- Consent model with type/status tracking
- Validation on create/update

## License

Dual-licensed: Apache-2.0 OR BSD-3-Clause OR GPL-2-or-later OR MIT.
