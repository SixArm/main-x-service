## 9. API Consumption

The front-end binds 1:1 to the Person Service REST surface (see [`person-service-with-loco/agents/restful.md`](../../person-service-with-loco/agents/restful.md)):

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/persons/search` | `/persons` list |
| `GET /api/persons/{id}` | `/persons/[id]`, `/persons/[id]/edit`, merge preview |
| `POST /api/persons` | `/persons/new` |
| `PUT /api/persons/{id}` | `/persons/[id]/edit` |
| `DELETE /api/persons/{id}` | Detail page (soft-delete button) |
| `POST /api/persons/match` | `/persons/match` |
| `POST /api/persons/check-duplicates` | (available — not yet routed) |
| `POST /api/persons/merge` | `/persons/merge` |
| `POST /api/persons/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/persons/{id}/audit` | `/persons/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/persons/{id}/masked` | (available — not yet routed) |
| `GET /api/persons/{id}/export` | (available — not yet routed) |
| `GET /api/persons/{id}/links` | `LinksPanel` on `/persons/[id]` |
| `POST /api/persons/{id}/links` | `LinksPanel` — assert an outbound edge |
| `DELETE /api/persons/{id}/links/{linkId}` | `LinksPanel` — withdraw an edge |
| `POST /api/persons/import` | `/persons/bulk` — submit an import job (`FormData`: file + format + dry_run) |
| `GET /api/persons/import/{id}` | `/persons/bulk` — poll an import job to a terminal state |
| `POST /api/persons/export` | `/persons/bulk` — submit an export job (format + filter + masking profile) |
| `GET /api/persons/export/{id}` | `/persons/bulk` — poll an export job to a terminal state |
| `GET /api/persons/bulk-jobs` | `/persons/bulk` — recent-jobs table (client-filtered by kind/status) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `PersonRepository` return unwrapped `data`.

### Cross-service links

The three `…/links` endpoints are the **write side** of
[`cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
§4.1. They are distinct from the `links` field on the `Person` payload,
which is the within-entity person→person merge relationship and a matcher
signal; cross-service edges are never a matcher signal (§7), so the two
surface in separate sections with separate types (`EntityLink` vs
`PersonLink`).

Person may originate exactly three edge kinds, each with a required
target entity type — enforced server-side by `validate_edge` and
mirrored client-side in `src/lib/links.ts` so the constraint is visible
in the form rather than learned from a `422`:

| Kind | Target ref | Meaning |
| --- | --- | --- |
| `same_identity` | `worker:<uuid>` | the same human in the workforce registry |
| `works_at` | `organization:<uuid>` | affiliation (temporal) |
| `member_of` | `organization:<uuid>` | membership (temporal) |

`POST` is an idempotent upsert on `(kind, to_ref, valid_from)`, so
re-asserting an edge refreshes it rather than duplicating it. `DELETE`
is a soft-delete (the edge is withdrawn, not erased) and answers `200`
with an empty payload rather than `204`. The server's `422` reason
string is human-readable and is surfaced inline.

### Bulk import/export

The five `.../import`, `.../export`, `.../bulk-jobs` endpoints are the
front-end's binding to
[`bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
Both submit endpoints answer `202` with a job id; the front-end polls
the matching `GET` until the job reaches a terminal status
(`completed` / `completed_with_errors` / `failed` — an unrecognised
status is treated as **not** terminal, per `src/lib/bulk.ts`, so a
newer service vocabulary cannot freeze the poll). `download_url` /
`errors_url` on a finished job are opaque artifact-store references
(`file://…` / `s3://…`) with no serving endpoint, so they render as
plain text rather than a link (§16 OQ-7).

