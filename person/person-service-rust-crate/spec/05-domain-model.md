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

