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
| GET | `/api/care-pathways/search?q=` | Case-insensitive name search (`ILIKE`, cap 50) |
| POST | `/api/care-pathways/match` | Rank `{query, candidates}` |
| POST | `/api/care-pathways/check-duplicates` | Match query vs stored pathways |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor |
| GET | `/api/care-pathways/merges/recent` | Merge-history records |
| GET | `/api/care-pathways/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/care-pathways/events/recent` | In-memory event stream |
| GET | `/api/care-pathways/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root path, public under auth enforcement) |

### Compliance ([spec §12](./spec/index.md))

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/compliance` | Posture: build provenance, IEC 62304 safety class, live controls, data-protection declarations, and per-framework **not-claimed** lines |
| GET | `/api/compliance/sbom` | CycloneDX 1.5 SBOM + SOUP register (from this binary's own `Cargo.lock`) |
| GET | `/api/compliance/audit/verify` | Verify the tamper-evident audit hash chain (HIPAA §164.312(c)) |
| GET | `/api/compliance/records/verify` | Verify row-level record integrity — detects an entity row edited outside the service |
| GET | `/api/care-pathways/{pid}/audit/disclosures` | Accounting of disclosures (HIPAA §164.528) |
| POST | `/api/care-pathways/{pid}/erase` | GDPR Art. 17 erasure — **irreversible**, destructive under ABAC |

### FHIR R5 ([spec §12.3](./spec/index.md))

| Method | Path | Purpose |
|---|---|---|
| GET/POST/PUT/DELETE | `/fhir/PlanDefinition{,/{id}}` | Resource interactions + search |
| POST | `/fhir/PlanDefinition/$validate` | Profile + terminology validation, persists nothing |
| GET | `/fhir/metadata` | `CapabilityStatement` (public) |
| GET | `/fhir/.well-known/smart-configuration` | SMART discovery — only when a SMART authorization server is configured, else `404` (public) |
| GET | `/fhir/$export` → `/$export-status/{id}` → `/$export-file/{id}/{file}` | Bulk Data Access (NDJSON); `DELETE /$export-status/{id}` cancels |

> **Not claimed:** ONC certification (this serves FHIR R5; certification
> targets R4 + US Core, and `PlanDefinition` has no US Core profile),
> SMART App Launch itself (the credential is PASETO, not OAuth 2.0), or
> medical-device qualification. See [spec §12.5](./spec/index.md).

See [AGENTS.md](./AGENTS.md) and [spec §6](./spec/index.md) for the full
route contract.

The body for a care pathway **is** the `care_pathway_matcher::CarePathway`
shape (name, pathway code + provider, care setting, target condition
codes (ICD/SNOMED), interventions, keywords, identifiers, sameAs).

## Quick start

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/care_pathway_service_development
cargo loco start        # migrations auto-run in development

# Create
curl -s localhost:5150/api/care-pathways -H 'content-type: application/json' \
  -d '{"name":"Acute Stroke Care Pathway","condition_codes":[{"system":"Icd10","code":"I63"}]}'

# Name search
curl -s 'localhost:5150/api/care-pathways/search?q=stroke'

# Match an explicit query against candidates (no persistence)
curl -s localhost:5150/api/care-pathways/match -H 'content-type: application/json' \
  -d '{"query":{"name":"Acute Stroke Care Pathway"},"candidates":[{"name":"Stroke Care Pathway"}]}'

# Check for duplicates of a query against stored pathways
curl -s localhost:5150/api/care-pathways/check-duplicates -H 'content-type: application/json' \
  -d '{"name":"Acute Stroke Care Pathway"}'

# Merge a duplicate into a survivor (the survivor is `main_pid`)
curl -s localhost:5150/api/care-pathways/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<duplicate-uuid>"}'

# Authenticated request: present a short-lived bearer PASETO minted by the
# auth-service. /whoami echoes the verified claims (401 without a valid
# token). With blanket enforcement on (CARE_PATHWAY_REQUIRE_AUTH=1) every
# /api/* route needs the same header.
curl -s localhost:5150/api/care-pathways/whoami \
  -H 'authorization: Bearer <paseto-from-authentication-service>'
```

A validation failure returns `422` with **every** problem in one body,
e.g. a blank `name` plus a malformed condition code plus a bad UUID
identifier come back together:

```bash
curl -s localhost:5150/api/care-pathways -H 'content-type: application/json' \
  -d '{"name":"  ","condition_codes":[{"system":"Icd10","code":"not-a-code"}],
       "identifiers":[{"scheme":"Uuid","value":"not-a-uuid"}]}'
