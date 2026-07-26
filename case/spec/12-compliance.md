## 12. Compliance

The Main X Index targets worldwide public governmental systems, and
**case data is personal data**: a case concerns an identified or
identifiable person or organisation. Privacy and compliance therefore
matter *more* for this entity than for most siblings (a pathway or a
place definition is reference data; a case is about someone). Family
frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
and
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
(the latter applies to healthcare / social-care cases).

### 12.1 Data classification

Case records are **personal data**, and some are **special-category**
(health, social-services, immigration, criminal-matter cases). Three
facets:

- **The record itself is personal data.** Title, case type, status,
  agency, opened date, and the involved `subjects` together identify a
  matter about a person — even though `subjects` are opaque references,
  re-identification is trivial for the holding agency. This is GDPR /
  UK DPA personal data; some cases engage GDPR Art. 9 special
  categories.
- **Audit trails are personal data too.** Who created, edited,
  reviewed, or merged a record is personal data about operators; the
  audit log (delivered) MUST be governed accordingly.
- **Free text can leak.** `keywords` and `alternate_titles` MUST never
  carry substantive case content or personal detail; `subjects` carry
  only opaque ids. This is a shared invariant (§5.5) and an
  operator-training point.

### 12.2 Frameworks

| Framework | Application to this entity |
|---|---|
| UK DPA 2018 / UK GDPR / EU GDPR | Fully engaged — case records are personal data. Lawful basis (usually public task / legal obligation), data-subject rights (access, rectification, erasure), and accountability documentation are mandatory. Soft delete supports retention policy; a GDPR-erasure path on top of it is required (§13 T-10). |
| US HIPAA | Engaged for healthcare / social-care cases that touch PHI; HIPAA-grade audit trails (delivered) and access controls required; soft delete preserves history. |
| UK Common Law Duty of Confidentiality | Engaged for cases holding confidential personal information (health, social care). |
| ISO/IEC 27001 | ISMS operational controls (deployment-side): access control, encryption at rest, backups, logging. |
| ISO/IEC 42001:2023 | AIMS controls if matcher weights/thresholds are ever ML-tuned (today they are hand-set constants). |

### 12.3 Information-governance posture

- **Minimisation by design.** `subjects` are opaque references, not
  personal detail; the registry holds identity/routing metadata, not
  the case file (§1.3). This shrinks but does **not** remove the
  personal-data footprint — the record is still about a person.
- **Privacy controls are a priority gap.** Per-field masking (for the
  masked-view endpoint) and GDPR data-subject export are **not yet
  built** and are higher-priority here than for any sibling — tracked
  as §13 T-10 and roadmap §15. Until they land, masking/export are an
  honest gap (§14), and deployments must mitigate operationally
  (access control, need-to-know, data-protection impact assessment).
- **Auditability.** Delivered: soft delete + durable `audit_logs` row
  per create/update/delete/merge + in-memory event stream, per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
  The remaining gap is a durable cross-replica event bus (roadmap §15).
- **Access control.** Production deployments MUST sit behind SSO
  (central authentication entity, PASETO v4 public token verification —
  delivered for `whoami` / `actor`; *blanket `/api/*` enforcement is
  roadmap*; see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  supersedes RS256 JWT) and TLS; writes are restricted to caseworkers /
  registry operators.
- **Explainability for accountability.** Per-component match breakdowns
  give auditors a replayable rationale for every duplicate / merge
  decision — keep this property (NFR-9).

### 12.4 Extended frameworks

