## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `person-service-with-loco/AGENTS/models.md`.
- [x] T-3: `ApiClient` + `PersonRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `PersonRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-12a: Playwright **integration** suite (`tests/integration/golden-paths.spec.ts`) driving the live preview against a running `person-service-with-loco`. 9 tests covering FR-1, FR-3 (×2 — happy path + 409 duplicate), FR-5, FR-6, FR-7, FR-8, FR-9, and per-record audit. Idempotent (timestamped family names + REST `DELETE` cleanup). Run with `bin/e2e` or `pnpm test:integration`. Harness is validated (svelte-check clean, playwright `--list` discovers all 9 tests, smoke project still 6/6, bin/e2e exits 1 with a clear message when the service is down). **End-to-end validation against a live service is blocked on a pre-existing issue in the service crate — see OQ-5.**
- [ ] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Identifier / address / emergency-contact edit (currently read-only on detail; edit form re-PUTs whole record but no UI to add/remove sub-records).
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [ ] T-18: Batch deduplicate-scan results UI.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-23 (repo FE-2): Cross-service **links panel** on `/persons/[id]` — list this person's active outbound edges, assert a new one (`same_identity` → worker, `works_at` / `member_of` → organization), and withdraw one behind a confirm. `LinksPanel.svelte` + `EntityLink` / `CreateLinkRequest` types + `listLinks` / `createLink` / `deleteLink` on the repository + the pure kind↔target-type rules in `src/lib/links.ts` (mirroring the service's `validate_edge`, so a wrong target type is caught before the request). Server `422` reasons are surfaced inline. Deliberately distinct from the `Person.links` merge relationship (§9). Tests: `tests/unit/links-validation.test.ts` (12), three repository tests + a 422-surfacing test, an i18n-parity extension for 26 new keys across all 13 locales, and a route-stubbed Playwright smoke assertion.
- [x] T-24 (repo FE-3): **Bulk import/export screen** at `/persons/bulk` — upload a JSONL/CSV file with a dry-run toggle, submit a filtered export with a masking profile, poll each `202`-accepted job to a terminal state, and list recent jobs with client-side kind/status filters (the `bulk-jobs` endpoint takes only `limit` — no server-side filtering). `ApiClient` gained `FormData` pass-through (no JSON serialization, and the forced `content-type` is stripped so `fetch` sets the multipart boundary); repository gained `importPersons` / `exportPersons` / `getImportJob` / `getExportJob` / `listBulkJobs`; `src/lib/bulk.ts` holds the pure rules (terminal-state set, the dry-run token encoding matching the service's `1|true|yes|on`, progress clamping, and the import-format set excluding export-only Parquet). Each submit sends a fresh `Idempotency-Key` (SEC-B9). Scope decisions, all forced by the service rather than chosen here: `download_url` / `errors_url` are **rendered as plain text, not links** — they are opaque artifact-store references (`file://…` / `s3://…`) and the service exposes no endpoint serving their bytes (see §16 OQ-7); `include_soft_deleted` is **not offered** because the endpoint accepts it but the worker rejects it; a `404` on a status poll stops the loop and reports expired-or-gone, since the service returns `404` both for a job past its retention TTL and for another actor's job. Tests: `tests/unit/bulk.test.ts` (14 — pure rules, the `FormData`-not-JSON body with its header contract, the JSON-body regression pin, and the 404 path), an i18n-parity extension for 69 new keys across all 13 locales, and a route-stubbed Playwright smoke assertion.
- [ ] T-22: Auth — adopt BFF + httpOnly cookie + CSRF; the browser holds only `__Host-mxi_session`, the SvelteKit server attaches a short-lived PASETO server-side; no `mxi_access_token`/`localStorage` bearer, no fragment handoff (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).

