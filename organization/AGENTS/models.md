# Domain Model Reference — Organization Entity

One canonical type across the trio:
`organization_matcher::Organization`. Normative description: entity
[spec §5](../spec/05-domain-model.md) and matcher
[spec §6](../organization-matcher-rust-crate/spec/index.md).

## Organization

**File:** [`organization-matcher-rust-crate/src/organization.rs`](../organization-matcher-rust-crate/src/organization.rs)
— used as-is by the service (API body + JSONB payload) and mirrored in
the front-end's TypeScript. Wire names are **snake_case** (no serde
rename).

| Field | Type | Description |
|---|---|---|
| name | String (required) | Common name — the only required field |
| legal_name | Option\<String\> | Registered legal name |
| alternate_names | Vec\<String\> | Trading names, former names, abbreviations |
| identifiers | Vec\<OrgIdentifier\> | Register identifiers (scheme + value) |
| url | Option\<String\> | Official website |
| same_as | Vec\<String\> | Authoritative reference URLs (deterministic on overlap) |
| address | Option\<PostalAddress\> | Registered / principal address |
| jurisdiction | Option\<String\> | ISO 3166 country of registration; gates the tax-ID rule |
| founding_date | Option\<String\> | ISO-8601; compared by year |
| telephone | Option\<String\> | Carried, not yet scored (matcher §23) |
| email | Option\<String\> | Carried, not yet scored (matcher §23) |
| keywords | Vec\<String\> | Free tags; Jaccard component |

**Constructor:** `Organization::new(name)` — everything else defaults
empty/`None`. All optional fields are `#[serde(default)]`, so sparse
JSON bodies deserialise cleanly.

## OrgIdentifier / IdentifierScheme

| Class | Schemes | Behaviour |
|---|---|---|
| Deterministic | `Lei`, `Duns`, `Iso6523`, `Gln`, `Wikidata`, `Ror`, `Isni`, `Vat` | Shared value → match pinned to 1.0 (R-0) |
| Jurisdiction-scoped | `TaxId` | Pins only with matching `jurisdiction` (R-1) |
| Classification | `Naics`, `IsicV4`, `Sic` | Sector, not identity — never pins |
| Escape hatch | `Custom(String)` | Never pins |

`IdentifierScheme::is_deterministic()` is the gate the matcher uses.

## PostalAddress

All fields optional; only fields present on **both** sides of a
comparison contribute to the address score.

| Field | Internal match weight |
|---|---:|
| street_address | 0.30 |
| locality | 0.25 |
| postal_code | 0.20 |
| region | 0.15 |
| country | 0.10 |

## Persistence row (service)

**Files:** [`src/models/organizations.rs`](../organization-service-rust-crate/src/models/organizations.rs),
[`migration/src/m20220101_000001_organizations.rs`](../organization-service-rust-crate/migration/src/m20220101_000001_organizations.rs)

| Column | Type | Notes |
|---|---|---|
| id | auto PK | Internal only |
| pid | UUID unique | Public id (route param, audit key) |
| name | string | Denormalised for list + `ILIKE` search |
| data | JSONB | The full `Organization`, verbatim — `Model::to_org()` deserialises it |
| active | bool default true | |
| deleted_at | timestamptz null | Soft delete; reads filter `IS NULL` |

Audit row (`audit_logs`,
[`src/models/audit_logs.rs`](../organization-service-rust-crate/src/models/audit_logs.rs)):
`entity_pid`, `action` (created/updated/deleted), `actor` (null until
auth lands), `snapshot` (JSONB).

Event (`OrgEvent`,
[`src/streaming.rs`](../organization-service-rust-crate/src/streaming.rs)):
`{kind, pid, name, seq}` in an in-memory ring buffer (capacity 1 000).

## Wire-only response shapes (service)

Defined in
[`src/controllers/organizations.rs`](../organization-service-rust-crate/src/controllers/organizations.rs):

- `OrgRef` — `{pid, name}` (create / update / list / search).
- `ScoredRef` — `{pid, name, score, confidence, is_match}`
  (check-duplicates).
- `MatchRequest` — `{query: Organization, candidates: [Organization]}`.

## Front-end TypeScript mirror

**File:** [`src/lib/api/types.ts`](../organization-front-end-with-svelte/src/lib/api/types.ts)
— `Organization`, `OrgIdentifier`, `PostalAddress`, `OrgRef`,
`ScoredRef`. The Rust type is upstream; fix the mirror in the same
change cycle as any DTO change (entity spec §5.4).
