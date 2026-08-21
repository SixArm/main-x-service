## 14. Implementation Status

### 14.1 Delivered

**organization-matcher-rust-crate** (complete for its scope):

| Capability | Notes |
|---|---|
| Deterministic short-circuits | R-0 (LEI / DUNS / ISO 6523 / GLN / Wikidata / ROR / ISNI / VAT), R-1 same-jurisdiction tax ID, R-2 `same_as` overlap |
| Probabilistic scoring | Name 0.35, address 0.20, url 0.15, jurisdiction 0.10, founding date 0.10, keywords 0.10; renormalised over present components |
| Normalisation | `fold` (NFKC), `legal_name` suffix stripping, `domain`, `fold_set`; diacritics preserved |
| Config | Threshold 0.85; `strict()` 0.95 / `lenient()` 0.70 presets |
| Quality | No IO / `unsafe` / panics; deterministic; per-component breakdown; unit + public-API + doctests |

**organization-service-with-loco** (MVP + recent additions):

| Capability | Notes |
|---|---|
| loco boot + chassis | loco.rs 0.16, Axum 0.8, SeaORM 1.1 |
| Schema + migrations | `organizations` (JSONB payload), `audit_logs` |
| CRUD | Create / list / get / replace / soft-delete; blank-name guard (`422` on create + replace); unknown pid `404` |
| Matching endpoints | `/match` (rank) and `/check-duplicates` (vs stored, ≤ 1 000 rows) embedding the matcher directly — no adapter |
| Name search | `GET /search?q=` — Postgres `ILIKE`, capped 50 *(recent)* |
| Audit log | Best-effort row per CRUD with JSONB snapshot; recent + per-record queries *(recent)* |
| Event streaming | In-memory ring buffer (capacity 1 000) + `/events/recent` *(recent)* |
| Record merge | `POST /merge` folds a duplicate into a survivor (union fields, former-name alias, soft-delete, `merge_records` history + snapshot, `Merged` event); pure `src/merge.rs`; `/merges/recent` *(recent)* |
| Token verification | Offline verification against the auth-service published key (`src/auth.rs`, embeds `authentication-verifier`); `AuthUser`/`MaybeAuthUser` extractors; `/whoami` protected; audit + merge `actor` stamped from the token *(recent)*. Credential is **PASETO v4.public** (Ed25519) per [`authentication-sessions.md`](../../agents/share/authentication-sessions.md) — originally shipped against RS256-JWT/JWKS, since switched (§13 T-9) |
| OpenAPI / Swagger | Hand-written OpenAPI 3 at `/api-docs/openapi.json` + `/swagger-ui` *(recent)* |
| Tests | DB-free `tests/matching.rs` + unit tests (validation `422` pin, 5 `merge` cases); request-level suite `tests/requests/organizations.rs` (Postgres, `#[ignore]`-gated, 9 tests); green build + clippy |
| Hygiene | loco scaffolding leftovers removed (no `workers/` / `data/` / `tasks/` stubs) *(recent)* |

**organization-front-end-with-svelte** (MVP):

| Capability | Notes |
|---|---|
| Routes | `/` list, `/new` create, `/[pid]` detail + delete + check-duplicates, `/[pid]/edit` |
| API layer | Lean raw-JSON client + `OrganizationRepository`; TS mirror of the DTO |
| Form | Builds an `Organization` from inputs (comma lists split, blanks stripped, address assembled only if any field set) |
| Quality | svelte-check strict 0/0; production build green |

### 14.2 Open gaps

Open gaps drive tasks in §13 (entity-level) or the subproject queues.

| Gap | Task |
|---|---|
| Service docs thin (single-file spec, no `agents/`) | T-1 |
| Privacy: masking / GDPR export / consent | T-5 |
| Merge, review queue, batch dedup | T-6 |
| Duplicate check scans ≤ 1 000 rows in-process | T-7 |
| `ILIKE`-only search (no Tantivy / fuzzy / phonetic) | T-8 |
| Token verification (`/whoami` + audit/merge `actor`) verifies PASETO v4.public per [`authentication-sessions.md`](../../agents/share/authentication-sessions.md), but blanket `/api/*` enforcement is wired default-off, and keys are injected via env rather than fetched from the auth service's `/.well-known/paseto-keys` | T-9 follow-up |
| In-memory event stream (not durable, not HA-safe) | T-10 |
| Front-end has vitest + Playwright tests now, but still lacks a search box and audit views | T-11 follow-up |
| Matcher spec is one file (its own §23 queues the split) | matcher §23 |
| Matcher `telephone` / `email` carried but unscored | matcher §23 |
