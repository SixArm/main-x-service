## 12. Compliance

A worldwide public governmental deployment makes compliance a
first-class requirement, not a checklist. Frameworks:
[agents/share/compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md)
and
[agents/share/compliance-for-technology.md](../../agents/share/compliance-for-technology.md).

### 12.1 Framework → mechanism

| Standard | Mechanism (owner) |
|---|---|
| EU / UK GDPR, UK DPA 2018 | Data-subject rights mapping (§12.2); consent model; per-field masking (service) |
| HIPAA (where healthcare-relevant) | HIPAA-style audit trail — old/new JSON, user ID, IP, user agent, timestamp on every mutation; access tracking; soft delete (service) |
| UK NHS Act 2006 s.251 / Common Law Duty of Confidentiality | Audit + consent records support confidentiality governance (service); NHS-number handling is scheme-local in the matcher |
| ISO/IEC 27001 | Operational ISMS controls — deployment-side: TLS at edge, non-root containers, env-based secrets, health checks |
| ISO/IEC 42001:2023 | AIMS controls where matcher tuning becomes ML-driven (roadmap); today the matcher is deterministic and fully explainable, which is the strongest control |
| HL7 FHIR R5 | Person resource bidirectional conversion (service; partial — bundles and capability statement queued) |

### 12.2 Data-subject rights mapping (GDPR / UK DPA)

| Right | Implementation | Status |
|---|---|---|
| Access / portability (Art. 15 / 20) | `GET /api/persons/{id}/export` — full structured export | Service ✔; front-end download button queued (front-end §13 T-20) |
| Erasure (Art. 17) | Soft delete (`active = false`) + consent revocation; audit trail retained for legal-obligation carve-out | Service ✔ |
| Rectification (Art. 16) | `PUT /api/persons/{id}` with validation + audit row | Service ✔, front-end edit ✔ |
| Restriction / objection (Art. 18 / 21) | Consent model (`DataProcessing`, `DataSharing`, `Marketing`, `Research`) with `Active` / `Revoked` / `Expired`; query-layer enforcement is an open question (service §16 OQ-3) | Partial |
| Transparency (Art. 12–14) | Explainable match breakdowns; audit history per person | ✔ |

### 12.3 Privacy engineering

- Masking of sensitive fields (tax ID, document numbers, telecom,
  addresses) on demand: `mask_sensitive` search flag and
  `GET /api/persons/{id}/masked`.
- The matcher never logs and never performs IO — person data cannot
  leak from the scoring path.
- No PII in test fixtures, in any subproject.
- Front-end keeps no durable person data client-side (§10.3).

### 12.4 Gaps (tracked)

- No authentication on the API today — every endpoint is open until
  JWT verification lands (§13 E-1). This is the single largest
  compliance gap for a governmental deployment.
- Consent is recorded but not enforced in the query layer (service
  §16 OQ-3).
- Data-residency / cross-border transfer policy for a multi-region
  deployment is undecided (§16 EOQ-4).

### 12.4b Tamper-evident audit history (delivered 2026-07-26)

The audit chain from [`spec/compliance` §8.5](../../spec/compliance/index.md)
step 3, ported from the care-pathway reference implementation. Migration
`m20260726_000001_audit_chain` adds `seq` / `prev_hash` / `hash` /
`context` / `disclosure` / `redacted_at` to `audit_log`; every write
through `AuditLogRepository::log_action_on` — the single choke point all
audit writes already funnel through — is chained under
`pg_advisory_xact_lock`; `GET /api/audit/verify` reports linkage and
content breaks (HIPAA §164.312(c)).

**Ported, not copied.** Person's `audit_log` predates the loco-style
services, so two things differ from the reference:

- **Order comes from a new `seq BIGSERIAL`, not the primary key.** The PK
  is an application-assigned UUID, which carries no insertion order, and
  a chain needs a total order to mean anything. `timestamp` alone is not
  enough — two rows can share a microsecond and the tie-break would be
  arbitrary.
- **The digest binds request provenance** (`user_id`, `ip_address`,
  `user_agent`) alongside the old/new value pair, so an attacker cannot
  rewrite *who* acted while leaving *what* they did intact.

**Read/disclosure auditing (delivered).** `PERSON_AUDIT_READS`
(**default off**) audits `get` / `masked` / `search` / `export`. The
caller declares context in `X-Purpose-Of-Use` and
`X-Disclosure-Recipient` (normalised against a closed vocabulary, never
echoed), and it is persisted in the row's `context` with the §164.528
`disclosure` flag. On `GET /api/persons/{id}` the row is written **after**
the record-level authorization decision, so a denied request — which
disclosed nothing — never enters the accounting. A **masked** read is
still audited: §164.312(b) records activity, not just full disclosure.
An Art. 15 **export** is always audited, whatever the caller declared,
because it hands over the whole record. `search` is recorded against the
nil id, since it disclosed many records rather than one.
`PERSON_AUDIT_FAIL_CLOSED` (**default off**) decides whether a failed
audit write refuses the read with `503` or is logged.

**Still open.** The §164.528 *accounting endpoint* itself (`GET
/api/persons/{id}/audit/disclosures`), GDPR Art. 17 erasure by redaction,
and row-level record integrity. The rows are being recorded and are
queryable through the existing audit endpoints; the dedicated
disclosure-only view is not built.

