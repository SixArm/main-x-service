# Care Pathway Service

A registry of **clinical care-pathway** records: CRUD + matching, built
on **loco.rs** and embedding the canonical
[care-pathway-matcher](../care-pathway-matcher-rust-crate).

A *care pathway* (clinical / critical / integrated care pathway) is a
structured, evidence-based, multidisciplinary plan of care for a
specific condition over a defined episode.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [care-pathway-front-end-with-svelte](../care-pathway-front-end-with-svelte)

## API

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/care-pathways` | Create |
| GET | `/api/care-pathways` | List |
| GET | `/api/care-pathways/{pid}` | Fetch |
| PUT | `/api/care-pathways/{pid}` | Update |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete |
| POST | `/api/care-pathways/match` | Rank `{query, candidates}` |
| POST | `/api/care-pathways/check-duplicates` | Match query vs stored pathways |

The body for a care pathway **is** the `care_pathway_matcher::CarePathway`
shape (name, pathway code + provider, care setting, target condition
codes (ICD/SNOMED), interventions, keywords, identifiers, sameAs).

## Quick start

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/care-pathway-service_development
cargo loco start        # migrations auto-run in development

curl -s localhost:5150/api/care-pathways -H 'content-type: application/json' \
  -d '{"name":"Acute Stroke Care Pathway","condition_codes":[{"system":"Icd10","code":"I63"}]}'
```

## Testing

```bash
cargo test --test matching   # DB-free: matcher embedding + JSON round-trip
cargo clippy --all-targets
```

## Status

MVP: CRUD + matching. Search, streaming, audit, privacy, OpenAPI, and
richer validation are tracked in [spec §13](./spec/index.md). JWT auth is
provided by the central
[authentication-service](../authentication-service-rust-crate).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
