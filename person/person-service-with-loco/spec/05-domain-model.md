## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](../AGENTS/models.md).

### 5.1 `Person`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` + optional
  `tax_id` shortcut.
- **Names** — primary `name: HumanName` + `additional_names`; each name
  carries `use_type`, family, given, prefix, suffix.
- **Contact** — `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`.
- **Identity documents** — passport, birth certificate, national ID,
  driver's licence, voter ID, military ID, residence / work permit.
- **Emergency contacts** — name, relationship, telecom, address,
  `is_primary` flag.
- **Demographics** — `gender`, `birth_date`, `marital_status`,
  `multiple_birth`, `deceased` + `deceased_datetime`, `photo`.
- **Organisation** — `managing_organization` reference + per-person
  `links: Vec<PersonLink>` (`ReplacedBy` / `Replaces` / `Refer` /
  `Seealso`).
- **Audit** — `active`, `created_at`, `updated_at`.

> **Partition rule — within-entity links vs cross-service links.** The
> within-entity `links: Vec<PersonLink>` (and any within-entity
> `relationships`) reference **other person records** and ARE a matcher
> signal. Cross-service `entity_links` (§5.4 — `same_identity` to a
> worker, `works_at` to an organization) are **entirely separate**: they
> are NOT stored in `links`/`relationships`, NOT routed to the matcher,
> and NOT a match signal. The matching adapter
> (`src/matching/adapter.rs`) MUST NEVER project `entity_links` into the
> matcher input. See [cross-service linking §7](../../../agents/share/cross-service-linking.md).

### 5.2 Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name.family` is non-empty.
- `birth_date`, when present, is not in the future.
- An `Identifier` is unique within `(person_id, identifier_type, system, value)`.
- `IdentityDocument.expiry_date`, when present, is on or after `issue_date`.
- Soft delete (`active = false`) is the only delete; rows MUST NOT be
  removed.

### 5.3.1 National-identifier scheme routing (matcher bridge)

The matching bridge (`src/matching/adapter.rs::route_identifier`) projects a
service `Identifier` onto one of the `person-matcher` crate's **26**
national-ID builder slots, keyed by a case-insensitive substring of the
identifier's `system` URI (most-specific fragment first). As of E-8's
follow-up, **all 26** slots are reachable from service data; none are
matcher-only.

| `system` URI contains | Matcher slot |
|---|---|
| `nhs.uk` / `uk-nhs` / `nhs-number` | `united_kingdom_national_health_service_number` |
| `us-ssn` / `ssa.gov` (+ type `SSN`/`TAX`, + `tax_id` shortcut) | `us_ssn` |
| `cpf` | `br_cpf` |
| `nir` / `ameli.fr` | `fr_nir` |
| `tsi` / `ingesa` | `es_tsi` |
| `aadhaar` / `uidai` | `in_aadhaar` |
| `my-number` / `myna` | `jp_my_number` |
| `curp` | `mx_curp` |
| `personnummer` | `se_personnummer` |
| `kvnr` | `de_kvnr` |
| `bsn` | `nl_bsn` |
| `nhi` | `nz_nhi` |
| `ihi` (≥14 digits) | `au_ihi` |
| `ihi` (<14 digits) | `ie_ihi` |
| `hc-number` / `health-and-care` | `uk_hc_number` |
| `chi-number` / `:chi` / `/chi` | `uk_chi_number` |
| `nino` / `national-insurance` | `uk_nino` |
| `codice` / `it-cf` / `:cf` | `it_cf` |
| `egn` | `bg_egn` |
| `dni` | `es_dni` |
| `oib` | `hr_oib` |
| `fnr` / `fodselsnummer` | `no_fnr` |
| `pesel` | `pl_pesel` |
| `cnp` | `ro_cnp` |
| `emso` | `si_emso` |
| `rrn` | `cn_rrn` |

A typed `Identifier` with an unrecognised `system` falls back to its type
default (`SSN`/`TAX` → `us_ssn`; `PPN` → handled via `IdentityDocument`;
`MRN`/`DL`/`NPI`/`Other` → unrouted). Every row is pinned by
`tests/duplicate_detection.rs::all_national_id_schemes_route_to_their_slot`,
which asserts each scheme both routes to its slot **and** drives a
deterministic match on a shared well-formed value despite divergent names.
Adding a scheme is a three-part change: a fast path in `route_identifier`, a
row here and in the adapter rustdoc, and a test case.

### 5.4 Cross-service entity links (write side)

Distinct from within-entity `links`/`relationships` (see the partition
rule in §5.1), a Person may originate **cross-service edges** to records
in sibling services. The full topology — shared `EntityRef` URN format,
the read-model aggregator, integrity lifecycle, governance, and the
edge-kind registry — is fixed in
[cross-service linking](../../../agents/share/cross-service-linking.md);
this section documents only the **write side that the Person Service
owns**.

Person owns these outbound edge kinds in v1
([cross-service linking §9](../../../agents/share/cross-service-linking.md)):

| Kind | From → To | Direction | Card. | Temporal |
|---|---|---|---|---|
| `same_identity` | person ↔ worker | symmetric (either side may assert; aggregator canonicalises) | 1:1 | no |
| `works_at` / `member_of` | person → organization | directed | M:N | yes (`valid_from`/`valid_to`) |

Outbound edges are stored in a dedicated `entity_links` table (§10.4),
**not** in `person_links`/`relationships`. Per
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md),
each row carries `from_pid` (the local person), `kind`, `to_ref` (the
target `EntityRef` URN, e.g. `organization:9a2f…`), optional `role`,
`confidence`, `provenance`, and `valid_from`/`valid_to`, with a soft
`deleted_at`.

Writes are **optimistic**: creating a link records the assertion and
emits an event — it does **not** call the target service, so latency and
availability are unaffected by the target service's state. Verification
(`unverified` / `verified` / `dangling`) is the aggregator's view, not a
write-side property, because only the aggregator sees both endpoints
([cross-service linking §5](../../../agents/share/cross-service-linking.md)).

