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
  [`AGENTS/security-and-privacy.md`](../place-matcher-rust-crate/AGENTS/security-and-privacy.md).
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
