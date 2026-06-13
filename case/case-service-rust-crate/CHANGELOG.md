# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [0.1.0] - 2026-06-13

Inaugural release. A loco.rs governmental **case** registry, copy-adapted
from the proven `care-pathway-service` with the domain swapped from care
pathway to case.

### Added

- **`cases` table** (`pid`, denormalised `title`, full `Case` payload as
  JSONB `data`, `active`, soft-delete) + `audit_logs` + `merge_records`,
  via `sea-orm-migration`.
- **Embeds `case-matcher` directly**: the API DTO *is*
  `case_matcher::Case`, stored verbatim and matched with the canonical
  engine — no separate model or adapter.
- **CRUD controller** (`/api/cases`): create / list / get / update /
  soft-delete, plus `GET /search?q=` (Postgres `ILIKE` on `title`),
  `POST /match`, `POST /check-duplicates`, `POST /merge`,
  `GET /merges/recent`.
- **Validation → `422`** (family convention): blank `title`, malformed
  `opened_date` (ISO-8601 `YYYY` / `YYYY-MM-DD`), blank identifier value,
  blank `subjects` / `keywords` entries; one response lists every
  problem (`src/validation.rs`).
- **Record merge** (`src/merge.rs` + `models/merge_records.rs`): union
  list fields, keep main's scalars (fall back to the duplicate's), add
  the duplicate's title as a former `alternate_titles` entry; `422` on
  self-merge, `404` on unknown pid.
- **Audit log + in-memory event stream** on every CRUD/merge
  (`models/audit_logs.rs`, `src/streaming.rs`; `created` / `updated` /
  `deleted` / `merged`), with audit / event query endpoints.
- **Offline RS256 JWT verification** (`src/auth.rs`, embeds
  `authentication-verifier`): `GET /whoami` proves end-to-end JWKS
  verification; CRUD/merge stamp the audit + merge `actor` from the
  verified caller. Env: `CASE_JWKS`, `CASE_JWT_ISSUER`,
  `CASE_JWT_AUDIENCE`.
- **OpenAPI 3 + Swagger UI** (`src/openapi.rs`, `controllers/docs.rs`):
  `/api-docs/openapi.json` + `/swagger-ui`.
- **Tests.** DB-free unit tests (validation, merge, auth crypto, openapi,
  streaming, `escape_like`) + `tests/matching.rs` (matcher embedding +
  JSON round-trip) run on `cargo test`. Request-level integration tests
  (`tests/requests/cases.rs`, loco testing harness) cover every endpoint;
  `#[ignore]`-gated on a PostgreSQL `DATABASE_URL` (`cargo test -- --ignored`).

### Notes

- MVP scope is CRUD + `ILIKE` title search + matching. Tantivy full-text
  search, search-blocked dedup candidates, durable event bus, privacy,
  and blanket `/api/*` JWT enforcement are tracked in spec §13.
