# Compliance for healthcare

The regimes the Main X Index family targets wherever an entity carries
clinical, patient-linked, or care-delivery data. §1 is the baseline list;
§2 expands the four frameworks that actually **drive technical controls**
in the code (rather than being satisfied by deployment or organisational
process); §3 states what each one obliges us to build.

## 1. Baseline regimes

- United States (US) Health Insurance Portability and Accountability Act (HIPAA)
- United Kingdom (UK) Data Protection Act (DPA) 2018
- United Kingdom (UK) Common Law Duty of Confidentiality
- United Kingdom (UK) National Health Service (NHS) Act 2006, Section 251
- United Kingdom (UK) General Data Protection Regulation (GDPR)
- European Union (EU) General Data Protection Regulation (GDPR)
- European Union (EU) European Health Data Space (EHDS) — Regulation (EU) 2025/327
- United States (US) ASTP/ONC Health IT Certification Program — 45 CFR Part 170 (HTI rules)
- IEC 62304 — medical device software life-cycle processes (with ISO 14971, IEC 82304-1)

## 2. The four control-driving frameworks

### 2.1 HIPAA (US) — access & disclosure logging, integrity, transmission security

The HIPAA Security Rule (45 CFR Part 164, Subpart C) is the family's
reference for **what a health-grade audit trail must be**. The provisions
that translate into code:

| Provision | Obligation |
|---|---|
| §164.312(b) **Audit controls** | Record and examine activity in systems that hold ePHI. Recording *mutations only* does not satisfy this — **reads are activity**. |
| §164.308(a)(1)(ii)(D) **Information system activity review** | Regularly review audit logs and access reports; the trail must therefore be queryable, not just written. |
| §164.312(c)(1)–(2) **Integrity** | Protect ePHI from improper alteration or destruction, and provide a mechanism to **corroborate that data has not been altered**. |
| §164.312(e)(1)–(2) **Transmission security** | Integrity controls and encryption for ePHI in transit. |
| §164.312(a)(1), (d) **Access control / person authentication** | Unique user identification and verification of the entity seeking access. |
| §164.528 **Accounting of disclosures** | An individual may request an accounting of disclosures of their PHI (6-year window); the trail must distinguish an internal **access** from an outward **disclosure**. |

**What it drives:** **read-auditing** — an audit row for reads, searches,
exports and FHIR fetches, not only for writes — and **tamper-evident
history**: the audit trail must be able to prove it has not been rewritten
(a hash chain over the rows), and disclosures must be separable from
ordinary access so §164.528 can be answered from the data.

### 2.2 GDPR / EU EHDS — erasure vs. immutable history, residency, lawful basis

GDPR (EU 2016/679 and the UK retained form, with the DPA 2018) and the
**European Health Data Space** Regulation (EU) 2025/327 together govern
the data-protection posture. EHDS is the newer half and is health-specific:
it splits **primary use** (care delivery, EHR exchange, European Electronic
Health Record systems) from **secondary use** (research, policy, statistics,
regulatory activity), routed through health-data access bodies, data
permits, and **secure processing environments**. EHDS applies in stages
after entry into force; a deployment must confirm which chapter is live for
its member state and use case rather than assuming the whole regulation
applies at once.

| Provision | Obligation |
|---|---|
| GDPR Art. 5–6, Art. 9 | Purpose limitation and minimisation; a recorded **lawful basis** (usually public task or legal obligation for a public health registry) and an Art. 9(2) condition for health data. |
| GDPR Art. 15 / 20 | Right of access and portability — a structured export of the subject's data. |
| GDPR Art. 17 | Right to erasure — which **collides with an append-only, tamper-evident audit trail**; the collision must be resolved explicitly, not by ignoring one side. |
| GDPR Art. 30 | Records of processing activities. |
| GDPR Art. 32 | Security of processing — encryption, integrity, availability, restoration. |
| GDPR Ch. V (Art. 44–49) | **Cross-border transfer** — adequacy, safeguards, derogations; every export leaving the region is a transfer event. |
| EHDS Ch. II / IV | Primary vs. secondary use, data-holder duties, data permits, **secure processing environments** and their location constraints; a data-quality and utility label for secondary-use datasets. |

**What it drives:** an **erasure path that survives immutable history** —
redact the *content* while preserving the chain linkage and the fact that a
row existed, so the trail stays verifiable and the subject's data is gone;
a declared **data residency** for the deployment, surfaced so that an export
crossing it is visible; **lawful-basis and consent records** attached to
processing rather than assumed; and a **cross-border transfer** posture that
an operator can read off the running service.

### 2.3 ONC / HTI certification (US) — profile and terminology conformance

The ASTP/ONC Health IT Certification Program (45 CFR Part 170; the HTI
rulemakings) is what a US health-IT module is certified against. The
criterion that shapes an API surface is **§170.315(g)(10) Standardized API
for patient and population services**, which requires:

- **HL7 FHIR** with **US Core** implementation-guide conformance — a
  resource is not merely well-formed FHIR, it must satisfy the profile's
  **must-support elements, cardinalities, and terminology bindings**.
- **SMART App Launch** — OAuth 2.0 authorisation with scoped access, and a
  discoverable `/.well-known/smart-configuration` document.
