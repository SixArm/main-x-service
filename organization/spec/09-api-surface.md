## 9. API Surface

Source of truth for routes:
[`src/controllers/organizations.rs`](../organization-service-with-loco/src/controllers/organizations.rs)
and [`docs.rs`](../organization-service-with-loco/src/controllers/docs.rs);
machine-readable form at `/api-docs/openapi.json`
([`src/openapi.rs`](../organization-service-with-loco/src/openapi.rs)).

### 9.1 REST endpoints (service, default port 5150)

| Method | Path | Request body | Response | Notes |
|---|---|---|---|---|
| POST | `/api/organizations` | `Organization` | `{pid, name}` | `422` when `name` is blank |
| GET | `/api/organizations` | — | `[{pid, name}]` | Active rows, newest first, capped 100 |
| GET | `/api/organizations/search?q=` | — | `[{pid, name}]` | `ILIKE %q%` on name, capped 50; blank `q` → `400` |
| GET | `/api/organizations/{pid}` | — | `Organization` | The stored payload; `404` unknown / deleted pid |
| PUT | `/api/organizations/{pid}` | `Organization` | `{pid, name}` | Replaces the whole payload; `422` blank `name`, `404` unknown pid |
| DELETE | `/api/organizations/{pid}` | — | `{}` | Soft delete (`deleted_at` stamped) |
| POST | `/api/organizations/match` | `{query, candidates}` | ranked `[(index, MatchResult)]` | Pure scoring; no persistence |
| POST | `/api/organizations/check-duplicates` | `Organization` | `[{pid, name, score, confidence, is_match}]` | Stored matches above threshold, score-desc |
| POST | `/api/organizations/merge` | `{main_pid, duplicate_pid, reason?}` | `{main_pid, duplicate_pid, main}` | Fold a duplicate into a survivor; `422` equal pids, `404` unknown |
| GET | `/api/organizations/merges/recent` | — | `[merge row]` | Merge history (incl. transferred snapshot) |
| GET | `/api/organizations/whoami` | — | `Claims` | Verified bearer-token claims; `401` without a valid token |
| GET | `/api/organizations/audit/recent` | — | `[audit row]` | Newest 100 system-wide |
| GET | `/api/organizations/{pid}/audit` | — | `[audit row]` | Per-record trail; `400` invalid pid |
| GET | `/api/organizations/events/recent` | — | `[OrgEvent]` | Newest 100 from the in-memory stream |
| POST | `/api/v1/organizations/import` | format, dedupe_mode, dry_run + file | `202 {job_id}` | Async bulk import (§8.7; roadmap §13). Not yet implemented |
| GET | `/api/v1/organizations/import/{id}` | — | job status + counts + errors_url + review_url | Bulk import job status. Not yet implemented |
| POST | `/api/v1/organizations/export` | format, filter, fields, include_soft_deleted, masking_profile | `202 {job_id}` | Async bulk export (§8.7). Not yet implemented |
| GET | `/api/v1/organizations/export/{id}` | — | job status + download_url | Bulk export job status. Not yet implemented |
| GET | `/api/v1/organizations/bulk-jobs` | — | `[bulk_job]` | List/filter bulk jobs; `.../{id}` for one. Not yet implemented |
| GET | `/api-docs/openapi.json` | — | OpenAPI 3 document | Hand-written |
| GET | `/swagger-ui` | — | HTML | Interactive docs |
| GET | `/_health`, `/_ping` | — | — | loco defaults |

### 9.2 Conventions

- **Raw loco JSON — no envelope.** Unlike the older Axum-native
  entities (person et al. wrap responses in
  `{success, data, error}`), this service returns bodies directly.
  The front-end client is written for this.
- Wire field names are snake_case (`legal_name`, `founding_date`, …)
  per the canonical DTO (§5.1).
- Status codes: `200` success, `422` validation failure (blank
  `name` on create/replace — family convention), `400` malformed
  request (blank `q`, invalid pid), `404` not found, `500` internal.
  Code, OpenAPI, crate spec, and this section agree (T-2, resolved
  2026-06-13).
- No authentication yet (§13); `actor` in audit rows is `null`.

### 9.3 Front-end route surface

| Route | Purpose | Endpoints consumed |
|---|---|---|
| `/` | List organizations | `GET /api/organizations` |
| `/new` | Create form → redirect to detail | `POST /api/organizations` |
| `/[pid]` | Detail + delete + check-duplicates | `GET` / `DELETE /api/organizations/{pid}`, `POST /api/organizations/check-duplicates` |
| `/[pid]/edit` | Edit form → redirect to detail | `PUT /api/organizations/{pid}` |

The front-end does not yet consume `/search`, the audit endpoints, or
the event stream (its spec §13).
