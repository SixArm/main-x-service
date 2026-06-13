## 5. Domain Model

The person entity has **one canonical domain model and three
representations**. The service's Rust model is canonical; the matcher
and front-end representations are projections of it.

### 5.1 Canonical `Person` (service)

Defined in the service crate (`src/models/person.rs`); field-by-field
reference in
[`person-service-rust-crate/AGENTS/models.md`](../person-service-rust-crate/AGENTS/models.md).
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
[`src/matching/adapter.rs`](../person-service-rust-crate/src/matching/adapter.rs):
`to_matcher_person(&service::Person) -> person_matcher::Person`.

Routing rules (normative; pinned by
[`tests/duplicate_detection.rs`](../person-service-rust-crate/tests/duplicate_detection.rs)):

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
`active`, `links`, `managing_organization`, timestamps, …) are dropped
— they have no matcher counterpart. Full rationale: service
[spec §6.2](../person-service-rust-crate/spec/06-functional-requirements.md).

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
- Soft delete (`active = false`) is the only delete, end to end: the
  service never row-deletes, and the front-end never offers hard
  delete.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
