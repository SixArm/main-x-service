## 14. Implementation Status

Honest snapshot, rewritten 2026-08 (professionalization audit) against
`case/case-service-with-loco/CHANGELOG.md` and a grep of its `src/` —
not against this file's own previous text, which had drifted enough to
contradict itself (an earlier draft said in one line that the
credential was already PASETO v4 public and, a few lines later, that
it "is not yet switched to PASETO" — both cannot be true; the resolved
answer, confirmed against `src/auth.rs`, is that PASETO v4 public
**is** and has been the credential since the family's RS256→PASETO
pivot). Aspirational items live in §15.

The crate-level spec
([`case-service-with-loco/spec/index.md`](../../case/case-service-with-loco/spec/index.md)
§13/§14) is the more granular and more frequently updated record; this
file is the entity-level summary across matcher + service + front-end
and should be read as a coarser view of the same facts, not a
competing source.

### 14.1 Delivered

| Subproject | Capability | Notes |
|---|---|---|
| matcher | Domain model | `Case` + `CaseType` / `CaseStatus` / `Priority` / `CaseIdentifier` / `IdentifierScheme` |
| matcher | Deterministic matching | R-0 identifier schemes (`Docket`/`ExternalCaseId`/`Uri`/`Uuid`), R-1 agency + `case_number`, R-2 `same_as` overlap → 1.0 |
| matcher | Probabilistic matching | Title (Jaro-Winkler + Soundex bonus), subjects Jaccard, case number, case type, status, keywords Jaccard; renormalised weights; presets strict/default/lenient |
| matcher | Quality bar | No `unsafe`/`unwrap`/`panic`; deterministic; diacritic-preserving; unit + public-API tests + doctests; demo binary; a `proptest` harness + `cargo-fuzz` targets since the family-wide SEC-M6/FUZZ-2 rounds |
| service | loco.rs chassis | loco 1.0.1 (Axum 0.8, SeaORM 2.0 as of the 2026-08-02 bump); `cargo loco start`; config yamls; port 5150 |
| service | Persistence | `cases`, `audit_logs`, `merge_records`, `entity_links`, `event_outbox`, `review_queue`, `bulk_jobs` tables; migrations; auto-migrate in dev |
| service | CRUD | Create / list / read / replace / soft-delete; `404` unknown pid; `?limit=`/`?offset=` pagination with `X-Total-Count`/`X-Limit`/`X-Offset` headers (2026-08-01) |
| service | Validation | Blank `title` → `422` (create + update); `opened_date` ISO check; blank identifier value / subject / keyword → `422`; per-field/array size caps (SEC-M1); all problems in one response (`src/validation.rs`) |
| service | Search | **Tantivy** full-text/fuzzy/phonetic search, replacing the original Postgres `ILIKE` title search — `src/search/` (`CaseIndex`/`SearchEngine`), `GET /api/cases/search?q=` with `?fuzzy=true`/`?phonetic=true`, `503` on index unavailability. **Landed 2026-08-02**, not "search TBD" as an earlier draft of this file said. |
| service | Matching endpoints | `/match` (rank explicit candidates), `/check-duplicates` — now scores a **search-blocked** candidate set from the Tantivy index (up to 200: fuzzy title, exact identifier, phonetic title) instead of the old capped 1 000-row in-memory scan |
| service | Audit + streaming | `audit_logs` table with the audit write now **inside the same transaction** as the entity mutation and its outbox row under the `outbox` transport (2026-07-09 — "the three can never disagree"); durable event bus **Phases 1–3**: versioned `Envelope` + `EventPublisher`/`EventSink` seam (`src/streaming.rs`), a transactional Postgres outbox (`models/event_outbox.rs`), and a relay to a real broker (`src/relay.rs`'s `FluvioSink`, behind the `fluvio` Cargo feature and `CASE_FLUVIO_ENDPOINT`, default-off, landed 2026-08-03). No deployment yet points a live broker at it — the sink is wired and feature-compiled but its live-broker round trip is `#[ignore]`d, not exercised by this repo's CI. |
| service | Record merge | `POST /merge` folds a duplicate into a survivor (union fields, former-title alias, soft-delete, `merge_records` history, `Merged` event, participant rows locked against a concurrent-merge race — SEC-B5); pure `src/merge.rs`; `/merges/recent` history |
| service | Privacy / masking | `mask_case` (`src/controllers/cases.rs`) redacts `subjects`/`identifiers`/`same_as`/`case_number`, wired to the ABAC `mask` obligation on **every** read path (native `GET`/`list`/`search`/`check-duplicates` **and** FHIR `read`/`search` — SEC-G2/G3, closing an earlier gap where only the single-record native `GET` was covered); `GET /{pid}/masked` (always-masked view) and the audited `GET /{pid}/export` GDPR right-of-access envelope both **landed 2026-08-02**. GDPR Art. 17 erasure (`POST /{pid}/erase`, `access=admin`) landed earlier, 2026-07-25..27, as part of the compliance suite below. **Not built:** a subject-scoped export spanning every case that shares one `subjects` id (today's export is per-case). |
| service | Cross-service links | `entity_links` write side, **case is the family's reference/first originating implementation** (landed 2026-07-10, ahead of person/worker). Case originates exactly one edge kind: **`subject_of`** (case → person), via `POST`/`GET`/`DELETE /api/cases/{pid}/links` (`src/controllers/links.rs`); optimistic write (no cross-service call), `linked`/`unlinked` events on the same outbox transaction. `GET /api/cases/links[?since=]` is the bulk pull for the link-graph aggregator's reconciliation, gated as a privileged/audited governed read (SEC-G1: `Action::Destructive`, default policy admits only `svc=true` or `admin`) precisely because `subject_of` is the family's highest-sensitivity v1 edge kind (`agents/share/cross-service-linking.md` §10). |
| service | FHIR R5 API | `Task` resource (best-effort/`low`-fidelity mapping — a governmental case has no exact FHIR analog), mounted at `/fhir/Task{,/{id}}` + `GET /fhir/metadata` (`src/fhir/{mod,resources,search}.rs`, `src/controllers/fhir.rs`). Read/create/update/delete/search reuse the native model helpers, validators, and event/audit paths. **Record-level ABAC + the `mask` obligation apply on FHIR read and search** (SEC-G2, closing what an earlier version of this file listed as omitted) — a denied caller gets `403`, a `mask`-obligation allow returns a redacted `Task`. Supported search params: `_id`, `_lastUpdated` (accepted, ignored), `_count`, `identifier`, `status`, `priority`. Documented lossy gaps: `alternate_titles`/`opened_date`/`keywords`/`same_as`/`in_language` and 2nd+ `subjects` entries are not carried; some status/priority values collide on the FHIR side. |
| service | Compliance suite | `src/compliance/` (landed 2026-07-25..27; also omitted from an earlier version of this file): tamper-evident **audit hash chain** (`audit_chain.rs`) with external-witness **checkpoints** (`checkpoint.rs`); **read/disclosure auditing** (`disclosure.rs`, feeding `GET /{pid}/audit/disclosures`, HIPAA §164.528 accounting); row-level **record integrity** (`record_integrity.rs`, `content_hash`, verified at `GET /records/verify`); **GDPR Art. 17 erasure** (`erasure.rs`, `POST /{pid}/erase`); a keyed **HMAC-SHA256 integrity MAC** (`mac.rs`, default off — no key configured ⇒ no MAC written); a **CycloneDX SBOM + SOUP register** (`soup.rs`, IEC 62304 §5.3.3/§8.1.2), served at `GET /api/compliance` (service identification + build provenance) and `GET /api/compliance/sbom`. |
| service | Bulk import/export | `src/bulk/` (BLK-5, **landed 2026-08-03**; also omitted from an earlier version of this file): async, job-based `POST`/`GET /api/cases/import{,/​{id}}` and `.../export{,/​{id}}` plus `GET /api/cases/bulk-jobs`, backed by a loco `BackgroundWorker`. Formats: **JSONL and CSV only — no Parquet.** Artifact storage: **local filesystem only — no S3 backend** (`ArtifactStore` is async so S3 is additive later, not built in this rollout). Stable key for idempotent upsert: the agency-scoped pair `(agency_id, case_number)`, then an explicit `pid`, then keyless rows routed through the existing duplicate-check → `review_queue` (new table, `provenance` column from day one — case had no review queue before this). Export reuses `mask_case` as the default `masked` profile; `full` export requires `Action::Destructive` authorisation; every export is audited and the audit write **gates delivery** (SEC-B8). **Documented, not built:** true concurrent-importer race safety beyond sequential idempotency (SEC-B3 — no advisory-lock hook in the current `create_and_emit`/`update_and_emit` shape) and per-row record-level ABAC *inside the async export worker* (no live bearer token to evaluate against there; the same gap the person reference implementation has). |
| service | Authorization (ABAC) | **Case is the family's reference implementation** for `agents/share/authorization-attributes.md`, not just a consumer of it. Blanket `/api/*` guard (`CASE_REQUIRE_AUTH`, default off) plus the shared policy engine (`authentication-verifier`), landed 2026-07-05, evaluated over the token's `attrs` claim (first-match-wins, default allow-read/deny-mutation); hot-reloadable policy (`CASE_ABAC_POLICY[_FILE]`, file-mtime watcher, no restart needed) and a hot-reloadable PASETO key set (`CASE_PASETO_KEYS_URL`, periodic refresh) both landed the same day. **Record-level** authorization — the deeper, per-record pass beyond the coarse guard — derives `resource.case_type`/`resource.status`/`resource.priority` from the loaded case (`auth::case_resource_attrs`) plus request-time `env.hour`/`env.after_hours` (`auth::request_env_attrs`), and evaluates them with `$sub`/`$email` ownership templates and **mask-on-allow obligations** (`auth::authorize_record`) — so a policy can express e.g. "deny write when `resource.status=closed` unless `access=admin`" or grant a masked cross-department read, as configuration rather than code. This record-level pass runs on `GET`/`PUT`/`DELETE /api/cases/{pid}`, `list`/`search`/`check-duplicates` (as concealment — SEC-G3), and FHIR `read`/`search` (SEC-G2) alike. |
| service | Token verification | Offline bearer-token verification against the auth-service's published key (`src/auth.rs`, embeds `authentication-verifier`); `AuthUser`/`MaybeAuthUser`; `/whoami` protected; audit/merge `actor` stamped from the token. **The credential is, and has been since the family's auth pivot, PASETO v4 public (Ed25519)** — never re-litigate this against an older sentence in this file that said otherwise; see [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md) (source of truth; supersedes RS256-JWT + JWKS). The key set is fetched over HTTP at boot **and refreshed periodically** (`CASE_PASETO_KEYS_URL` + `CASE_PASETO_KEYS_REFRESH_SECS`, default hourly, landed 2026-07-04/07-05) — env-injected keys (`CASE_PASETO_KEYS`) remain the fallback if the URL is unset or a fetch fails. |
| service | API docs | OpenAPI 3 (`src/openapi.rs`, hand-written) + Swagger UI at `/api-docs/openapi.json` · `/swagger-ui` |
| service | Observability | Prometheus metrics at `/metrics.prom` (root-mounted, public even under enforcement): four CRUD counters + an `http_requests_total` label vec |
| service | Tests | DB-free unit/property/fuzz tests + module unit tests (validation, merge, streaming, auth crypto, openapi, search, bulk codecs); request-level loco tests (`#[ignore]`-gated on Postgres, **enrolled in CI's `test-db` stage** — no longer just local-only); Criterion benches (`benches/service_bench.rs`); green build + clippy |
| front-end | Routes | `/`, `/new`, `/[pid]` (detail + delete + check-duplicates), `/[pid]/edit` |
| front-end | API layer | Lean raw-JSON client, `CaseRepository`, hand-mirrored TS types |
| front-end | Form | Full-DTO editing incl. type/status/priority selects, date input, identifier row editor, comma-list fields |
| front-end | Quality bar | `pnpm run check` strict 0/0; production build green |
| front-end | Tests | vitest units (client + repository, `check-duplicates` regression) + Playwright smoke (4 routes, API-stubbed, runs on `vite preview`) |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| No front-end search box, audit view, or event view (service endpoints exist ahead of the UI); no front-end merge action | T-11 |
| No subject-scoped GDPR export spanning every case sharing one `subjects` id (today's export is per-case only) | T-10 follow-up (§13) |
| Bulk import: no true concurrent-importer race safety beyond sequential idempotency (SEC-B3); bulk export: no per-row record-level ABAC inside the async worker; no S3 artifact-store backend; no Parquet | crate spec §16 |
| Durable event bus: `FluvioSink` is wired and feature-gated-tested but no deployment points `CASE_FLUVIO_ENDPOINT` at a live broker — the live round trip is `#[ignore]`d, not exercised by CI | T-12 follow-up (§13) |
| FHIR: several fields are lossy in the `Case ↔ Task` mapping (`alternate_titles`, `opened_date`, `keywords`, `same_as`, `in_language`, 2nd+ `subjects`); `CarePlan` mapping is roadmap only | crate spec §9/§13 |
| No deeper validation (docket / case-number format, status transitions) | T-9 follow-up |
| No real-time duplicate detection on create (`409`) | (roadmap §15; OQ-4) |
| No localization of the operator UI | (roadmap §15; no task yet) |
| Blanket `/api/*` enforcement (`CASE_REQUIRE_AUTH`) and the ABAC policy are both implemented but **default off** — activation is a deployment decision, not a code gap (`agents/share/security.md` §4); the shipped default is open. | operational, not a `spec/13` task |

### 14.3 Corrected in this pass (2026-08 professionalization audit)

For a future reader wondering why this file looks different from an
older revision: it was not always accurate. Two things were wrong and
are now fixed, not merely silently rewritten:

- **Self-contradiction on the auth credential.** An earlier revision
  stated in one place that the credential was PASETO v4 public, and a
  few lines later that it "is not yet switched to PASETO v4 public" —
  both referring to the same running code. Checked directly against
  `src/auth.rs` (which verifies `v4.public` Ed25519 tokens via
  `authentication-verifier`, with `CASE_PASETO_KEYS_URL` boot-fetch and
  periodic refresh) and the crate-level spec (§13 T-7, §14): PASETO has
  been the credential since the family's RS256→PASETO pivot; the
  "not yet" sentence was stale drift, not a live gap.
- **Omitted capabilities.** This file's "Delivered" table did not
  mention `entity_links` (cross-service `subject_of` links, case is the
  family's first originating implementation), the FHIR R5 surface, the
  compliance suite (audit chain, disclosure/read auditing, record
  integrity, erasure, checkpoints, MAC, SBOM), bulk import/export, or
  record-level ABAC (for which case is the family's reference
  implementation) — all real, all shipped, none of them new as of this
  edit. They are added above, each grounded against the source file(s)
  that implement it rather than against the earlier prose describing
  it.
