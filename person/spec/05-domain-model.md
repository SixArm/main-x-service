## 5. Domain Model

The person entity has **one canonical domain model and three
representations**. The service's Rust model is canonical; the matcher
and front-end representations are projections of it.

### 5.1 Canonical `Person` (service)

Defined in the service crate (`src/models/person.rs`); field-by-field
reference in
[`person-service-with-loco/AGENTS/models.md`](../person-service-with-loco/AGENTS/models.md).
Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>`
  (`(identifier_type, system, value)`) + optional `tax_id` shortcut.
- **Names** — primary `name: HumanName` + `additional_names`.
- **Contact** — `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`,
  `emergency_contacts`.
- **Identity documents** — passport, birth certificate, national ID,
  driver's licence, voter ID, military ID, residence / work permit.
- **Demographics** — `gender`, `birth_date`, `marital_status`,
  `deceased` + `deceased_datetime`, `multiple_birth`, `photo`.
- **Biological parentage** — optional `biological_mother` and
  `biological_father`, **each a reference to another `Person` in the
  registry** (by `id`; 0..1 each, nullable — unknown parents stay null).
  Self-referential links within the dataset (schema.org `parent`),
  distinct from `emergency_contacts` and from the merge
  `links: Vec<PersonLink>`.
- **Household** — the set of people **living together in one home /
  flat / place**. A `Household { id, address (the dwelling), members }`
  groups all co-resident persons; a `Person` may belong to **0..many**
  households (membership is **many-to-many** — e.g. a child of divorced
  parents who lives in two homes), expressed as `household_ids` on the
  person and `members` on the household. Membership reflects current
  shared residence, independent of biological parentage (parents and
  children may or may not share a household).
- **Relationships** — typed person-to-person links:
  `relationships: Vec<PersonRelationship>`, each `{ relation, person_id }`
  **referencing another `Person` in the registry**. `relation` is a
  `RelationKind` enum, initially **`ParentOf`**, **`ChildOf`**,
  **`SiblingOf`**, and **`GuardianOf`**:
  - `ParentOf` / `ChildOf` are **inverses** — A `ParentOf` B ⇔ B `ChildOf` A.
  - `SiblingOf` is **symmetric** — A `SiblingOf` B ⇔ B `SiblingOf` A.
  - `GuardianOf` / `WardOf` are **inverses** — A `GuardianOf` B (A is the
    legal guardian) ⇔ B `WardOf` A (B is the ward / under guardianship).
    Guardianship is a legal relationship, independent of parentage (a
    guardian may or may not be a parent).
  These are broader than the `biological_mother` / `biological_father`
  fields (which name the two specific *biological* parents): a
  parent-of / child-of link may be biological, step, adoptive, legal, or
  foster. The enum is extensible (e.g. `SpouseOf` later).
- **Tags** — `tags: Vec<String>`, a list of short free-text labels an
  operator can attach to a record for grouping, filtering, triage, or
  workflow (e.g. `"vip"`, `"review"`, `"archived-2026"`, `"fast-track"`).
  **Any `Person` can carry tags.** Each tag is a short, trimmed,
  non-empty string; the list is unordered, de-duplicated
  case-insensitively, and defaults to empty. The `Person` entity has no
  `keywords` field, so **tags are the labelling mechanism** — they are
  user-applied operational labels for grouping and workflow, not
  algorithmically derived descriptors. Tags ARE a **supporting** match
  signal: the matcher scores them by plain set Jaccard over the
  case-insensitively normalised tag sets (matcher §12.2), weighted
  `tags_weight` (§5.3). A supporting signal only — not identifying on its
  own.
- **Registry plumbing** — `active`, `managing_organization`,
  `links: Vec<PersonLink>`, `created_at`, `updated_at`.

### 5.2 Matcher `Person` (flat builder shape)

Defined in the matcher crate
([spec §8](../person-matcher-rust-crate/spec/08-domain-model.md)):
flat fields (`family_name`, `given_name`, `date_of_birth`, `address`,
`phone` / `mobile` / `email`, …), one field per national-identifier
scheme (42 schemes), and `passport_books: Vec<PassportBook>`.

### 5.3 Service ↔ matcher DTO contract (the adapter)

The service embeds the matcher (path dependency, re-exported from
`src/matching/mod.rs` as `matcher_lib`) and bridges via
[`src/matching/adapter.rs`](../person-service-with-loco/src/matching/adapter.rs):
`to_matcher_person(&service::Person) -> person_matcher::Person`.

Routing rules (normative; pinned by
[`tests/duplicate_detection.rs`](../person-service-with-loco/tests/duplicate_detection.rs)):

- `name.family` → `family_name`; first/second `name.given` →
  `given_name` / `middle_name`.
- `birth_date` → `date_of_birth`; `gender` → `gender`.
- First `addresses[]` → `address` (rest → `previous_addresses`);
  `state` renamed `county`, `postal_code` → `postcode`.
- First telecom of each `ContactPointSystem` → `phone` / `mobile` /
  `email`.
- `identifiers[]` routed to scheme-specific slots by `system` URI
  (e.g. `https://fhir.nhs.uk/Id/nhs-number` → `uk_nhs_number`);
  falls back to `IdentifierType` when no URI hint.
- `tax_id` defaults to `us_ssn` unless a typed identifier overrides.
- `IdentityDocument` of type `Passport` → `passport_books`.
- `relationships[]` → matcher `relationships` (typed `(relation,
  person_id)` refs); `biological_mother` / `biological_father` fold in as
  `ParentOf` refs (the strongest parent signal). Scored by typed-set
  Jaccard (matcher §12.2), weighted `relationships_weight`. `household_ids`
  stay registry-only (dropped — household co-membership is a weaker,
  separate signal, not routed today).