- **FHIR Bulk Data Access (Flat FHIR)** — the asynchronous `$export`
  operation returning NDJSON, for population-level access.
- Certified modules are tested with **Inferno**, ONC's open-source FHIR test
  kit, which exercises exactly those conformance points.

Certification also brings the privacy/security criteria — notably
§170.315(d)(2)/(d)(3)/(d)(10) on **auditable events, tamper-resistance and
audit reports** — which land on the same audit machinery as §2.1.

**Honest scope note.** Certification targets **FHIR R4 + US Core**, while
this family serves **FHIR R5** (see [`fhir.md`](fhir.md)), and several of
our resources (`PlanDefinition`, `Task`, `Basic`, `Appointment`) have no US
Core profile at all. No crate here is a certifiable EHR module and none
claims to be. What genuinely transfers is the **conformance machinery**:
declaring the profile a resource claims, validating against it, validating
codes against their bound value sets, exposing SMART discovery, and serving
Bulk Data — each of which is worth building on its own merits and is what
makes an Inferno-style suite runnable against us later.

**What it drives:** **profile validation** (a declared profile URL plus
must-support and cardinality checks, surfaced as `OperationOutcome`
issues), **terminology validation** (codes checked against the value set
their element is bound to — ICD-10 / ICD-11 / SNOMED CT / LOINC — rather
than merely being non-blank), a **`$validate` operation**, **SMART
discovery**, and **Bulk Data `$export`**.

### 2.4 IEC 62304 / SaMD — lifecycle, SBOM, traceability, reproducible builds

IEC 62304:2006+A1:2015 governs the **life cycle of medical device
software**, alongside ISO 14971 (risk management), IEC 82304-1 (health
software products), and IEC 62366-1 (usability). It classifies software by
the harm a failure could cause — **Class A** (no injury possible), **Class
B** (non-serious injury), **Class C** (death or serious injury) — and scales
process rigour accordingly. Regulatory context: EU MDR (Regulation (EU)
2017/745) **Rule 11** puts software that informs diagnostic or therapeutic
decisions in Class IIa or above (MDCG 2019-11 for qualification); the UK
route is UKCA / MHRA plus NHS **DTAC**; the US route is the FDA's SaMD and
Clinical Decision Support framing, and since FD&C Act §524B a **software
bill of materials is a premarket requirement** for cyber devices.

The clauses that translate into code and repository artefacts:

| Clause | Obligation |
|---|---|
| §5.1 Software development planning | A written plan, and a declared **safety classification** the plan is scaled to. |
| §5.2–5.5 Requirements → architecture → unit verification | Requirements are identified, and **each is verified**. |
| §5.3.3, §8.1.2 **SOUP** | Every piece of *Software Of Unknown Provenance* (i.e. every third-party dependency) is listed with its identity, version, and purpose. |
| §7 Risk management (with ISO 14971) | Hazards traced to software items and to the controls that mitigate them. |
| §8 Configuration management | The exact configuration of a release is identifiable and **reconstructible**. |
| §9 Problem resolution | Defects are recorded, analysed, and traced to their fix and its verification. |

**Qualification caveat.** A registry of pathway *templates*, or of person /
place / organization identities, is generally **not** a medical device: it
does not itself drive an individual patient's treatment. The line is crossed
when a deployment surfaces the data as clinical decision support, or when a
service tracks an individual patient's progress through a pathway. Each
entity spec states where its own line sits; we build the evidence artefacts
regardless, because they are good engineering practice and because they are
what a device-qualification assessment would otherwise have to reconstruct
after the fact.

**What it drives:** a **SOUP register + SBOM** (CycloneDX/SPDX) generated
from the real dependency graph, with supply-chain gating (`cargo-deny`);
**traceability from requirement to test**, machine-checked so a requirement
cannot silently lose its verification; **signed and reproducible builds**
(pinned toolchain, `SOURCE_DATE_EPOCH`, recorded commit and build inputs) so
a released binary can be tied back to its source; and a runtime **software
identification** surface stating version, build provenance, safety
classification, and which controls are active.

## 3. Where the controls live

| Framework | Primary control home |
|---|---|
| HIPAA audit / integrity | [`auditability.md`](auditability.md), [`security.md`](security.md) |
| HIPAA access control | [`authentication-sessions.md`](authentication-sessions.md), [`authorization-attributes.md`](authorization-attributes.md) |
| GDPR / EHDS rights, masking, erasure | [`privacy.md`](privacy.md), [`bulk-import-export.md`](bulk-import-export.md) |
| ONC / HTI conformance | [`fhir.md`](fhir.md) |
| IEC 62304 evidence | [`security.md`](security.md) (supply chain), each crate's `spec/` §11 testing and §13 tasks |

The **reference implementation** of all four in code is the
[care-pathway service](../../care-pathway/care-pathway-service-with-loco/)
(`src/compliance/`, `src/fhir/profile.rs`, `compliance/`), which the other
services copy-adapt. See also
[`compliance-for-technology.md`](compliance-for-technology.md) for the
non-clinical regimes and the monorepo-wide
[`spec/compliance`](../../spec/compliance/index.md) for the honest
obligation → control → location map.
