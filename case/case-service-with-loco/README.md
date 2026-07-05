# Case Service

A registry of **governmental case** records: CRUD + matching, built
on **loco.rs** and embedding the canonical
[case-matcher](../case-matcher-rust-crate).

A *case* (case management / case tracking) is an open or historical
matter handled by a public agency on behalf of one or more subjects — a
benefit claim, legal action, social-services referral, licensing
application, complaint, appeal, and so on.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [case-front-end-with-svelte](../case-front-end-with-svelte)

## API

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/cases` | Create |
| GET | `/api/cases` | List |
| GET | `/api/cases/search?q=` | Case-insensitive title search |
| GET | `/api/cases/{pid}` | Fetch |
| PUT | `/api/cases/{pid}` | Update |
| DELETE | `/api/cases/{pid}` | Soft-delete |
| POST | `/api/cases/match` | Rank `{query, candidates}` |
| POST | `/api/cases/check-duplicates` | Match query vs stored cases |
| POST | `/api/cases/merge` | Merge a duplicate into a survivor |
| GET | `/api/cases/whoami` | Verified PASETO-token claims |

The body for a case **is** the `case_matcher::Case` shape (title,
alternate titles, case number + agency, case type, status, priority,
opened date, subjects, keywords, identifiers, sameAs, languages).

## Quick start

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/case_service_development
cargo loco start        # migrations auto-run in development

curl -s localhost:5150/api/cases -H 'content-type: application/json' \
  -d '{"title":"Housing benefit appeal","agency_id":"dwp","case_number":"HB-2024-0007","identifiers":[{"scheme":"Docket","value":"CV-2024-001234"}]}'
```

## Testing

```bash
cargo test                   # DB-free: matcher embedding, JSON round-trip,
                             # validation / merge / auth / openapi / streaming
cargo test -- --ignored      # request-level tests (need Postgres DATABASE_URL)
cargo clippy --all-targets
```

Validation failures — blank `title`, malformed `opened_date`, blank
identifier value, or blank `subjects` / `keywords` entry — return
`422 Unprocessable Entity`, the family convention.

## Status

MVP: CRUD + `ILIKE` title search + matching, with validation, OpenAPI 3
+ Swagger UI, an audit log + in-memory event stream, record merge, and
offline PASETO v4.public verification (published Ed25519 key). Tantivy
full-text search, durable event bus, and privacy are tracked in
[spec §13](./spec/index.md). Auth credentials are issued by the central
[authentication-service](../../authentication/authentication-service-with-loco).

> Auth pivot in progress: the family moved from RS256 JWT + JWKS to
> cookie sessions + offline PASETO v4.public verification — see
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> (source of truth; RS256/JWKS decommissioned). The runtime here
> verifies PASETO v4.public via `authentication-verifier`, with the
> published key set fetched over HTTP once at boot when
> `CASE_PASETO_KEYS_URL` is set (fetched key set wins; falls back to the
> `CASE_PASETO_KEYS` env key set, so the service always boots) — see
> [spec §13](./spec/index.md).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
