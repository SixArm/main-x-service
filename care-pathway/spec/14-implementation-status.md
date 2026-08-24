## 14. Implementation Status

Honest snapshot. The entity is an **MVP**: CRUD + matching.
Aspirational items live in §15, not here.

### 14.1 Delivered

| Subproject | Capability | Notes |
|---|---|---|
| matcher | Domain model | `CarePathway` + `ConditionCode`/`CodeSystem`, `CareSetting`, `PathwayIdentifier`/`IdentifierScheme` |
| matcher | Deterministic matching | R-0 identifier schemes (DOI/Wikidata/GuidelineId/URI/UUID), R-1 provider+code, R-2 `same_as` overlap → 1.0 |
| matcher | Probabilistic matching | Name (Jaro-Winkler + Soundex bonus), condition-code Jaccard, pathway code, care setting, interventions/keywords Jaccard; renormalised weights; presets strict/default/lenient |
| matcher | Quality bar | No `unsafe`/`unwrap`/`panic`; deterministic; diacritic-preserving; unit + public-API tests + doctests; demo binary |
| matcher | Docs | `spec/index.md` (§1–§25, single file), `AGENTS.md` + 4 AGENTS guides, README, CHANGELOG |
| service | loco.rs chassis | loco 0.16, Axum 0.8, SeaORM 1.1; `cargo loco start`; config yamls; port 5150 |
| service | Persistence | `care_pathways` table (pid, name, JSONB `data`, active, `deleted_at`); migration; auto-migrate in dev |
| service | CRUD | Create / list (cap 100) / read / replace / soft-delete; `404` unknown pid; blank-name rejection (`422`, create + update) |
| service | Validation | `condition_codes` format-checked per `system` — ICD-10, ICD-11, SNOMED CT (SCTID Verhoeff); all problems reported in one `422` (`src/validation.rs`) |
| service | API docs | OpenAPI 3 (`src/openapi.rs`, hand-written) + Swagger UI at `/api-docs/openapi.json` · `/swagger-ui` (`controllers/docs.rs`) |
| service | Matching endpoints | `/match` (rank explicit candidates), `/check-duplicates` (scan ≤ 1 000 stored rows, ranked hits) |
| service | Audit + streaming | `audit_logs` table + best-effort row per CRUD (action + snapshot + `actor`); in-memory `PathwayEvent` stream (cap 1 000); read at `/audit/recent`, `/{pid}/audit`, `/events/recent` |
| service | Name search | `GET /search?q=` — Postgres `ILIKE` substring match on the denormalised `name` (cap 50, wildcards escaped); blank `q` → `400` |
| service | Record merge | `POST /merge` folds a duplicate into a survivor (union fields, former-title alias, soft-delete, `merge_records` history, `Merged` event); pure `src/merge.rs`; `/merges/recent` history |
| service | Token verification | Offline bearer-token verification against the auth-service's published key (`src/auth.rs`, embeds `authentication-verifier`); `AuthUser`/`MaybeAuthUser` extractors; `/whoami` protected; audit `actor` stamped from the token. Credential is **PASETO v4 public** (Ed25519) per [`authentication-sessions.md`](../../agents/share/authentication-sessions.md) — originally RS256-JWT, since switched (T-7) |
| service | Tests | DB-free `tests/matching.rs` (matcher embedding + JSON round-trip) + controller validation unit tests (422 pin); request-level loco tests `tests/requests/care_pathways.rs` (`#[ignore]`-gated on Postgres); green build + clippy |
| front-end | Routes | `/`, `/new`, `/[pid]` (detail + delete + check-duplicates), `/[pid]/edit` |
| front-end | API layer | Lean raw-JSON client, `CarePathwayRepository`, hand-mirrored TS types |
| front-end | Form | Full-DTO editing incl. condition-code and identifier row editors |
| front-end | Quality bar | `pnpm run check` strict 0/0; production build green |
| service | Cross-service journey links | `entity_links` write-side for the `continues_as` edge (§9): `POST`/`GET`/`DELETE /api/instances/{pid}/links` plus the aggregator's reconciliation pull `GET /api/instances/links`. High-sensitivity governance — authorised at the read-the-journey level, audited, bulk pull privileged. The third originating service in the family |
| service | Stitched journeys | `GET /api/instances/{pid}/journey` — follows `continues_as` across services, fetching each leg under the **caller's** credential (never a service identity), bounded and cycle-safe, withholding combined figures unless every leg resolved (`src/journey.rs`) |
| service | Time-based analysis | `instance_segments` + the `clock_start_at`/`clock_stop_at` columns (backfilled); the pure `src/tba.rs` (interval union/subtract, the four-bucket clock partition, gaps, handoffs, nearest-rank percentiles, the NHS access-standard catalogue, cohort rollup, constraint ranking, Little's Law — no I/O, `as_of` a parameter); nine endpoints under `src/controllers/tba.rs`; OpenAPI-documented, guarded, audited. See [`time-based-analysis.md`](time-based-analysis.md) |
| service | Flow gauges | `src/flow_metrics.rs` — a default-off refresh loop publishing the `care_pathway_flow_*` family: cohort %VA, p90 lead time, coverage and instance count per pathway, capped and small-cohort-suppressed because `/metrics.prom` is on the public allow-list. Neither bound is silent |
| front-end | Time-based analysis | `/time`: the cohort ratio, coverage, lead-time percentiles, a score against a named NHS access standard, the constraint ranking, Little's-Law flow, and one journey's timeline wall (`src/lib/api/tba.ts` + `src/lib/components/JourneyTimeline.svelte`). `nav.time` translated across all 13 locales |
| front-end | Tests | vitest units (`tests/unit/`, 16 — client + repository, `check-duplicates` regression) + Playwright smoke (`tests/e2e/`, 4 routes, API-stubbed, runs on `vite preview`) |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| Single-file crate specs; no service `agents/` reference set | T-1 |
| Event streaming is in-memory only (process-local ring buffer); no durable broker, no cross-replica delivery | T-3 follow-up / §15 |
| Request-level tests exist but are `#[ignore]`-gated; no DB-backed run in CI yet | T-4 follow-up |
| Front-end tests run locally but aren't wired into CI; no merge-action UI yet | T-5 follow-up |
| Name search is Postgres `ILIKE` only — no full-text/fuzzy search over the JSONB payload, and `check-duplicates` still full-scans (capped at 1 000 rows) rather than using search-blocked candidates | T-6 follow-up |
| Offline token verification (extractor + `/whoami` + audit `actor`) verifies PASETO v4 public per [`authentication-sessions.md`](../../agents/share/authentication-sessions.md), but blanket `/api/*` enforcement is wired default-off (`CARE_PATHWAY_REQUIRE_AUTH`), and the keys are injected via env rather than fetched from the auth service's `/.well-known/paseto-keys` | T-7 follow-up |
| Record merge has no front-end action yet (backend `POST /merge` is done) | T-8 follow-up / T-5 |
| No terminology-server check that codes exist in a published release (formats are validated; existence is not) | T-9 follow-up |
| No privacy controls (none required while no restricted fields exist — §12.3) | (re-assess; no task) |
| No localization of the operator UI | (roadmap §15; no task yet) |
| `patient-flow` exposes no timeline endpoint, so a stay leg of a stitched journey reports `not_configured` — the contract it must satisfy is in `src/journey.rs` | patient-flow adoption |
| The instance and insight endpoints are still absent from `openapi.json` (the TBA surface is documented; they are not) | time-based-analysis §17 |