# → 422 {"error":"validation","description":"name must not be blank; \
#        condition_codes[0] … ; identifiers[0] …"}
```

## Testing

```bash
cargo test                   # DB-free: matcher embedding, JSON round-trip,
                             # validation pins, the audit-chain and FHIR
                             # profile/terminology cores, and the
                             # requirement→test traceability check
cargo test -- --ignored      # request-level tests (need Postgres DATABASE_URL)
cargo clippy --all-targets
```

Two DB-free checks fail the build on purpose when the repository drifts:

- **`every_direct_dependency_is_annotated`** — adding a dependency
  without an entry in [`compliance/soup.tsv`](./compliance/soup.tsv)
  fails (IEC 62304 §8.1.2).
- **`every_named_test_exists`** — renaming or deleting a test that
  [`compliance/traceability.tsv`](./compliance/traceability.tsv) cites
  fails, so a requirement cannot silently lose its verification.

The audit chain's key property — that a digest computed before an
`INSERT` still matches after a Postgres JSONB round-trip — can only be
checked against a real database, so it lives in the `--ignored` suite
(`chain_survives_a_jsonb_round_trip`,
`tampering_with_a_row_breaks_verification`).

```bash
scripts/sbom.sh                        # CycloneDX SBOM + SOUP + cargo-deny
scripts/build-reproducible.sh --verify # build twice, compare hashes
cargo run --bin sbom                   # the SBOM alone, to stdout
```

Validation failures return `422 Unprocessable Entity` — the family
convention — for a blank `name`, a malformed `condition_codes` entry
(ICD-10 / ICD-11 / SNOMED CT Verhoeff), a malformed `identifiers` entry
(UUID / DOI shapes), or an `in_language` tag that is not valid BCP-47.
All problems are reported together in one body, on both create and
update. See [spec §6.1](./spec/index.md).

## Status

Implemented: CRUD + `ILIKE` name search + matching + record merge +
audit log + in-memory event streaming (durable-bus Phase 1) +
OpenAPI/Swagger + Prometheus metrics + offline PASETO v4.public
verification + blanket `/api/*` auth enforcement (off by default, gated
by `CARE_PATHWAY_REQUIRE_AUTH`) + rich payload validation
(ICD/SNOMED/UUID/DOI/BCP-47) + boot-time published-key fetch over HTTP
(once, when `CARE_PATHWAY_PASETO_KEYS_URL` is set, with warn + env
fallback). The durable event bus's Phase-2 transactional outbox + relay
have landed (`src/models/event_outbox.rs`, `src/relay.rs`,
`src/streaming.rs`; default-off via
`CARE_PATHWAY_EVENT_TRANSPORT=memory`).

**Compliance (2026-07-25)** — this crate is the family's reference
implementation of the four control-driving frameworks in
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2: a tamper-evident audit hash chain + read/disclosure auditing
(HIPAA), Art. 17 erasure that survives that chain plus residency and
lawful-basis declarations (GDPR / EU EHDS), FHIR profile + terminology
validation with `$validate`, SMART discovery and Bulk Data `$export`
(ONC / HTI), and a SOUP register, CycloneDX SBOM, machine-checked
requirement→test traceability and reproducible-build tooling
(IEC 62304 / SaMD). Read-auditing is **default off**
(`CARE_PATHWAY_AUDIT_READS`); for a compliance deployment also set
`CARE_PATHWAY_REQUIRE_AUTH` and `CARE_PATHWAY_EVENT_TRANSPORT=outbox`.
See [spec §12](./spec/index.md) — including §12.5, which states the
limits plainly.

Deferred (see
[spec §13](./spec/index.md)): Tantivy full-text/fuzzy search,
search-blocked dedup candidates, the durable bus's Phase-3 Fluvio broker
sink, privacy, front-end merge action, a key-set refresh loop (the
boot fetch runs once), and the compliance follow-ups in T-15
(row-level entity hashing, Bulk Data on `bg_pg`, CI wiring, Inferno).
Token issuance is
provided by the central
[authentication-service](../../authentication/authentication-service-with-loco).

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the
`CARE_PATHWAY_REQUIRE_AUTH` flag and enforcement semantics are unchanged,
only the credential changed. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
