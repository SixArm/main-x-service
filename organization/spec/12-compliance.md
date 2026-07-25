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

### 12.3 Extended frameworks

Four frameworks impose obligations beyond the table above. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). Organization is
the family's **FHIR reference implementation**
([`agents/share/fhir.md`](../../agents/share/fhir.md) §10), which makes
the ONC row the load-bearing one here.

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity | **Weak, and honestly so** — a company register is public by design (§12.2). It engages only for healthcare organizations acting as covered entities, and then mostly as the *provider directory* other systems' audit trails point at. | The **tamper-evident audit chain** (a SHA-256 chain over `audit_logs`), adopted on ISO/IEC 27001 and register-integrity grounds rather than HIPAA's: a public register's credibility rests on its trail being provably unrewritten. **Read-auditing** applies only to the person-linked contact fields §12.1 flags, not to the open registration identity. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | GDPR engaged for the sole-trader and contact-field cases §12.1 names. **EHDS not engaged** directly, though a healthcare organization record is the *data holder* identity its Ch. IV pipeline references. | An **erasure path that survives immutable history** for sole-trader records (redact content, keep chain linkage) — which must respect the §12.2 per-field split, erasing person-linked contact data without destroying the public registration identity a register is legally obliged to keep; plus a declared **data residency** and **lawful basis** (public task). |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **The strongest ONC fit in the family after person/worker.** Organization maps to **`Organization`**, which has a US Core profile, and this service is already the family's FHIR reference. | **Profile and terminology validation**: US Core Organization must-support elements (`identifier` — NPI, CLIA — `active`, `name`, `telecom`, `address`) and cardinalities, with identifier systems validated against their registries rather than merely non-blank; `$validate`; SMART discovery; and Bulk Data `$export`. Because this crate is the FHIR copy-source, its conformance implementation is what the other seven inherit. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device. The engagement is **supply-chain and configuration evidence**, plus one real hazard: a merged or misidentified provider organization can misroute a referral. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability**, notably over the deterministic identifier check-digit validators (LEI / GLN / DUNS / VAT — SEC-M5), where a silent regression creates false identity; and **signed, reproducible builds**. |

### 12.4 Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; the family serves **R5**. Implementing US Core-shaped
  validation against an R5 resource is genuinely useful and genuinely not
  certification.
- **Masking and Article 15 export are still deferred** (the table above),
  so the per-field transparency-vs-privacy split §12.2 promises is not
  yet enforced anywhere in code.
- **Every extended control is unimplemented in this service today.** The
  reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/);
  organization takes the FHIR conformance machinery at step 4 of the
  rollout ([`spec/compliance` §8.5](../../spec/compliance/index.md)) and
  is the natural second adopter, since it already owns the FHIR pattern.
