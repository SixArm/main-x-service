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
| GET | `/api/organizations/search?q=` | Case-insensitive name search (`ILIKE`) |
| POST | `/api/organizations/merge` | Fold a duplicate into a survivor |
| GET | `/api/organizations/merges/recent` | Merge-history records |
| GET | `/api/organizations/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api/organizations/audit/recent` · `/{pid}/audit` | Audit trail |
| GET | `/api/organizations/events/recent` | In-memory event stream (`EventView`) |
| GET | `/swagger-ui` · `/api-docs/openapi.json` | API docs |
| GET | `/metrics.prom` | Prometheus metrics (root path, public) |

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
export DATABASE_URL=postgres://loco:loco@localhost:5432/organization_service_development
cargo loco start        # migrations auto-run in development

curl -s localhost:5150/api/organizations -H 'content-type: application/json' \
  -d '{"name":"Acme, Inc.","jurisdiction":"US","url":"https://acme.com"}'
# -> {"pid":"<uuid>","name":"Acme, Inc."}

# Fold a confirmed duplicate into a survivor.
curl -s localhost:5150/api/organizations/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<dup-uuid>","reason":"confirmed"}'
# -> {"main_pid":"...","duplicate_pid":"...","main":{<merged Organization>}}

# Inspect the verified bearer-token claims (401 without a token).
curl -s localhost:5150/api/organizations/whoami -H 'authorization: Bearer <paseto>'

# Scrape Prometheus metrics (root path, public even under blanket auth).
curl -s localhost:5150/metrics.prom
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

Done: CRUD + matching + name search (`ILIKE`) + audit log + event
streaming + record merge + OpenAPI/Swagger + Prometheus metrics
(`/metrics.prom`) + offline PASETO v4.public verification (blanket
`/api/*` enforcement is wired, default-off; the published key set is
fetched over HTTP once at boot when `ORGANIZATION_PASETO_KEYS_URL` is
set, with warn + env fallback). Still deferred (see
[spec §13](./spec/index.md)): Tantivy full-text search, per-field
privacy/GDPR export, a key-set refresh loop (the boot fetch runs once),
and richer validation. Auth is provided by the central
[authentication-service](../../authentication/authentication-service-with-loco).

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against
a published Ed25519 key (RS256/JWKS decommissioned). See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
