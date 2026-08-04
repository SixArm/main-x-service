# Thing Service

A registry of **thing identities** based on
[schema.org/Thing](https://schema.org/Thing). The Thing Service is the
**most general** entity in the Main X Index family — books, papers,
software, devices, products, digital assets — anything that doesn't fit
one of the more opinionated sibling crates (`person`, `worker`,
`event`, `place`, `course`).

Identifiers follow
[schema.org/PropertyValue](https://schema.org/PropertyValue): DOI, ISBN,
ISSN, GTIN, SKU, MPN, SerialNumber, URI, UUID, or `Custom(String)`.
The first seven are globally unique and short-circuit matching to
`1.0` on exact match. It carries no gRPC surface (unlike
person/worker/event) — see the honest capability matrix in
[`agents/share/overview.md`](../../agents/share/overview.md).

## Features

- CRUD on Thing records with soft delete; schema.org/Thing canonical
  properties (`name`, `alternateName`, `description`,
  `disambiguatingDescription`, `additionalType`, `url`, `identifier`,
  `image`, `mainEntityOfPage`, `owner`, `sameAs`, `subjectOf`,
  `potentialAction`).
- Required-field enforcement (`name`); URL format checks; per-type
  identifier format checks (ISBN 10/13, ISSN 8-digit, DOI `10.*/*`,
  GTIN 8/12/13/14-digit, UUID, URI); dedupe on `alternate_names` /
  `same_as` / `images`; URL scheme normalization.
- Per-field masking (`owner` → withheld, identifier `value` → last 4
  visible, per-identifier `url` cleared); GDPR Article 15 export at
  `GET /api/things/{id}/export`; a consent model
  (`DataProcessing`/`DataSharing`/`Marketing`/`Research` ×
  `Active`/`Revoked`/`Expired`).
- Row-level integrity digests (SHA-256, SHA-3, and an optional keyed
  MAC) over the assembled record — including child-table
  identifiers, not just the root row — recomputed on demand via
  `GET /api/records/verify` and `GET /api/audit/verify`.

## Quick start

Prerequisites: Rust 1.93+ (2024 edition), PostgreSQL 18+.

```bash
cd thing/thing-service-with-loco   # within the main-x-service monorepo

# Build and run (loco.rs): point DATABASE_URL at the database, then
# start. Migrations run automatically in development (auto_migrate).
export DATABASE_URL=postgres://localhost/thing_service_development
cargo loco start            # or: cargo run -- start

# Service binds on the configured port (config/development.yaml):
curl http://localhost:5150/api/health
```

## API

REST routes mount under `/api/things/*`. See
[`AGENTS/restful.md`](AGENTS/restful.md) for the full list. All
endpoints return the standard `{success, data, error}` envelope.
Duplicate handling includes a stored review queue:
`GET /api/things/review-queue` (filter `status`, `limit`) and
`POST /api/things/review-queue/{id}/decision` (decide a pending item,
`confirmed` / `rejected`).

An HL7 FHIR R5 surface (`src/controllers/fhir.rs`) maps things to the
FHIR `Device` resource: `GET /fhir/metadata` (CapabilityStatement),
`POST /fhir/Device` (create), `GET /fhir/Device` (search),
and `GET` / `PUT` / `DELETE /fhir/Device/{id}`.

Integrity verification: `GET /api/records/verify` and `GET
/api/audit/verify` (`?limit=`, capped at 1000) reassemble each record
(or audit row) and recompute its SHA-256/SHA-3/optional-MAC digests,
naming any row whose stored digest no longer matches its content.

Auth is offline PASETO `v4.public` verification + ABAC, blanket-guarded
behind `THING_REQUIRE_AUTH` (default off — see
[`agents/share/security.md`](../../agents/share/security.md) §4 for why
that default matters before a deployment is reachable).

Prometheus metrics are served outside `/api`, at the application root:

| Method | Path            | Description                                                          |
| ------ | --------------- | ------------------------------------------------------------------- |
| GET    | `/metrics.prom` | Prometheus text-exposition format (`text/plain; version=0.0.4`) for scraping |

Configure your scraper with `metrics_path: /metrics.prom`.

Interactive OpenAPI 3 documentation:

- Swagger UI: `http://localhost:8080/swagger-ui`
- Raw spec: `http://localhost:8080/api-docs/openapi.json`

### Worked example

Create a book by ISBN:

```bash
curl -X POST http://localhost:8080/api/things \
  -H 'content-type: application/json' \
  -d '{
    "name": "Pride and Prejudice",
    "description": "A novel of manners by Jane Austen.",
    "additional_type": "https://schema.org/Book",
    "url": "https://en.wikipedia.org/wiki/Pride_and_Prejudice",
    "same_as": [
      "https://www.wikidata.org/wiki/Q170583",
      "https://openlibrary.org/works/OL1394865W"
    ],
    "identifiers": [
      { "property_id": "Isbn", "value": "9780141439518" }
    ]
  }'
```

Match against a typo'd candidate (deterministic short-circuit on
shared ISBN):

```bash
curl -X POST http://localhost:8080/api/things/match \
  -H 'content-type: application/json' \
  -d '{
    "name": "Prde and Prejudice",
    "identifiers": [{ "property_id": "Isbn", "value": "9780141439518" }]
  }'
# → score 1.0 (deterministic)
```

## Configuration

Configuration is loaded from `config/{development,test,production}.yaml`
(Loco convention). Environment-overridable variables:

| Variable             | Description                | Default                 |
| -------------------- | -------------------------- | ----------------------- |
| `PORT`               | REST bind port — read by `config/production.yaml` only; `development.yaml` hardcodes `5150` (see the quick start above) | `8080` (production) |
| `DATABASE_URL`       | Postgres connection string | per config file         |
| `SEARCH_INDEX_PATH`  | Tantivy index directory    | `./data/search_index`   |
| `MATCHING_THRESHOLD` | Probabilistic match cutoff | `0.7`                   |
| `SEARCH_CACHE_SIZE_MB` | Tantivy cache budget in MB | `512` |
| `THING_EVENT_TRANSPORT` | Event bus transport: `memory` (in-memory publish) or `outbox` (durable transactional outbox — one `event_outbox` row per change, written inside the entity write's transaction) | `memory` |
| `THING_EVENT_RELAY` | Enable the Phase-3 outbox relay loop (`1`/`true`/`yes`/`on`); only runs when the transport is `outbox` | `off` |
| `THING_EVENT_RELAY_INTERVAL_SECS` | Relay poll interval in seconds (floored at 1) | `5` |
| `THING_EVENT_RETENTION_DAYS` | Outbox row TTL; the relay's retention sweep purges published rows older than this | `7` |
| `THING_FLUVIO_ENDPOINT` | Fluvio broker SC address; selects the real-broker `FluvioSink` over the default `LoggingSink` (requires this crate's `fluvio` Cargo feature, off by default) | unset (`LoggingSink`) |
| `THING_EVENT_TOPIC` | Topic the relay publishes to | `mxi.thing.events` |
| `THING_REQUIRE_AUTH` | Blanket `/api/*` PASETO + ABAC enforcement (`1`/`true`/`yes`/`on`) | off |
| `RUST_LOG`           | tracing-subscriber filter  | `info`                  |

> `GRPC_PORT`, `OTLP_SERVICE_NAME`, `OTLP_ENDPOINT`,
> `STREAMING_BROKER_URL`, `STREAMING_TOPIC`, `DATABASE_MAX_CONNECTIONS`,
> `DATABASE_MIN_CONNECTIONS`, `SERVER_HOST`, and `SERVER_PORT` also
> parse into this crate's legacy `src/config/mod.rs::Config` struct but
> are **not read anywhere else in `src/`** — a pre-loco holdover. There
> is no gRPC server ([T-3](spec/13-tasks.md) is still open) and no OTLP
> exporter; the real REST bind address/port and DB pool size come from
> loco's own `config/*.yaml`.

## Testing

```bash
# Unit tests (models, matching components, validation, privacy).
cargo test --lib

# Bridge tests pinning the service ↔ canonical thing-matcher
# contract (DOI/ISBN/UUID deterministic short-circuits, SKU
# non-deterministic distinction, same_as URL contribution).
cargo test --test duplicate_detection

# Integration tests across the per-pipeline files.
cargo test --tests

# Criterion benchmarks (matching, validation, search, privacy).
cargo bench
```

See [`AGENTS/testing.md`](AGENTS/testing.md) for the layout.

## Matching at a glance

| Component   | Weight | Algorithm                               |
| ----------- | -----: | --------------------------------------- |
| Name        |   0.40 | Jaro-Winkler + Soundex phonetic bonus   |
| Identifier  |   0.30 | `(property_id, value)` exact, best pair |
| Description |   0.10 | Jaro-Winkler (case-insensitive)         |
| URL         |   0.10 | Scheme/case-normalized host + path      |
| Same-as     |   0.10 | Best pair across `same_as` URL lists    |

Deterministic short-circuit: any matched DOI / ISBN / ISSN / GTIN /
MPN / SerialNumber / UUID → 1.0. (SKU / URI / Custom are evidence,
not pins — they are not globally unique.)

See [`AGENTS/matching.md`](AGENTS/matching.md) for the per-component
detail.

## Compliance

- **GDPR**: right of access via `GET /api/things/{id}/export`;
  right to erasure via soft-delete + `/masked` view. Consent records
  for Things subject to data-protection regimes (e.g. a personal
  device).
- **Identifier privacy**: serial numbers and custom identifier values
  are masked (last 4 visible); identifier URLs stripped from masked
  view.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only
OR GPL-3.0-only.
