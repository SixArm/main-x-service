## 12. Compliance

A public governmental deployment must satisfy the technology
compliance baseline in
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md):
EU GDPR, UK GDPR, UK DPA 2018, ISO/IEC 27001, ISO/IEC 42001.

### 12.1 Place data can be personal data

A place record is not inherently personal data — but it **becomes
personal data when linked to an identifiable individual**: a home
address, a residence-type place, a place whose `telephone` reaches a
person, or precise coordinates of where someone lives. The entity
therefore treats residence-linked places with person-grade controls:

- **Masking** — phone / fax redaction and coordinate rounding to 2
  decimal places (~1 km) via `GET /api/places/{id}/masked` and the
  `mask_sensitive` search flag (service spec §6.6). Precision policy
  is an open question (service spec §16 OQ-1).
- **Consent** — `Consent` records (`DataProcessing` / `DataSharing` /
  `Marketing` / `Research`; `Active` / `Revoked` / `Expired`) where
  the place represents a private residence.

### 12.2 Mechanisms by framework

| Framework | Mechanism |
|---|---|
| GDPR Art. 15 (access) | `GET /api/places/{id}/export` — full-record export |
| GDPR Art. 17 (erasure) | Soft delete + consent revocation; soft delete is the only delete end to end |
| GDPR Art. 5(1)(f) / UK DPA (integrity & confidentiality) | Masked views, no-PII-in-logs rule in all three subprojects, env-based secrets |
| GDPR accountability / UK DPA | Complete audit trail: who / what / when, old + new JSON, queryable over REST |
| ISO/IEC 27001 | Operational controls (deployment-side): non-root containers, health checks, secrets management; security audit is roadmap |
| ISO/IEC 42001 | The matching "AI-ish" component is rule-based and fully explainable — per-component breakdowns (FR-8), deterministic outputs (NFR-5), tunable but inspectable weights; no opaque models |
| schema.org/Place | Domain-model conformance (service spec §12) |

### 12.3 Subproject obligations

- **Service** — implements every mechanism above; see service
  [spec §12](../place-service-with-loco/spec/12-compliance.md).
- **Matcher** — never logs or `Debug`-formats place data; no real
  personal data in tests (synthetic fixtures only); see matcher
  [`agents/security-and-privacy.md`](../place-matcher-rust-crate/agents/security-and-privacy.md).
- **Front-end** — never `console.log`s place values; confirm-before-
  delete; GDPR-export and masked-view UI are deferred tasks (front-end
  [spec §12](../place-front-end-with-svelte/spec/12-compliance.md), §13
  T-19 / T-20).

### 12.4 Gaps

User attribution in the audit trail is incomplete until JWT
enforcement lands (E-5): unauthenticated writes cannot carry a
verified `user_id`, which weakens the accountability story regulators
expect. Cross-border data residency for multi-region replication is
unresolved — [§16](16-open-questions.md).

### 12.5 Extended frameworks

Four frameworks impose obligations beyond §12.2. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.528 accounting of disclosures | **Conditional, and narrower than person/worker** — engaged only for the residence-linked places §12.1 already singles out. A home address plus precise coordinates is the location of an identifiable individual; looking one up is activity worth recording. | **Read-auditing** on the paths that reveal an address or coordinates (`get`, `search`, `export`, the unmasked view, FHIR reads), carrying purpose-of-use and a disclosure flag; and **tamper-evident history** (a SHA-256 chain over `audit_log`), which also repairs the §12.4 accountability gap by making the trail's own integrity provable even while `user_id` attribution is incomplete. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | Fully engaged for residence-linked places; EHDS barely, since a place is not health data itself (it becomes so only through linkage). | An **erasure path that survives immutable history** (redact content, keep chain linkage), replacing "soft delete is erasure"; a declared **data residency** — which is the concrete answer to §12.4's unresolved cross-border question for multi-region replication; a recorded **lawful basis**; and export beyond the region recorded as a **Ch. V transfer**. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | Modest but real: place maps to **`Location`**, which has a US Core profile. It is a supporting resource in a certification bundle, not a headline one. | **Profile and terminology validation**: US Core Location must-support elements (`name`, `address`, `telecom`) and cardinalities, with `address` element structure and any `type` coding validated against its bound value set rather than merely non-blank; `$validate`; SMART discovery; Bulk Data `$export`. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device and no direct clinical hazard; the engagement is **supply-chain and configuration evidence**, which is worth having on general engineering grounds and feeds the ISO/IEC 27001 controls §12.2 already claims. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability** (notably over the GLN check-digit and coordinate-bounds validators, where a silent regression corrupts identity); and **signed, reproducible builds**. |

### 12.6 Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; the family serves **R5**.
- **Coordinate-precision policy is still open** (service spec §16 OQ-1);
  read-auditing records *that* an unmasked coordinate was disclosed, but
  it does not decide how precise a masked one should be.
- **Every extended control is unimplemented in this service today.** The
  reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/);
  place adopts the FHIR conformance machinery at step 4 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)).
