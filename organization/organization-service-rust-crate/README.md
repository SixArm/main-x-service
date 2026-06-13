# Organization Service

A registry of **organization identities**
([schema.org/Organization](https://schema.org/Organization)): CRUD +
matching, built on **loco.rs** and embedding the canonical
[organization-matcher](../organization-matcher-rust-crate).

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [organization-front-end-with-svelte](../organization-front-end-with-svelte)

## API

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/organizations` | Create |
| GET | `/api/organizations` | List |
| GET | `/api/organizations/{pid}` | Fetch |
| PUT | `/api/organizations/{pid}` | Update |
| DELETE | `/api/organizations/{pid}` | Soft-delete |
| POST | `/api/organizations/match` | Rank `{query, candidates}` |
| POST | `/api/organizations/check-duplicates` | Match query vs stored orgs |

The request/response body for an organization **is** the
`organization_matcher::Organization` shape (name, legalName,
alternateName, identifiers, url, sameAs, address, jurisdiction,
foundingDate, keywords).

## Quick start

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/organization-service_development
cargo loco start        # migrations auto-run in development

curl -s localhost:5150/api/organizations -H 'content-type: application/json' \
  -d '{"name":"Acme, Inc.","jurisdiction":"US","url":"https://acme.com"}'
```

## Testing

```bash
cargo test --test matching   # DB-free: matcher embedding + JSON round-trip
cargo clippy --all-targets
```

Request-level tests require a Postgres instance (standard loco).

## Status

MVP: CRUD + matching. Tantivy search, streaming, audit, privacy/GDPR,
OpenAPI, and richer validation are tracked in [spec §13](./spec/index.md).
JWT auth is provided by the central
[authentication-service](../authentication-service-rust-crate).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
