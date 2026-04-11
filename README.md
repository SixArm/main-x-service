# Main X Index Rust crate

@AGENTS/share/overview.md

Subprojects:

- [Main Person Index Rust crate](main-person-index-rust-crate/)
- [Main Place Index Rust crate](main-place-index-rust-crate/)
- [Main Thing Index Rust crate](main-thing-index-rust-crate/)
- [Main Event Index Rust crate](main-event-index-rust-crate/)
- [Main Patient Index Rust crate](main-patient-index-rust-crate/)
- [Main Worker Index Rust crate](main-worker-index-rust-crate/)

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Docker Deployment](#docker-deployment)
- [Technology Stack](#technology-stack)
- [Architecture](#architecture)
- [Development](#development)
- [API Documentation](#api-documentation)
- [Configuration](#configuration)
- [Testing](#testing)
- [Deployment](#deployment)
- [Security & Compliance](#security--compliance)
- [Performance](#performance)
- [Contributing](#contributing)

## Features

### Data Management

- Create, read, update, and delete (CRUD) records
- Soft delete support with complete audit trails
- Identifier management; multiple identifiers per record.
- Identity document management; multiple identity documents per record.
- Contact information management; multiple contacts per record.
- Automatic event stream publishing for all CRUD operations

### Matching

- **Probabilistic Matching**: Advanced fuzzy matching algorithms
- **Deterministic Matching**: Rule-based exact matching
- **Configurable Scoring**: Customizable match thresholds and weights
- **Match Components**:
  - String matching (Jaro-Winkler, Levenshtein, Soundex phonetic)
  - Date matching with error tolerance
  - Identifier matching
  - Identification document matching
- **Score Breakdown**: Full per-component score breakdown in API responses

@AGENTS/architecture.md
@AGENTS/matching.md
@AGENTS/models.md
@AGENTS/restful.md
@AGENTS/testing.md

@AGENTS/share/auditability.md
@AGENTS/share/availability.md
@AGENTS/share/match-search-merge.md
@AGENTS/share/observability.md
@AGENTS/share/privacy.md
@AGENTS/share/restful.md
@AGENTS/share/technology.md

### Data Quality & Validation

- Required field enforcement
- Date validation
- ID format validation
- Email format validation
- Phone number digit count validation
- Address validation (requires city, postal code, or country)
- Document validation (required number, expiry check, issue-before-expiry)
- Phone number normalization (E.164-like format)
- Address standardization (title-case city, uppercase state/country, expand abbreviations)
- Validation integrated into create and update handlers (returns 422)

## Quick Start

### Option 1: Docker (Recommended)

```sh
# Clone repository
git clone https://github.com/sixarm/main-x-index-rust-crate.git
cd main-x-index-rust-crate
```

**Services Available:**

- **API**: http://localhost:8080/api
- **Swagger UI**: http://localhost:8080/swagger-ui
- **pgAdmin** (optional): http://localhost:5050
  ```bash
  docker-compose --profile tools up -d
  ```

See [DEPLOY.md](DEPLOY.md) for complete deployment guide.

### Option 2: Local Development

**Prerequisites:**

- Rust 1.93+ ([Install Rust](https://rustup.rs/))
- PostgreSQL 18+
- SeaORM CLI: `cargo install sea-orm-cli`

### Data Flow

**Create Flow:**

1. HTTP POST -> REST API Handler
2. Validation (required fields, format checks)
3. Duplicate Detection (search + match against existing)
4. If duplicates found: return 409 with matches
5. Repository `create()` -> Database INSERT
6. Search Engine `index_person()` -> Tantivy Index
7. Event Publisher -> Created Event
8. Audit Logger -> audit_log INSERT
9. HTTP Response -> Client

**Person Merge Flow:**

1. HTTP POST /merge -> REST API Handler
2. Fetch master and duplicate from database
3. Transfer data from duplicate to master
4. Update master in database
5. Soft-delete duplicate
6. Update search index
7. Publish Merged event
8. Return merge record with transferred data

**Person Search Flow:**

1. HTTP GET -> REST API Handler
2. Search Engine `search()` -> Tantivy Query
3. Person IDs -> Repository `get_by_id()` batch
4. Optional: mask sensitive data
5. Person Records -> JSON Serialization
6. HTTP Response -> Client (with pagination)
