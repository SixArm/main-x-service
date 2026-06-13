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
