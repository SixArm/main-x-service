## 5. Domain Model

The organization entity has **one canonical type, used at every
tier**: `organization_matcher::Organization`. The service does not
fork a DTO (its golden rule), the front-end mirrors it in TypeScript,
and PostgreSQL stores it verbatim as JSONB. This is deliberately
simpler than the person entity's adapter projection — there is no
mapping layer to drift.

### 5.1 Canonical `Organization` (matcher crate)

Defined in
[`organization-matcher-rust-crate/src/organization.rs`](../organization-matcher-rust-crate/src/organization.rs);
normative description in the matcher
[spec §6](../organization-matcher-rust-crate/spec/index.md). Wire
field names are **snake_case** (the struct carries no serde rename).

| Field | Type | schema.org analogue |
|---|---|---|
| `name` | `String` (required) | name |
| `legal_name` | `Option<String>` | legalName |
| `alternate_names` | `Vec<String>` | alternateName |
| `identifiers` | `Vec<OrgIdentifier { scheme, value }>` | identifier |
| `url` | `Option<String>` | url |
| `same_as` | `Vec<String>` | sameAs |
| `address` | `Option<PostalAddress>` | address |
| `jurisdiction` | `Option<String>` (ISO 3166) | — (register jurisdiction) |
| `founding_date` | `Option<String>` (ISO-8601) | foundingDate |
| `telephone` | `Option<String>` | telephone |
| `email` | `Option<String>` | email |
| `keywords` | `Vec<String>` | keywords |

`PostalAddress` (all fields optional): `street_address`, `locality`,
`region`, `postal_code`, `country`. Only fields present on *both*
sides of a comparison contribute to the address score.

`telephone` and `email` are carried but **not yet scored** (matcher
spec §23 task).

### 5.2 Identifier schemes

`IdentifierScheme` partitions into three classes with different
matching authority:

| Class | Schemes | Matching behaviour |
|---|---|---|
| Deterministic (globally unique) | `Lei`, `Duns`, `Iso6523`, `Gln`, `Wikidata`, `Ror`, `Isni`, `Vat` | Shared value → score pinned to 1.0 (R-0) |
| Jurisdiction-scoped | `TaxId` | Pins to 1.0 only with matching `jurisdiction` (R-1) |
| Classification | `Naics`, `IsicV4`, `Sic` | Sector descriptor — never a pin, evidence at most |
| Escape hatch | `Custom(String)` | Never a pin |

### 5.3 Persistence representation (service)

One `organizations` row per record
(migration `m20220101_000001_organizations`):

| Column | Type | Content |
|---|---|---|
| `id` | auto PK | Internal only — never exposed |
| `pid` | UUID, unique | Public identifier |
| `name` | string | Denormalised from the payload for listing + `ILIKE` search |
| `data` | JSONB | The full `Organization` payload, verbatim |
| `active` | bool (default true) | Active flag |
| `deleted_at` | timestamptz, nullable | Soft-delete stamp; reads filter `IS NULL` |

### 5.4 Front-end TypeScript mirror

The front-end mirrors the wire format in
[`src/lib/api/types.ts`](../organization-front-end-with-svelte/src/lib/api/types.ts)
(`Organization`, `OrgIdentifier`, `PostalAddress`, `OrgRef`,
`ScoredRef`). The matcher type is upstream: if a field changes there,
the service re-exports it automatically (same type) but the front-end
types MUST be fixed in the same change cycle.

### 5.5 Shared invariants

All subprojects MUST uphold:

- `name` is non-empty — the service rejects blank names on create;
  the matcher requires it for the always-present name component.
- The JSONB `data` payload round-trips losslessly through
  `serde_json` (pinned by the service's
  [`tests/matching.rs`](../organization-service-rust-crate/tests/matching.rs)).
- Identifier values are compared **within a scheme** only; tax IDs
  never match across jurisdictions; classification codes never pin.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
- Soft delete is the only delete, end to end: the service never
  row-deletes, and the front-end offers no hard delete.
- Diacritics are preserved in normalisation (`Müller` ≠ `Muller`).
