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

