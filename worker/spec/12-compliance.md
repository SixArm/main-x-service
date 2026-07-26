## 12. Compliance

Posture for a worldwide public governmental deployment. Frameworks:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md),
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).

### 12.1 Frameworks and mechanisms

| Standard | Mechanism in the trio |
|---|---|
| EU / UK GDPR, UK DPA 2018 | Data-subject rights mapping (§12.2); consent model; per-field masking; data-minimising matcher (no logging, no persistence of inputs) |
| HIPAA (where the workforce is healthcare) | HIPAA-style audit trail — old/new JSON, user ID, IP, user agent, timestamp on every mutation; access tracking; soft delete |
| ISO/IEC 27001 | Operational controls: non-root containers, env-based secrets, health checks, audit query API for evidence gathering (deployment-side) |
| ISO/IEC 42001 | AI-management posture: matching is deterministic and explainable (per-field breakdowns, no black-box scoring) — every automated merge decision is reconstructible from the audit snapshot |
| HL7 FHIR R5 | Practitioner resource interoperability (service §6.8) |

### 12.2 Data-subject rights mapping (GDPR / UK DPA)

| Right | Implementation |
|---|---|
| Access (Art. 15) | `GET /api/workers/{id}/export` — full-record JSON export |
| Erasure (Art. 17) | Soft delete (`DELETE /api/workers/{id}`) + consent revocation; physical-deletion / retention policy is an open question (§16 OQ-2) |
| Rectification (Art. 16) | `PUT /api/workers/{id}` with full audit of old/new values |
| Restriction / objection | Consent model (`DataProcessing`, `DataSharing`, `Marketing`, `Research`) with `Active` / `Revoked` / `Expired` status and `has_active_consent()` checks |
| Transparency | Per-worker audit endpoint exposes who accessed/changed the record |

Gap: the front-end exposes none of these to operators yet (no
masked-view toggle, no export download, no consent UI) — tracked in
the front-end's §13 T-19/T-20 and this spec's §13.

### 12.3 Professional-licensing data considerations

Worker records carry data that is **regulated identity data plus
professional-status data**:

- Licence / credential numbers (NPI, DEA, board licence) are
  quasi-public in some jurisdictions and sensitive in others; masking
  defaults treat SSN, tax ID, DEA, and home address as sensitive
  (service §6.6). Per-jurisdiction masking policy is §16 OQ-1.
- Credential expiry and revocation have public-safety consequences;
  the credential-expiry workflow (service §13 T-7) is a compliance
  feature, not just a convenience.
- The registry records credentials but does not adjudicate them
  (§1.3); provenance MUST point at the issuing authority
  (`issuing_authority`, `assigner` fields).
- Merge errors can attach one professional's disciplinary history to
  another. Hence: deterministic short-circuits only on scheme-local
  identifiers (FR-7), human review queue between auto-merge
  thresholds, and reversible merge records with snapshots (FR-14).

### 12.4 Matcher-specific guardrails

The canonical matcher enforces privacy structurally: no IO, no
logging, no `Debug`-formatting of records into traces, synthetic
fixtures only ([matcher §20](../worker-matcher-rust-crate/spec/20-security-privacy-and-compliance.md),
[matcher `AGENTS/security-and-privacy.md`](../worker-matcher-rust-crate/AGENTS/security-and-privacy.md)).
The service MUST preserve this when bridging — worker data passed to
the matcher never gains a new egress path.

### 12.4b Tamper-evident audit history (delivered 2026-07-26)

