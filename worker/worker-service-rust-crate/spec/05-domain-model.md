## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](../AGENTS/models.md).

### 5.1 `Worker`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` + optional
  `tax_id` shortcut.
- **Names** — primary `name: HumanName` + `additional_names`
  (former names, name at credential issuance, married / maiden forms).
- **Contact** — `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`.
- **Identity / credential documents** — passport, driver's licence,
  professional credentials, certificates with type + number +
  issuing authority + issue / expiry dates + verified flag.
- **Emergency contacts** — name, relationship, telecom, address.
- **Demographics** — `gender`, `birth_date`, `marital_status`,
  `multiple_birth`, `deceased`, `photo`.
- **Organisation** — `managing_organization` reference + per-worker
  `links: Vec<WorkerLink>` (`ReplacedBy` / `Replaces` / `Refer` /
  `Seealso`).
- **Audit** — `active`, `created_at`, `updated_at`.

### 5.2 Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name.family` is non-empty.
- `birth_date`, when present, is not in the future.
- An `Identifier` is unique within `(worker_id, identifier_type, system, value)`.
- `IdentityDocument.expiry_date`, when present, is on or after
  `issue_date`. Credentials with no expiry are non-expiring; an
  expiry in the past flags an expired credential but does not refuse
  the record.
- Soft delete is the only delete.

