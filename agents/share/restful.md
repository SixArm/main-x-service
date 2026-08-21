### RESTful guidance

- OpenAPI 3.0 specification
- Interactive Swagger UI
- JSON request/response format
- CORS support for web applications
- Comprehensive error handling
- HTTP status codes following REST conventions

## Pagination

Collection reads take **`?limit=` and `?offset=`**, and report the total
in **response headers** — the body shape does not change.

| Header | Meaning |
|---|---|
| `X-Total-Count` | Rows matching the query, **ignoring** `limit`/`offset` |
| `X-Limit` | The limit actually applied (after clamping) |
| `X-Offset` | The offset actually applied |

```
GET /api/organizations?limit=25&offset=50
→ 200, body: [ …25 refs… ]
   X-Total-Count: 431
   X-Limit: 25
   X-Offset: 50
```

Rules, uniform across services:

- **Defaults preserve the previous behaviour.** Omitting both parameters
  returns what the endpoint returned before pagination existed (its old
  hard cap, from offset 0), so no existing client changes.
- **`limit` is clamped, not rejected**, to a per-endpoint maximum
  (`MAX_LIMIT`, 500 for list/search surfaces). A caller asking for
  100 000 rows gets the maximum and an `X-Limit` saying so, rather than a
  `400` it has to learn to handle.
- **`offset` is bounded** and a request beyond the bound is a `400`
  (SEC-G7): an unbounded offset makes the database materialise
  arbitrarily many rows to discard them, which is a cheap denial of
  service. Deep paging past the bound wants a cursor, not a bigger
  number.
- **Zero or unparseable values fall back to the default** rather than
  erroring; `limit=0` would otherwise be an easy way to get an empty page
  that looks like an empty collection.

### Why headers rather than an envelope

Every one of these endpoints already returns a bare JSON array, and the
front-ends parse it as one. Wrapping the array in
`{ "items": [...], "total": n }` would break every existing caller for a
number most of them do not use, so the count goes where it can be added
without changing what is already there. A future API version may adopt
an envelope; that is a version's job, not a patch's
([api-versioning.md](api-versioning.md)).

Cursor pagination is deliberately not offered yet: it is the right answer
for deep paging and for stable ordering under concurrent writes, and it
should be designed once, family-wide, rather than invented per service.
