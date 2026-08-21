## 9. API Surface

The entity exposes one public machine surface — the service's REST
API under `/api` (note: **not** `/api`; the event entity uses the
versioned prefix, course does not) — and one human surface, the
front-end routes. Complete endpoint reference:
[`course-service-with-loco/agents/restful.md`](../course-service-with-loco/agents/restful.md);
entity-level orientation: [`agents/restful.md`](../agents/restful.md).

### 9.1 REST API (service, :8084)

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/health` | Health check (plus loco `/_health`, `/_ping`) |
| POST | `/api/courses` | Create; `409` + `MatchResult[]` on duplicate; `422` on validation |
| GET | `/api/courses/{id}` | Get one course (includes nested `instances`) |
| PUT | `/api/courses/{id}` | Replace (excludes `instances`) |
| DELETE | `/api/courses/{id}` | Soft-delete |
| GET | `/api/courses/search` | Full-text / fuzzy search; `{ items, total }` |
| POST | `/api/courses/match` | Score a request course against the index |
| POST | `/api/courses/check-duplicates` | Duplicate check without writing |
| POST | `/api/courses/merge` | Merge duplicate into main |
| POST | `/api/courses/deduplicate` | Batch dedup scan with review queue + auto-merge |
| GET | `/api/courses/{id}/instances` | List instances (`start_date DESC NULLS LAST`) |
| POST | `/api/courses/{id}/instances` | Create instance |
| GET | `/api/courses/{id}/instances/{instance_id}` | Get one instance |
| PUT | `/api/courses/{id}/instances/{instance_id}` | Replace instance |
| DELETE | `/api/courses/{id}/instances/{instance_id}` | Soft-delete instance |
| GET | `/api/courses/{id}/masked` | Masked view (provider / instructor fields) |
| GET | `/api/courses/{id}/export` | GDPR Article 15 export |
| GET | `/api/courses/{id}/audit` | Audit log for one course + children |
| GET | `/api/audit/recent` | Recent audit activity |

All responses use the shared envelope
`{ "success": bool, "data": …, "error": … }`. OpenAPI: Swagger UI at
`/swagger-ui`, raw JSON at `/api-docs/openapi.json`. `GET
/api/courses` (list-all-without-search) is intentionally `501` — use
`search` with an empty `q`.

### 9.2 Front-end routes (operator UI, :5173)

| Route | Purpose |
|---|---|
| `/` | Dashboard — service health + recent audit activity |
| `/courses` | List & search (SVAR DataGrid) |
| `/courses/new` | Create; surfaces `409` duplicate candidates inline |
| `/courses/[id]` | Detail view (instances currently read-only) |
| `/courses/[id]/edit` | Edit |
| `/courses/[id]/audit` | Per-course audit log |
| `/courses/match` | Match check — score a hypothetical record |
| `/courses/merge` | Merge two courses (main + duplicate) |

The front-end reads the base URL from `PUBLIC_API_BASE_URL`
(default `http://localhost:8084`).

### 9.3 Contract rules

- The service owns the wire format; the front-end mirrors it in
  `src/lib/api/types.ts` and MUST be updated in the same change
  cycle as any service wire change (§5.4).
- gRPC is out of MVP scope (stub only); the matcher has **no**
  network surface — it is consumed only as a Rust library
  (`MatchingEngine::match_courses`, `match_one_to_many`; matcher
  [spec §20](../course-matcher-rust-crate/spec/20-consumption.md)).
