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
`organization_matcher::Organization` shape, serialized snake_case
(`name`, `legal_name`, `alternate_names`, `identifiers`, `url`,
`same_as`, `address`, `jurisdiction`, `founding_date`, `keywords`).
schema.org publishes the camelCase property names (`legalName`,
`sameAs`, …); the wire format here is the Rust DTO's snake_case
(entity spec OQ-1, resolved).

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
cargo test                   # DB-free: unit + matcher embedding + JSON round-trip
cargo test -- --ignored      # request-level suite; needs Postgres (config/test.yaml)
cargo clippy --all-targets
```

The request-level tests (`tests/requests/organizations.rs`) boot the
real app against `config/test.yaml` and are `#[ignore]`d so the
default run stays green without a database. Validation failures
(blank `name`) return `422 Unprocessable Entity`.

## Status

MVP: CRUD + matching. Tantivy search, streaming, audit, privacy/GDPR,
OpenAPI, and richer validation are tracked in [spec §13](./spec/index.md).
JWT auth is provided by the central
[authentication-service](../../authentication/authentication-service-rust-crate).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
