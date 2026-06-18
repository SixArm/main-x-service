## 12. Compliance

This service stores **no entity records**, but it does store **edges**,
and some edges are themselves sensitive data. The compliance posture is
inherited from the most sensitive edge a response can surface.

### 12.1 Regulatory frame

Follows the family compliance baselines:

- [Healthcare](../../../agents/share/compliance-for-healthcare.md) — US
  HIPAA, UK DPA 2018, UK Common Law Duty of Confidentiality, NHS Act
  2006 §251, UK/EU GDPR.
- [Technology](../../../agents/share/compliance-for-technology.md) —
  UK/EU GDPR, ISO/IEC 27001, ISO/IEC 42001.

### 12.2 `case ↔ person` — the high-governance edge

The `subject_of` / `about` edge asserts that a person is the subject of
a government case (benefits, legal, investigation). The **link itself is
sensitive data**, so it carries the case service's posture, not the
lighter affiliation posture
([design §10](../../../agents/share/cross-service-linking.md#10-governance--case--person)):

- **Access control** on reading the edge — at least the authorisation
  required to read the case. Enforced on `/edges`, `/neighbors`, and
  transitively on `/single-view` (§6 FR-18). An unauthorised caller MUST
  NOT learn the edge exists (concealment, not a distinguishable
  `403`-vs-`404`).
- **Audit** of every read/write of these edges — the apply of a
  `linked` / `unlinked` / `merged` touching them, and any read that
  surfaces them — written to `audit_log` (§10.4, §6 FR-19), consistent
  with the case service's trail.
- **Privacy masking** — `single-view` / `neighbors` responses honour the
  same masking/authorisation as the case service (§6 FR-20).

### 12.3 Affiliation edges

`same_identity`, `works_at`, `member_of`, `employed_by` are
medium-sensitivity (design §9). They carry the default JWT-verification
posture (NFR-10) once the family auth rollout reaches this service
(§14). `same_identity` is an **identity assertion** and is
operator-asserted / high-confidence, but does not require the case-grade
controls above.

### 12.4 Data minimisation & derivation

- The read-model is **derived and rebuildable**; it holds no
  un-derivable personal data beyond what the entity services already
  publish on the bus.
- `EntityRef` is a UUID-bearing URN (the record's public `pid`); the
  service does not store entity record bodies, only the edges between
  references and a boolean presence flag.

### 12.5 Audit vs event stream

The `audit_log` here is the compliance record for governed-edge access;
it is distinct from the operational change feed (the bus this service
consumes). They are not interchangeable — the bus is upstream input;
`audit_log` is this service's own who/what/when record.
