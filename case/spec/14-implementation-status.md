## 14. Implementation Status

Honest snapshot. The entity delivers CRUD + matching + audit +
streaming + merge + offline token verification + OpenAPI. Aspirational items live
in §15, not here. The headline gap — **privacy controls (masking /
GDPR export)** — is honest and high-priority because case data is
personal data (§12).

### 14.1 Delivered

| Subproject | Capability | Notes |
|---|---|---|
| matcher | Domain model | `Case` + `CaseType` / `CaseStatus` / `Priority` / `CaseIdentifier` / `IdentifierScheme` |
| matcher | Deterministic matching | R-0 identifier schemes (`Docket`/`ExternalCaseId`/`Uri`/`Uuid`), R-1 agency + `case_number`, R-2 `same_as` overlap → 1.0 |
| matcher | Probabilistic matching | Title (Jaro-Winkler + Soundex bonus), subjects Jaccard, case number, case type, status, keywords Jaccard; renormalised weights; presets strict/default/lenient |
| matcher | Quality bar | No `unsafe`/`unwrap`/`panic`; deterministic; diacritic-preserving; unit + public-API tests + doctests; demo binary |
| service | loco.rs chassis | loco 0.16, Axum 0.8, SeaORM 1.1; `cargo loco start`; config yamls; port 5150 |
| service | Persistence | `cases` table (pid, title, JSONB `data`, active, `deleted_at`); `audit_logs`; `merge_records`; migrations; auto-migrate in dev |
| service | CRUD | Create / list (cap 100) / read / replace / soft-delete; `404` unknown pid |
| service | Validation | Blank `title` → `422` (create + update); `opened_date` ISO check; blank identifier value / subject / keyword → `422`; all problems in one response (`src/validation.rs`) |
| service | Title search | `GET /search?q=` — Postgres `ILIKE` substring match on the denormalised `title` (cap 50, wildcards escaped); blank `q` → `400` |
| service | Matching endpoints | `/match` (rank explicit candidates), `/check-duplicates` (scan ≤ `CHECK_DUPLICATES_SCAN_CAP` rows, WARN at cap, ranked hits) |
| service | Audit + streaming | `audit_logs` table + best-effort row per CRUD/merge (action + snapshot + `actor`); in-memory `CaseEvent` stream (cap 1 000); read at `/audit/recent`, `/{pid}/audit`, `/events/recent` |
| service | Record merge | `POST /merge` folds a duplicate into a survivor (union fields, former-title alias, soft-delete, `merge_records` history, `Merged` event); pure `src/merge.rs`; `/merges/recent` history |
| service | Token verification | Offline bearer-token verification against the auth-service's published key (`src/auth.rs`, embeds `authentication-verifier`); `AuthUser`/`MaybeAuthUser`; `/whoami` protected; audit/merge `actor` stamped from the token. Credential is switching RS256-JWT → PASETO v4 public per [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md) (source of truth; supersedes RS256-JWT + JWKS), §13 T-7 |
| service | API docs | OpenAPI 3 (`src/openapi.rs`, hand-written) + Swagger UI at `/api-docs/openapi.json` · `/swagger-ui` |
| service | Tests | DB-free `tests/matching.rs` + module unit tests (validation, merge, streaming, auth crypto, openapi, scan-cap); request-level loco tests `tests/requests/cases.rs` (`#[ignore]`-gated on Postgres); green build + clippy |
| front-end | Routes | `/`, `/new`, `/[pid]` (detail + delete + check-duplicates), `/[pid]/edit` |
| front-end | API layer | Lean raw-JSON client, `CaseRepository`, hand-mirrored TS types |
| front-end | Form | Full-DTO editing incl. type/status/priority selects, date input, identifier row editor, comma-list fields |
| front-end | Quality bar | `pnpm run check` strict 0/0; production build green |
| front-end | Tests | vitest units (client + repository, `check-duplicates` regression) + Playwright smoke (4 routes, API-stubbed, runs on `vite preview`) |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| **No privacy controls** — no per-field masking, no masked-view endpoint, no GDPR data-subject export, no erasure path. *Highest-priority gap: case data is personal data (§12).* | T-10 |
| Event streaming is in-memory only (process-local ring buffer); no durable broker, no cross-replica delivery | T-12 / §15 |
| Title search is Postgres `ILIKE` only — no full-text/fuzzy search over the JSONB payload, and `check-duplicates` still full-scans (capped at 1 000 rows) rather than using search-blocked candidates | T-6 |
| No front-end search box, audit view, or event view (service endpoints exist ahead of the UI) | T-11 |
| Token verification exists (extractor + `/whoami` + audit `actor`) but is not yet *enforced* on every `/api/*` route, the credential is not yet switched to PASETO v4 public per [`authentication-sessions.md`](../../agents/share/authentication-sessions.md), and the published key is injected via env rather than fetched from the auth service | T-7 follow-up |
| Record merge has no front-end action yet (backend `POST /merge` is done) | T-8 follow-up / T-11 |
| Request-level tests exist but are `#[ignore]`-gated; no DB-backed run wired into CI yet | T-4 follow-up |
| No deeper validation (docket / case-number format, status transitions) | T-9 follow-up |
| No real-time duplicate detection on create (`409`) | (roadmap §15; OQ-4) |
| No localization of the operator UI | (roadmap §15; no task yet) |
| Thin service / front-end `AGENTS/` reference sets | T-13 |
