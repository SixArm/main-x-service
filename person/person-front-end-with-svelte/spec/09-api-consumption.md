## 9. API Consumption

The front-end binds 1:1 to the Person Service REST surface (see [`person-service-with-loco/AGENTS/restful.md`](../../person-service-with-loco/AGENTS/restful.md)):

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

