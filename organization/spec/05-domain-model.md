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
| `tags` | `Vec<String>` | — (operator-applied labels) |
| `relationships` | `Vec<OrganizationRelationship { relation, organization_id }>` | subOrganization / parentOrganization (and beyond) |

`PostalAddress` (all fields optional): `street_address`, `locality`,
`region`, `postal_code`, `country`. Only fields present on *both*
sides of a comparison contribute to the address score.

**Relationships** — typed organization-to-organization links:
`relationships: Vec<OrganizationRelationship>`, each
`{ relation, organization_id }` **referencing another `Organization` in
the registry**. `relation` is a `RelationKind` enum, initially
**`SubOrganizationOf`**, **`ParentOrganizationOf`**, **`SuccessorOf`**,
and **`PredecessorOf`**:

- `SubOrganizationOf` / `ParentOrganizationOf` are **inverses** (schema.org
  `subOrganization` / `parentOrganization`) — A `SubOrganizationOf` B ⇔ B
  `ParentOrganizationOf` A. This generalises the org-chart containment
  hierarchy into the relationship set.
- `SuccessorOf` / `PredecessorOf` are **inverses** capturing mergers,
  renames, and reorganisations — A `SuccessorOf` B ⇔ B `PredecessorOf` A
  (A is the surviving / renamed entity that succeeds B).

The enum is **extensible** (e.g. `AffiliatedWith` later). Relationships
are a **supporting** identity signal — never identifying on their own.

**Partition rule — within-entity `relationships` vs cross-service links.**
The `relationships[]` here are **within-entity**: organization → organization
references inside this registry, and they **are** a matcher signal
(scored by typed-set Jaccard, above). They are entirely **separate** from
**cross-service links** (e.g. `person works_at organization`), which live
only in the aggregator and the originating service's `entity_links` /
`linked` events — **never** in `relationships`, and **never** fed to any
matcher (cross-service edges are not sameness evidence). See
[`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
§7 and §8.6 here.

`telephone` and `email` are carried but **not yet scored** (matcher
spec §23 task).

**Tags** — `tags: Vec<String>` is a list of **short free-text labels**
that operators attach to a record for grouping, filtering, triage, or
workflow (e.g. `"vip"`, `"review"`, `"archived-2026"`, `"fast-track"`).
**Any `Organization` can carry tags.** Each tag is a short, trimmed,
non-empty string; the list is **unordered**, **de-duplicated
case-insensitively**, and **defaults to empty**.

Tags are **distinct from `keywords`**: `keywords` are descriptive /
discovery terms about *what the record is* (sector, subject, what the
organization does); **`tags` are user-applied operational labels** for
grouping and workflow. The two coexist — neither replaces the other.

Tags ARE a **supporting match signal**: the matcher scores them by set
Jaccard over the case-insensitively normalised tag sets (matcher spec
§14b), weighted `tags_weight` (default `0.05`) — never identifying on
their own, and skipped when either side has no tags. Operationally they
remain a registry attribute too (grouping / filtering / triage).

As the canonical type, `Organization` is **upstream**: `tags` reaches
the service (same type, persisted verbatim in `data`) and the front-end
TypeScript mirror automatically, and the front-end types MUST be fixed
in the same change cycle (§5.4).

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

Because the entity uses the matcher's canonical type at every tier,
there is no DTO fork and no lossy projection: `relationships[]` is part
of the `Organization` payload, persists verbatim in `data`, and reaches
the matcher's `relationships` field **unchanged** — it is **not** dropped
(contrast the registry-only persistence columns `id` / `active` /
`deleted_at`, which have no matcher counterpart). The matcher scores it
by typed-set Jaccard over the `(relation, organization_id)` pairs
(matcher spec §14a), weighted `relationships_weight`.

Likewise `tags` is part of the `Organization` payload, persists verbatim
in `data`, and reaches the matcher's `tags` field **unchanged** — it is
**not** in the lossy-drop set. The matcher scores it by set Jaccard over
the case-insensitively normalised tag sets (matcher spec §14b), weighted
`tags_weight` (default `0.05`).

### 5.4 Front-end TypeScript mirror

The front-end mirrors the wire format in
[`src/lib/api/types.ts`](../organization-front-end-with-svelte/src/lib/api/types.ts)
(`Organization`, `OrgIdentifier`, `PostalAddress`, `OrgRef`,
`ScoredRef`, `OrganizationRelationship`, `RelationKind`). The matcher type is upstream: if a field changes there,
the service re-exports it automatically (same type) but the front-end
types MUST be fixed in the same change cycle.

### 5.5 Shared invariants

All subprojects MUST uphold:

- `name` is non-empty — the service rejects blank names on create;
  the matcher requires it for the always-present name component.
- The JSONB `data` payload round-trips losslessly through
  `serde_json` (pinned by the service's
  [`tests/matching.rs`](../organization-service-with-loco/tests/matching.rs)).
- Identifier values are compared **within a scheme** only; tax IDs
  never match across jurisdictions; classification codes never pin.
- An `OrganizationRelationship` references an **existing**
  `Organization`; **no organization relates to itself** (not its own
  parent / sub / successor / predecessor). The directional kinds must stay
  **acyclic** — `SubOrganizationOf` / `ParentOrganizationOf` and
  `SuccessorOf` / `PredecessorOf` form no cycle (no organization is its own
  ancestor or successor, directly or transitively) — and, where both
  directions are stored, mutually **inverse-consistent**
  (A `SubOrganizationOf` B ⇔ B `ParentOrganizationOf` A; A `SuccessorOf` B
  ⇔ B `PredecessorOf` A). No symmetric kinds are defined today; a future
  symmetric kind (e.g. `AffiliatedWith`) MUST be stored symmetrically.
- `tags` are short, trimmed, non-empty strings; the list is
  de-duplicated **case-insensitively** and defaults to empty.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
- Soft delete is the only delete, end to end: the service never
  row-deletes, and the front-end offers no hard delete.
- Diacritics are preserved in normalisation (`Müller` ≠ `Muller`).
