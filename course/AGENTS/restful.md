# RESTful API — Course Entity

Orientation only. The complete endpoint reference (parameters,
envelopes, status codes) is
[`course-service/AGENTS/restful.md`](../course-service-with-loco/AGENTS/restful.md);
the normative summary is entity spec [§9](../spec/09-api-surface.md).

## The one wire contract

- Base path: **`/api`** (not `/api/v1` — that's the event entity).
  Default port **8084**. The front-end's `CourseRepository`
  ([`src/lib/api/courses.ts`](../course-front-end-with-svelte/src/lib/api/courses.ts))
  hard-assumes this.
- Envelope everywhere:
  `{ "success": bool, "data": …, "error": { code, message, details } }`.
- Search returns `{ items, total }` (service FR-19 — the front-end
  depends on `items`).
- Status idioms: `201` create, `409` + `MatchResult[]` duplicate on
  create, `422` validation, `204` delete, `501` only for
  `GET /api/courses` list-all (use `search` with empty `q`).

## Surface map

| Group | Endpoints |
|---|---|
| Health | `GET /api/health`; loco `/_health`, `/_ping` |
| Course CRUD | `POST/GET/PUT/DELETE /api/courses[/{id}]` |
| Search | `GET /api/courses/search` (`q`, `limit`, `offset`, `fuzzy`, `phonetic`, `educational_level`, `language`, `provider_id`, `mask_sensitive`) |
| Match / dedup | `POST /api/courses/{match,check-duplicates,merge,deduplicate}` |
| Instances | `GET/POST /api/courses/{id}/instances`, `GET/PUT/DELETE /api/courses/{id}/instances/{instance_id}` |
| Privacy | `GET /api/courses/{id}/{masked,export}` |
| Audit | `GET /api/courses/{id}/audit`, `GET /api/audit/recent` |
| OpenAPI | `/swagger-ui`, `/api-docs/openapi.json` |

## Front-end routes consuming it

`/` (dashboard), `/courses` (grid), `/courses/new` (409-aware
create), `/courses/[id]` (+ `/edit`, `/audit`), `/courses/match`,
`/courses/merge`. See the front-end
[README](../course-front-end-with-svelte/README.md) and
[spec §5](../course-front-end-with-svelte/spec/05-information-architecture.md).

## Change rules

- The service owns the wire format. Any change ships with a
  front-end `types.ts` + test edit in the same change cycle, and an
  entity spec §9 edit if the surface shape moved.
- Handlers are idiomatic loco controllers (registered in
  `App::routes` with the `/api` prefix); this service is the family
  reference for that pattern — keep new endpoints in the same shape.
- Auth: none today; offline PASETO v4.public verification (published
  Ed25519 key) against the
  [authentication entity](../../authentication/) is the planned
  family-wide rollout (entity spec §13 T-7). See
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  for the cookie-session + PASETO model.
