## 12. Compliance

The entity targets public-sector deployment; the governing frameworks
are the technology set in
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).
Healthcare-specific frameworks
([`compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md))
apply only where a deployment registers healthcare organizations.

| Standard | Mechanism (today / planned) |
|---|---|
| EU / UK GDPR | Soft delete (retention with erasure semantics) ✔; audit trail ✔; per-field masking + Article 15 export endpoint **deferred** (§13); consent model **deferred** |
| UK DPA 2018 | Same mechanisms as GDPR; public-task lawful basis expected for register operation (deployment-side) |
| ISO/IEC 27001 | Audit log with snapshots ✔; attributable `actor` pending PASETO v4.public auth (§13); operational controls deployment-side |
| ISO/IEC 42001:2023 | Matching is deterministic + explainable (per-component breakdowns, no ML) ✔ — AIMS controls become relevant only if ML scoring is ever introduced (none planned) |

### 12.1 Organization data is not automatically non-personal

Most organization fields are public-register data, but GDPR applies
wherever a record identifies a natural person:

- **Sole traders and partnerships** — the organization's `name`,
  `address`, and tax identifiers can be the proprietor's personal
  data. Treat such records as personal data end to end.
- **Contact fields** — `telephone` and `email` may be a named
  person's direct contact details; they are first in line for the
  masking layer when it lands.
- Both subproject specs already flag this (service spec §12,
  front-end spec §12); the entity-level rule is: **the privacy layer,
  when implemented, must cover sole-trader records and contact
  fields, not just "sensitive" identifier types.**

### 12.2 Transparency vs privacy

Company registers are *meant* to be public — transparency of legal
entities is the register's purpose, and identifiers such as LEI and
VAT are published deliberately. The tension is resolved per field,
not per record: registration identity (name, legal name, identifiers,
jurisdiction, founding date) defaults to open; person-linked contact
data defaults to protected. The deferred masking design (§13) MUST
honour this split, and the audit trail keeps every disclosure
decision reviewable.