- `tags[]` → matcher `tags` (case-insensitively normalised label set).
  Scored by plain set Jaccard (matcher §12.2), weighted `tags_weight`.
  Tags are a **supporting** signal, not in the lossy-drop list below.

#### 5.3.1 Identifier scheme-routing audit (E-8)

`route_identifier` reaches a **subset** of the matcher's national-ID
slots. The matcher exposes 26 national-ID builder slots; the adapter
routes 14 of them via `system`-URI substring fast paths (plus the
type-based `tax_id`/`SSN`/`TAX` → `us_ssn` defaults). The remaining 12
slots are **unreachable from current service data** — no routing rule
targets them, so an operator cannot populate them through the service
shape today.

**Routable slots** (system-URI substring → matcher slot):

| `system` URI contains | Matcher slot | Bridge test |
|---|---|---|
| `nhs.uk` / `uk-nhs` / `nhs-number` | `united_kingdom_national_health_service_number` | `shared_nhs_number…`, `routable_identifier_systems…` |
| `us-ssn` / `ssa.gov` (+ type SSN/TAX, +`tax_id`) | `us_ssn` | `typed_ssn…`, `shared_tax_id…`, `routable_identifier_systems…` |
| `cpf` | `br_cpf` | `shared_cpf…`, `routable_identifier_systems…` |
| `nir` / `ameli.fr` | `fr_nir` | `routable_identifier_systems…` |
| `tsi` / `ingesa` | `es_tsi` | `routable_identifier_systems…` |
| `aadhaar` / `uidai` | `in_aadhaar` | `routable_identifier_systems…` |
| `my-number` / `myna` | `jp_my_number` | `routable_identifier_systems…` |
| `curp` | `mx_curp` | `routable_identifier_systems…` |
| `personnummer` | `se_personnummer` | `routable_identifier_systems…` |
| `kvnr` | `de_kvnr` | `routable_identifier_systems…` |
| `bsn` | `nl_bsn` | `routable_identifier_systems…` |
| `nhi` | `nz_nhi` | `routable_identifier_systems…` |
| `ihi` (≥14 digits) | `au_ihi` | `ihi_disambiguates…` |
| `ihi` (<14 digits) | `ie_ihi` | `ihi_disambiguates…` |

**Unreachable slots** (no routing rule; matcher-only until a URI hint or
type default is added): `uk_hc_number`, `uk_chi_number`, `uk_nino`,
`it_cf`, `bg_egn`, `es_dni`, `hr_oib`, `no_fnr`, `pl_pesel`, `ro_cnp`,
`si_emso`, `cn_rrn`.

Adding a scheme to the routable set is a three-part change: a new fast
path in `route_identifier`, a row in this table + the `adapter.rs`
rustdoc, and a case in `routable_identifier_systems_reach_their_matcher_slot`.

The projection is **lossy by design**: registry-only fields (`id`,
`active`, `links`, `household_ids`, `managing_organization`,
timestamps, …) are dropped
— they have no matcher counterpart. Full rationale: service
[spec §6.2](../person-service-with-loco/spec/06-functional-requirements.md).

### 5.4 Front-end TypeScript types

The front-end mirrors the service's wire format in
`src/lib/api/types.ts` (`Person`, `HumanName`, `MatchResult`, …) and
unwraps the shared envelope in `src/lib/api/client.ts`. The service
model is upstream: if a field changes in the service, the front-end
types MUST be fixed in the same change cycle (front-end
[`AGENTS.md`](../person-front-end-with-svelte/AGENTS.md)).

### 5.5 Shared invariants

All subprojects MUST uphold:

- `name.family` is non-empty; `birth_date`, when present, is not in
  the future.
- An `Identifier` is unique within
  `(person_id, identifier_type, system, value)`.
- National identifiers are **scheme-local** — never cross-matched
  across schemes (matcher FR-13; the adapter routes, it does not
  coerce).
- `biological_mother` / `biological_father`, when set, **reference an
  existing `Person`** in the registry (or are null); a person is **not
  its own** parent, and parentage **must not form a cycle** (no person is
  their own ancestor).
- A `Person` may belong to **0..many** households (membership is
  many-to-many — e.g. a child of divorced parents living in two homes).
  A `Household`'s `members` and a person's `household_ids` all reference
  existing records.
- A `PersonRelationship` references an **existing** `Person`; **no person
  relates to itself** (not its own parent / child / sibling / guardian).
  `ParentOf` / `ChildOf` and `GuardianOf` / `WardOf` must stay **acyclic**
  (no person is their own ancestor or guardian, directly or transitively)
  and, where both directions are stored, mutually consistent
  (A `ParentOf` B ⇔ B `ChildOf` A; A `GuardianOf` B ⇔ B `WardOf` A);
  `SiblingOf` is symmetric.
- `tags` are short, trimmed, non-empty strings, de-duplicated
  case-insensitively; the list is unordered and defaults to empty. The
  canonical model is upstream — the service model, matcher DTO (where
  carried), and front-end types follow in the same change cycle (§5.1
  contract). Tags are a **supporting** match signal, scored by set
  Jaccard in the matcher (weighted `tags_weight`; §5.3).
- Soft delete (`active = false`) is the only delete, end to end: the
  service never row-deletes, and the front-end never offers hard
  delete.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