Four frameworks impose obligations beyond §12.2. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). Because a case
**is about a person** (§12.1), these land here about as hard as they do
on person itself — and harder than on any other sibling, given the
`case ↔ person` edge's elevated governance
([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
§10).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.528 accounting of disclosures | Engaged for healthcare and social-care cases touching PHI. The decisive point: **learning that a case exists is itself the disclosure**, so a read must be audited, not just a write — and the `subject_of` edge's concealment rule (§10 of the linking doc) is exactly a §164.528 disclosure boundary. | **Read-auditing** on `get` / `list` / `search` / `check-duplicates` / `export` / FHIR reads and on **every traversal that surfaces a `subject_of` edge**, each row carrying purpose-of-use and a disclosure flag; **tamper-evident history** (a SHA-256 chain over `audit_logs`), which matters acutely because a case audit trail is potential legal evidence; and a per-record accounting of disclosures. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | **Fully engaged**, including Art. 9 special category for health, social-services, immigration, and criminal-matter cases. EHDS engages for the health and social-care subset, whose case data is exactly what its Ch. IV secondary-use pipeline would seek. | An **erasure path that survives immutable history** — redact the content, keep the chain linkage — which is the concrete shape of the §13 T-10 GDPR-erasure task, and which must extend to the `entity_links` rows and their `linked`/`unlinked` events, not just the case row; a declared **data residency** and **lawful basis** (usually public task or legal obligation); an `X-Purpose-Of-Use` marker; and export beyond the region recorded as a **Ch. V transfer**. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **Weak.** Case maps to `Task` — a best-effort mapping with no US Core profile ([`agents/share/fhir.md`](../../agents/share/fhir.md) §3). There is no certification target here. | The conformance *machinery* only: a declared `meta.profile`, structural validation, **terminology validation** of `status` / `intent` / `priority` against their bound value sets, and `$validate`. Bulk Data `$export` is available but, on this entity, is a **mass disclosure of personal data** and must inherit the §8 masking and audit rules of [`bulk-import-export.md`](../../agents/share/bulk-import-export.md) — plus the `subject` reference concealment. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device. The engagement is **supply-chain and configuration evidence**, plus one real harm: a false merge attaches one person's case history to another — a consequential error in benefits, immigration, or criminal-matter contexts. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability**, notably over the deterministic short-circuit rules and the record-level ABAC / masking-obligation paths, where a silent regression is a disclosure; and **signed, reproducible builds**. |

### 12.4b GDPR Art. 17 erasure (delivered 2026-07-26)

`POST /api/cases/{pid}/erase` destroys the case's personal data and
appends a chained `erased` accountability row. It is a **destructive**
action under ABAC ([`crate::auth::DESTRUCTIVE_POST_SUFFIXES`]), so it
requires `access=admin` — and it is **not** the soft delete: `DELETE
/{pid}` retires a record and keeps its data, this destroys the data and is
irreversible. The response says `irreversible: true` so a caller cannot
confuse the two.

The collision this resolves is real: honouring Art. 17 by deleting audit
rows would destroy the §164.312(c) integrity the chain exists to provide,
and refusing the erasure to protect the chain would breach Art. 17.
**Redaction** satisfies both — each audit row's snapshot is destroyed and
`redacted_at` stamped while its `hash` and `prev_hash` are left intact, so
verification still checks linkage across it and the chain as a whole keeps
verifying. What survives is the *fact* that a case existed and was erased,
by whom and when: the controller's own accountability record under the
Art. 17(3)(b) carve-out, holding nothing about the subject.

**The cross-service links are withdrawn too**, which is what makes this
meaningful for a case rather than merely correct. A `subject_of` edge
asserts that a named person is the subject of a benefits, legal, or
investigative proceeding — the family's highest-governance link
([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
§10). Tombstoning the payload while leaving that edge standing would erase
the details and keep the accusation, the opposite of what the subject
asked for. The links are **soft**-deleted rather than dropped: the link
aggregator reconciles against this table, and a row that vanishes without
trace is indistinguishable from one that was never written, which would
let a dropped event resurrect the edge.

Erasing an unknown or already-erased pid is answered, not refused. A
subject's right does not lapse once the record is soft-deleted — the audit
content held about it is still personal data — and a `404` would confirm
to a prober which pids are unknown.

DB-gated tests pin the load-bearing property: after an erasure the chain
still verifies and the redactions are *counted*, not hidden. If that ever
fails, the two obligations have stopped being simultaneously satisfiable
and the design is broken, not the test.

### 12.5 Honest limits

- **Masking and GDPR export are still not built** (§12.3), and that gap
  outranks everything in the table above. Read-auditing tells you a
  disclosure happened; masking is what stops the wrong one. Land §13
  T-10 first.
- **Not a certified health-IT module,** and not a candidate: `Task` is a
  best-effort mapping and the family serves **R5** against a
  certification targeting **R4 + US Core**.
- **Bulk export is the sharpest new risk.** Adding `$export` to an
  entity whose every row is personal data is only safe behind the
  masking profile, the elevated-authorisation gate, and the per-export
  audit that [`bulk-import-export.md`](../../agents/share/bulk-import-export.md)
  §8 requires. Do not ship it before those.
- **The audit half of the extended controls is implemented; the rest is
  not.** Delivered: the tamper-evident chain, read/disclosure auditing,
  `/audit/verify`, the §164.528 accounting, and (2026-07-26) Art. 17
  erasure by redaction including link withdrawal (§12.4b). Still absent:
  the GDPR residency and lawful-basis declarations, FHIR profile and
  terminology validation, and the SOUP/SBOM evidence bundle. The
  reference implementation remains the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/);
  case is step 3 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)).