**Blocker cleared (2026-07-26).** Person is now enrolled in CI's DB
suites ([`ci/db-suites.txt`](../../ci/db-suites.txt)): its whole
`--ignored` suite runs against Postgres on every CI run, so the audit
chain is verified end to end rather than in isolation. Four pre-existing
defects had to be fixed first, none of them from the compliance work:

- `2024122800000005` created the `pg_trgm` extension *after* the indexes
  that use `gin_trgm_ops`, and applied that operator class to
  `patient_names.given`, which is `text[]` and which `gin_trgm_ops` does
  not accept. Fixed in place — the block could never have applied, so no
  deployment can have run past it. `given` now takes the default GIN
  `array_ops` index (containment and overlap); an expression index over
  `array_to_string(given, ' ')` is impossible because Postgres requires
  index expressions to be IMMUTABLE and that function is not. No fuzzy
  matching is lost: it happens in `person-matcher` and Tantivy.
- The rename-to-`person` migration renamed the tables but not their
  `patient_id` foreign-key columns, which the SeaORM entities declare as
  `person_id`. Fixed by a **new forward migration**
  (`m20260726_000002_rename_patient_id_columns`), not by editing the
  original — that rename *can* have applied successfully, so its history
  must not be rewritten. The new migration is idempotent, guarded by
  `to_regclass` and `information_schema.columns`.
- The bulk-import advisory-lock key (SEC-B3) embedded a literal NUL as
  its field separator. Postgres `text` cannot hold one, so **every**
  identifier-keyed import row failed with `invalid byte sequence for
  encoding "UTF8": 0x00` — the lock had never worked for the stable-key
  case it exists to serialise. The boundary is now made unambiguous by
  length-prefixing, which is injective and uses no special bytes, and a
  test pins that the key is valid Postgres text.
- `tests/common/mod.rs` opened a Tantivy index at a path no fixture
  created, so every integration test panicked before its first
  assertion.

### 12.5 Extended frameworks

Four frameworks impose obligations beyond §12.1. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). Person is the
**highest-stakes adopter** in the family — it is the identity spine, so
every one of these lands harder here than anywhere else.

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.312(e) transmission security, §164.528 accounting of disclosures | **Fully engaged.** Person records are personal / special-category data and, in a health deployment, ePHI. Recording mutations only does not satisfy §164.312(b): a lookup *is* the activity that matters for an identity registry. | **Read-auditing** across `get` / `list` / `search` / `check-duplicates` / `export` / FHIR reads — the same paths SEC-G3 already flags for masking — each row carrying purpose-of-use and a disclosure flag; and **tamper-evident history**: a SHA-256 chain over `audit_log` so the trail can prove it was not rewritten, plus a per-subject accounting of disclosures answering §164.528. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | **Fully engaged.** The Art. 17 ↔ audit-retention tension noted in §12.2 is the sharpest case in the family: erasing a person must not silently destroy the accountability trail. EHDS matters because a person identity is the **linkage key** its Ch. IV secondary-use pipeline depends on. | An **erasure path that survives immutable history** — redact content, preserve chain linkage and the fact an event occurred — superseding "soft delete is erasure"; a declared **data residency** (resolving §16 EOQ-4's shape, if not a deployment's choice of region); **lawful basis + Art. 9 condition** recorded rather than assumed; and bulk export leaving the region recorded as a **Ch. V transfer** event ([`bulk-import-export.md`](../../agents/share/bulk-import-export.md) §8). |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **The most meaningful adopter.** Person maps to **`Patient`**, which — unlike most of the family's resources — *does* have a US Core profile. Conformance is genuinely achievable here, modulo the family's R5-vs-R4 gap (§12.6). | **Profile and terminology validation**: US Core Patient must-support elements (`identifier`, `name`, `gender`, `birthDate`) and cardinalities checked, with `gender` bound to `administrative-gender` and identifier systems validated rather than merely non-blank; `$validate`; SMART discovery; and Bulk Data `$export` for population-level access (which the existing bulk machinery already backs). |
| **IEC 62304 / SaMD** (with ISO 14971) | The registry is not itself a device, but a **false merge is a patient-safety hazard** — attaching one person's clinical history to another is exactly the ISO 14971 harm the standard exists to control. That hazard, not device status, is why the evidence artefacts belong here. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability** (so the deterministic short-circuit rules and merge guards cannot lose their verification silently); **signed, reproducible builds**; and a hazard-to-control trace tying the false-match controls — scheme-local short-circuits (FR-7), the review queue, reversible merge snapshots (FR-14) — to the harm they mitigate. |

### 12.6 Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; the family serves **R5**, and person's FHIR surface
  is a prototype that is not yet mounted
  ([`agents/share/fhir.md`](../../agents/share/fhir.md) §10). Profile and
  terminology validation are worth building on their own merits;
  certification is not claimed.
- **No hazard analysis exists.** The ISO 14971 risk file and the MDCG
  2019-11 qualification are organisational artefacts the operating
  organisation produces; the repository supplies the controls they cite.
- **Every extended control is unimplemented in this service today** —
  the reference implementation lives in the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/)
  and person is step 3 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)). Until it
  lands, read-auditing, chain integrity, and redaction-based erasure are
  an honest gap alongside §12.4's.