The audit chain from [`spec/compliance` §8.5](../../spec/compliance/index.md)
step 3, ported from the care-pathway reference implementation via person
(worker's `audit_log` is identically shaped). Migration
`m20260726_000001_audit_chain` adds `seq` / `prev_hash` / `hash` /
`context` / `disclosure` / `redacted_at`; every write goes through one
chained insert under `pg_advisory_xact_lock` (HIPAA §164.312(c)).

**Why this matters more here than the table shape suggests.** §12.3
already names the hazard: a merge error can attach one professional's
**disciplinary history** to another, and credential status has
public-safety consequences. An audit trail naming practitioners is the
evidence a disciplinary or licensure process would rely on — so its
integrity is not a filing detail. The digest binds request provenance
(`user_id`, `ip_address`, `user_agent`) alongside the old/new value pair,
so *who* acted cannot be rewritten while *what* they did stays intact.

Order comes from a new `seq BIGSERIAL` rather than the primary key, which
is an application-assigned UUID and carries no insertion order.

**Read auditing wired (2026-07-26).** `disclosure::record_access` now
runs on all four read paths — `GET /api/workers/{id}`, its `/masked`
view, `GET /api/workers/search`, and the Art. 15 `/export` — so a read
of a practitioner record is accounted for, not just a mutation. Three
properties carried over from the person service deliberately:

- A read is recorded **after** record-level authorization allows it. A
  denied request disclosed nothing, and recording it would pollute the
  §164.528 accounting with accesses that never happened.
- A **search** is recorded against the nil id: it disclosed many records
  rather than one, and attributing it to any single worker would corrupt
  that worker's accounting.
- A **masked** read is recorded as `read`, the same action person uses
  for its masked view. The accounting does not currently distinguish
  masked from full; inventing a worker-only action would only put the
  two services out of step. Worth revisiting family-wide.

Read auditing is gated behind `WORKER_AUDIT_READS` (default off, as
across the family); with `WORKER_AUDIT_FAIL_CLOSED` on, a failed audit
write refuses the read with `503 AUDIT_UNAVAILABLE` rather than
disclosing data it cannot account for.

**Chain verification endpoint (2026-07-26).** `GET /api/audit/verify`
recomputes the trailing window and reports every linkage or content
break, with an `interpretation` string stating plainly that it attests
to the **audit trail** and not to the worker records.

**Open finding — audit rows written by database triggers are outside the
chain.** Migration `2024122800000005` installs `audit_workers_changes`
and `audit_organizations_changes`, `AFTER INSERT OR UPDATE OR DELETE`
triggers that `INSERT INTO audit_log` themselves. A trigger cannot
compute the chain — it has neither the application's hashing nor its
advisory lock — so those rows land with a NULL `hash`, and verification
skips them. On a representative run 11 of 20 rows were unchained. Two
consequences, both real:

1. **Coverage is partial.** Roughly half the trail is not tamper-evident,
   and the triggers duplicate events the application already records
   with full request provenance (the trigger rows have no `user_id`,
   `ip_address`, or `user_agent`).
2. **Forgery is not excluded.** Because verification tolerates unchained
   rows, an inserted row with a NULL `hash` does not register as a break.

The verification report surfaces `unchained` precisely so this is
visible rather than rounded away, and the endpoint test pins that it
keeps being reported. The **person service has the same triggers**
(`audit_patients_changes`, `audit_organizations_changes`) and the same
gap. Resolving it is a design decision, not a mechanical fix: the
triggers cover row-level changes the application does not audit
separately (a soft delete reaches the trigger as an `UPDATE` while the
application records a `DELETE`), so dropping them loses coverage, while
keeping them leaves the chain partial. Deferred to a dedicated change
across both services.

**Still open.** GDPR Art. 17 erasure by redaction and row-level record
integrity.

**Blocker cleared (2026-07-26).** Worker is now enrolled in CI's DB
suites ([`ci/db-suites.txt`](../../ci/db-suites.txt)). Three pre-existing
defects had to be fixed first, none of them from the compliance work:

- `2024122800000005` created the `pg_trgm` extension *after* the indexes
  that use `gin_trgm_ops`, and applied that operator class to
  `worker_names.given`, which is `text[]`. This was the root cause of
  everything else: the block could never apply, so the **migration chain
  stopped there** and `audit_log.seq`, `workers.worker_type`, and every
  later column simply did not exist. Fixed in place — precisely because
  it could never have applied, no deployment can have run past it.
  `given` now takes the default GIN `array_ops` index; fuzzy matching
  lives in `worker-matcher` and Tantivy, not in a SQL trigram index.
- `Worker` and `HumanName` had lost the `#[serde(default)]` attributes
  the person service still carries, so omitting an optional field —
  `prefix`, `suffix`, `identifiers`, … — was rejected with `422`. This
  was the `422`-where-`201`-expected failure. The wire contract, not the
  test, was wrong: those fields are optional in the domain and should be
  optional on the wire. Restored, removing the drift from person.
- `tests/common/mod.rs` opened a Tantivy index at a path no fixture
  created, so every integration test panicked before its first
  assertion.

### 12.5 Extended frameworks

Four frameworks impose obligations beyond §12.1. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.312(e) transmission security, §164.528 accounting of disclosures | Engaged wherever the workforce is healthcare. Two distinct angles: worker records are personal data in their own right, **and** a worker is the *actor* in every other entity's audit trail — so this registry supplies the "who" that §164.312(b) requires everywhere else. | **Read-auditing** on `get` / `list` / `search` / `export` / FHIR reads with purpose-of-use and a disclosure flag; **tamper-evident history** (a SHA-256 chain over `audit_log`) — which matters doubly here, because an audit trail naming practitioners is the evidence a disciplinary or licensure process would rely on; and a per-subject accounting of disclosures. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | Fully engaged. Worker data mixes regulated identity data with **professional-status data** (§12.3), and erasure collides with the legitimate need to retain a credential-verification history. EHDS is relevant because health-professional identity is what gates access to its primary-use exchange. | An **erasure path that survives immutable history** (redact content, keep chain linkage) — resolving §12.2's open physical-deletion question (§16 OQ-2) in favour of redaction rather than a hard purge; a declared **data residency** and **lawful basis**; and export beyond the region recorded as a **Ch. V transfer**. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | Meaningful: worker maps to **`Practitioner`**, which has a US Core profile. Modulo the family's R5-vs-R4 gap (§12.6). | **Profile and terminology validation**: US Core Practitioner must-support elements (`identifier` — notably NPI with its own system URI — and `name`) and cardinalities, with identifier systems validated against their registries rather than merely non-blank; `$validate`; SMART discovery; Bulk Data `$export`. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device, but §12.3 already names the hazard precisely: **a merge error can attach one professional's disciplinary history to another**, and a stale credential has public-safety consequences. Those are ISO 14971 harms, which is why the evidence artefacts belong here. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability**, so the scheme-local short-circuit rule (FR-7), the review queue, and the reversible merge snapshots (FR-14) cannot lose their verification silently; **signed, reproducible builds**; and a hazard-to-control trace covering the credential-expiry workflow (service §13 T-7) as a **safety** feature, not a convenience. |

### 12.6 Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; the family serves **R5**, and worker's FHIR surface
  is a prototype that is not yet mounted
  ([`agents/share/fhir.md`](../../agents/share/fhir.md) §10).
- **No hazard analysis exists.** The ISO 14971 risk file and MDCG
  2019-11 qualification are organisational artefacts; the repository
  supplies the controls they would cite.
- **Every extended control is unimplemented in this service today.** The
  reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/);
  worker is step 3 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)).
