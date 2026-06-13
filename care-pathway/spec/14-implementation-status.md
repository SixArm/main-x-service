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
| service | Matching endpoints | `/match` (rank explicit candidates), `/check-duplicates` (scan ≤ 1 000 stored rows, ranked hits) |
| service | Tests | DB-free `tests/matching.rs` (matcher embedding + JSON round-trip) + controller validation unit tests (422 pin); request-level loco tests `tests/requests/care_pathways.rs` (`#[ignore]`-gated on Postgres); green build + clippy |
| front-end | Routes | `/`, `/new`, `/[pid]` (detail + delete + check-duplicates), `/[pid]/edit` |
| front-end | API layer | Lean raw-JSON client, `CarePathwayRepository`, hand-mirrored TS types |
| front-end | Form | Full-DTO editing incl. condition-code and identifier row editors |
| front-end | Quality bar | `pnpm run check` strict 0/0; production build green |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| Single-file crate specs; no service `AGENTS/` reference set | T-1 |
| No audit log, no event streaming | T-3 |
| Request-level tests exist but are `#[ignore]`-gated; no DB-backed run in CI yet | T-4 follow-up |
| No front-end unit / e2e tests | T-5 |
| No full-text search; `check-duplicates` full scan capped at 1 000 rows | T-6 |
| No authentication on `/api/*` | T-7 |
| No merge workflow | T-8 |
| No OpenAPI/Swagger; no ICD/SNOMED format validation | T-9 |
| No privacy controls (none required while no restricted fields exist — §12.3) | (re-assess; no task) |
| No localization of the operator UI | (roadmap §15; no task yet) |
