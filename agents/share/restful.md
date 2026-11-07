### RESTful guidance

- OpenAPI 3.0 specification
- Interactive Swagger UI
- JSON request/response format
- CORS support for web applications
- Comprehensive error handling
- HTTP status codes following REST conventions

## Pagination

Collection reads page by **signed cursor**, through
`axa::cursor_pagination` (axa's `cursor-pagination` feature). A page is
**`?limit=`** and an opaque **`?cursor=`**. While more rows follow, the
response carries **`Link: <…>; rel="next"`**, and the last page carries
none. The body stays a bare JSON array, and there is **no total**.

```
GET /api/organizations?limit=25
→ 200, body: [ …25 refs… ]
   Link: </api/organizations?limit=25&cursor=AQAAAAAAAAAZ…>; rel="next"
```

Rules, uniform across the services that page this way:

- **Follow the link.** A client fetches the `Link` URL as it is. It may
  change `limit` between pages, and nothing else.
- **The cursor is the service's.** It is signed with HMAC-SHA256 under
  the service's `CURSOR_KEY`, over its position, the path and every
  query parameter but `limit` and `cursor`. So a caller cannot make one
  up, edit one, or carry one to another list or filter. Such a cursor is
  a `422`.
- **`limit` is refused, not clamped.** No `limit` means the endpoint's
  old default page. A `limit` of zero, not a number, or above
  `MAX_LIMIT` (500 for list and search) is a `422`, so a caller is never
  handed less than it asked for without being told.
- **No total.** A count would include rows the caller's record policy
  hides, which tells it they exist. A client that needs to know whether
  it has everything reads whether a `next` link came.
- **No offset bound.** A position comes only from a cursor the service
  signed, so reaching a deep page means fetching every page before it.
  The old `offset` cap (SEC-G7) has nothing left to bound, and the
  request limits in [security.md](security.md) §4 cover volume.
- **Order ends in a unique column.** The position is an offset under
  the signature, so a row added or removed between fetches can make a
  page skip or repeat one row. A stable order keeps that to one row.
- **A search decides "more" from the index.** Where hits resolve to
  rows afterwards, a hit whose row was deleted is dropped from the page;
  the `next` link is decided from the hits first, so a short page does
  not end the walk.

`CURSOR_KEY` is at least 32 bytes and the same on every replica
([configuration.md](configuration.md)). Production refuses to start
without it; elsewhere a missing key is a random one for that process.

### Where it applies

organization, care-pathway, case, place and thing page this way,
since 2026-11-07. person, worker, event and course still take a bounded
`?offset=` through their own code, with no total header; moving them is
an open task in repo [`tasks.md`](../../tasks.md).

### Why no envelope

Every one of these endpoints already returned a bare JSON array, and the
front-ends parse it as one, so the next page goes in a header, where it
can be added without changing what is already there. A future API
version may adopt an envelope; that is a version's job, not a patch's
([api-versioning.md](api-versioning.md)).
