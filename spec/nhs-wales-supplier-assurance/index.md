# NHS Wales supplier assurance

Requirements for supplying, UK-hosting and supporting this family as a
managed workforce / HR service for an NHS Wales Health Board, Trust or
Digital Health and Care Wales (DHCW). It turns the procurement brief we
were given (2026-11-20) into numbered requirements, records honestly
what the repository already provides against each, and names the gaps
as tasks in the root [`tasks.md`](../../tasks.md) Phase 15.

The supplier is the **UK GDPR data processor** and a **third-party
supplier**. NHS Wales assesses the supplier's organisation and its
hosted build, not the upstream open-source project. Most of what follows
is therefore about how the family is **deployed and operated**, and this
spec keeps the three kinds of obligation apart:

| Status | Meaning |
|---|---|
| **Met** | The code provides the control today, with the location named. |
| **Partial** | Some of it exists; the missing part is a task. |
| **Gap** | Nothing in the code does this yet; it is a task. |
| **Deployment** | The control comes from how the service is configured and hosted (§6 lists the settings). |
| **Organisational** | A contract, certificate, policy or person; the repository can supply evidence, not the control. |

Figures in this spec that come from the brief, such as the 24–48 hour
breach-notification window, the 14-day critical-patch window, and
Cyber Essentials Plus as the NHS Wales baseline under WHC (2017) 025,
are **as stated in the brief**. Confirm each against the actual tender
and contract before quoting it back to NHS Wales.

## Contents

1. [Scope](#1-scope)
2. [Governance and legal (NWS-G)](#2-governance-and-legal-nws-g)
3. [Hosting, support and off-shoring (NWS-H)](#3-hosting-support-and-off-shoring-nws-h)
4. [Application security (NWS-A)](#4-application-security-nws-a)
5. [Shared responsibility](#5-shared-responsibility)
6. [Deployment settings for an NHS Wales instance](#6-deployment-settings-for-an-nhs-wales-instance)
7. [Supplier assurance pack](#7-supplier-assurance-pack)
8. [Open questions](#8-open-questions)
9. [Cross-references](#9-cross-references)

---

## 1. Scope

**In scope:** the services an HR deployment runs, and the front ends
that call them:

- [worker-service](../../worker/worker-service-with-axum/): the
  workforce registry, including **psychometric and selection
  assessments**, the most sensitive data the family holds about staff.
- [person-service](../../person/person-service-with-axum/): the person
  identity behind each worker (`same_identity`).
- [organization-service](../../organization/organization-service-with-axum/):
  employers, departments, sites.
- [authentication-service](../../authentication/authentication-service-with-axum/):
  sign-in, sessions, PASETO tokens, ABAC attributes.
- [link-graph-service](../../link/link-graph-service-with-axum/): the
  cross-service read model (`employed_by`, `works_at`).

**What the family does not hold today: sickness, absence reasons,
occupational-health notes, disciplinary cases or pay.** The brief's
field-level requirements for those records (NWS-A4, NWS-A5) apply as
**design requirements**: any service in this repository that adds such a
field MUST meet them before the field ships. They apply equally to an
HR application deployed alongside this family that reads worker
identities through `EntityRef`.

## 2. Governance and legal (NWS-G)

| ID | Requirement | Status | What exists / what is missing |
|---|---|---|---|
| **NWS-G1** | The supplier MUST sign a UK GDPR **Article 28 data processing agreement** with the NHS Wales controller, covering processing scope, Article 9 special-category data, breach notification within the contractual window (24–48 h per the brief), and sub-processor management. | Organisational | The technical annex is §4 and §6 of this spec; the sub-processor candidates are §2.1. |
| **NWS-G2** | The supplier MUST complete the **Welsh Information Governance Toolkit (WIGTK)** / DHCW third-party supplier onboarding before go-live. | Organisational | Evidence for its technical sections: §7. |
| **NWS-G3** | The supplier MUST hold **Cyber Essentials Plus**, and SHOULD hold **ISO/IEC 27001** covering hosting and support operations. | Organisational | Neither is a property of the code. The repository supplies ISMS evidence for configuration and supply-chain controls: SBOM, `cargo-deny`, signed reproducible-build inputs (§7). |
| **NWS-G4** | The supplier MUST keep a **sub-processor register**: every external service that receives NHS data, its UK location, and its security attestation (ISO 27001 / SOC 2). | Partial | The code determines which external services *can* receive data, so the candidate list is derivable (§2.1 below). Missing: a maintained register document per deployment (NW-11). |
| **NWS-G5** | The supplier MUST confirm that the licences of the shipped software permit commercial hosted delivery, and that hosting creates no obligation to disclose NHS configuration or data. | Partial | Every published crate declares `MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only OR OGL-UK-3.0 OR CC-BY-NC-SA-4.0`; every front end `MIT OR Apache-2.0`. None is AGPL, so running the service over a network creates no source-disclosure duty, and the supplier may choose a permissive licence for distribution. Rust dependencies are gated by the `cargo-deny` licence allow-list in each `deny.toml`. **Missing:** front-end npm dependencies (including the Lily packages) are not licence-checked by any CI stage (NW-10). |

### 2.1 Candidate sub-processors (NWS-G4)

Each of these is an external service the code sends data to when its
setting is configured. Each that a deployment uses is a sub-processor
and MUST be UK-hosted (NWS-H1).

| Service | Setting | Data it receives |
|---|---|---|
| PostgreSQL host | `DB_URL` | everything |
| Object store (bulk artifacts) | `<P>_BULK_ARTIFACT_BACKEND=s3`, `<P>_BULK_S3_*` | full exports and imports of records |
| SMTP relay | `SMTP_*` (authentication-service) | staff email addresses, magic-link URLs |
| Identity provider | `AUTH_OIDC_ISSUER_URL` | sign-in identity and claims |
| Event broker | `<P>_FLUVIO_ENDPOINT` | every change event, including record content |
| OpenTelemetry collector | `OTLP_ENDPOINT` | request spans, which can carry identifiers |
| Off-box checkpoint store | per the [integrity runbook](../../agents/share/runbooks/integrity-activation.md) | audit-chain checkpoints (digests, no record content) |

## 3. Hosting, support and off-shoring (NWS-H)

| ID | Requirement | Status | What exists / what is missing |
|---|---|---|---|
| **NWS-H1** | Production, file attachments, backups and disaster-recovery replicas MUST be in **UK-only** regions, and every sub-processor in §2.1 MUST be UK-located. | Deployment + Partial | Hosting region is a deployment choice. care-pathway declares a residency region (`CARE_PATHWAY_DATA_RESIDENCY`, surfaced at `GET /api/compliance`) and audits an export leaving it as a transfer. worker and person declare it too since 2026-11-20 (`WORKER_DATA_RESIDENCY`, `PERSON_DATA_RESIDENCY`, NW-6), with the Art. 6 / Art. 9 declarations alongside. **Missing:** organization, authentication-service and link-graph declare no region. |
| **NWS-H2** | Staff with administrative or support access to production MUST be **UK-based** and vetted to **BPSS** or **DBS**. | Organisational | — |
| **NWS-H3** | Support access MUST be **just-in-time**, MFA-protected, recorded and customer-approved (or done by screen-share with an NHS Wales administrator). No standing vendor access to live sensitive records. | Gap | Admin rights are the ABAC attribute `access=admin`, assigned through the `user_attributes` CLI task or the admin API and kept **until removed**: there is no expiry, no approval step and no break-glass record. Attribute changes are audited (`attributes_assigned`) and revoke the user's sessions, which is the base to build on (NW-4). Shell access to hosts, which can run the CLI task, is a deployment control. |
| **NWS-H4** | The NHS Wales instance MUST be **single-tenant**: its databases and storage are never shared with another customer. | Met by construction + Deployment | No service has a tenant column or multi-tenant mode: each deployment is one customer, with one database per service. The requirement on the deployment is not to put two customers' databases on one Postgres server or one bucket. |

## 4. Application security (NWS-A)

| ID | Requirement | Status | What exists / what is missing |
|---|---|---|---|
| **NWS-A1** | An annual **CHECK- or CREST-accredited** penetration test (IT Health Check) of the hosted build MUST cover the application, its APIs and the cloud infrastructure, including **role segregation** between employees, line managers, HR, payroll and supplier support admins. Critical and High findings MUST be remediated, with evidence. | Organisational | None has been commissioned. The repository's own assurance is the [2026-07-12 security audit](../../agents/share/security.md) (every SEC-* item closed), `cargo-fuzz` targets, and `proptest` never-panic properties. A test brief naming the role matrix is NW-12. |
| **NWS-A2** | A documented **patch SLA** MUST cover upstream CVEs (critical within 14 days per the brief), backed by automated dependency and **container image** scanning. | Partial | `cargo-deny` advisory checks run in CI on every crate; Dependabot is configured ([spec](../dependabot/index.md)); the SBOM renders per release. **Missing:** a written SLA with a record of time-to-patch (NW-9), and any scan of the built container images, whose Debian base layer `cargo-deny` never sees (NW-8). As of 2026-11-20 the `deny` stage is red on RUSTSEC-2026-0543 (`chrono` unmaintained) in most services and RUSTSEC-2026-0555 in event-service, which is the gate working and the SLA clock running. |
| **NWS-A3** | Sign-in MUST be federated to **NHS Wales Microsoft Entra ID / NADEX** (OIDC or SAML 2.0) **with MFA**; local login MUST be disabled; there MUST be no shared or built-in vendor accounts. | Partial | **OIDC:** delivered (authentication-service `oidc` feature, `AUTH_OIDC_*`; Entra ID is an OIDC provider), with just-in-time provisioning off by default. **SAML 2.0:** deliberately deferred, reasons in [authentication-sessions.md §7a](../../agents/share/authentication-sessions.md); OIDC covers Entra ID. **No built-in accounts:** none exist, and the development signing seed is refused in production (SEC-A1). **Local login:** can be turned off (`AUTH_MAGIC_LINK_ENABLED=false`, NW-1, done 2026-11-20); the sign-in front end hides its email form to match, but the other front ends' own sign-in forms do not yet (NW-1b). **Missing:** nothing checks that the IdP **performed MFA** (the ID token's `amr` / `acr`), so MFA rests entirely on Entra conditional-access policy (NW-2). |
| **NWS-A4** | Access MUST be separated **by field group**, not only by record: line managers see absence dates but not reasons or occupational-health notes; HR and occupational health see reasons and disciplinary files only for their assigned cases or department; payroll sees pay and grade but not disciplinary material. | Partial | The ABAC engine evaluates subject, record and environment attributes and carries obligations (`authentication-verifier`, [authorization-attributes.md](../../agents/share/authorization-attributes.md)); worker applies record-level decisions and the `mask` obligation on every read path (SEC-G3), including its assessments. Department or case scoping is expressible today as `resource.*` policy. **Missing:** `mask` is one all-or-nothing redaction. There is no way for a policy to say "show this record with field group X hidden and Y shown", which the manager / HR / payroll split needs (NW-3). Any sickness, disciplinary or pay field added later MUST be assigned to a field group before it ships. |
| **NWS-A5** | **TLS 1.2+** in transit; **AES-256 at rest**; **application- or column-level encryption** for special-category fields (sickness, disciplinary, and here also psychometric assessments). | Partial | **Database TLS:** production refuses a `DB_URL` without `sslmode` of `require` or stricter. **HTTP TLS:** terminated at the ingress (Deployment), which MUST refuse below TLS 1.2. **At rest:** volume encryption by the host (Deployment). **Column-level:** none: `pgcrypto` is installed but no service encrypts a field, so a database dump or a support engineer with SQL access reads assessment results in clear (NW-5). |
| **NWS-A6** | **Tamper-evident audit logs** MUST record every read, create, update and export of sensitive records (user, time, IP, action), be retained per NHS standards, and be exportable to the customer's **SIEM**. | Partial | **Met in code for worker and person:** SHA-256 hash chain over `audit_log` with `/api/audit/verify`, off-box signed checkpoints, read and disclosure auditing with purpose of use, the §164.528 disclosure accounting, keyed MACs. **Deployment:** read auditing is **off by default** and MUST be enabled, fail-closed (§6). **Conflict to resolve:** the caller's address is removed from audit rows after `WORKER_AUDIT_ADDRESS_RETENTION_DAYS` (default 7) under GDPR minimisation; the brief wants IP on the log, so the controller must choose the retention (§8). **Missing:** no push to a SIEM. Audit is pull-only (`GET /api/audit/recent`, `/api/audit/user`, `/api/workers/{id}/audit`), with no syslog, CEF or OTLP-logs export (NW-7); and no retention period or purge for audit rows themselves (NW-13). |
| **NWS-A7** | A tested **24/7 incident response** plan, with notification to NHS Wales inside the contractual window. | Organisational | The [runbooks](../../agents/share/runbooks/) cover technical response for specific failures (key rotation, bus outage, reconciliation divergence, bulk-job recovery); they are inputs to, not a substitute for, the plan. |
| **NWS-A8** | **Retention rules** MUST purge records automatically per the **NHS Wales Records Management Code of Practice**, configurable by the controller. | Gap | Only erasure on request (`POST /api/workers/{id}/erase`, Art. 17 by redaction, the chain still verifying) and audit-address retention exist. There is no scheduled purge by record age or status (NW-13). |
| **NWS-A9** | At contract end, **all records** MUST be exportable in a structured format (CSV / JSON / SQL), followed by **certified cryptographic erasure**. | Partial | **person:** bulk export in JSONL, CSV and Parquet. **organization:** JSONL, CSV and TSV. **worker:** only the single-record GDPR export (`GET /api/workers/{id}/export`), so a whole-workforce exit export is a gap (NW-14). A `pg_dump` meets "SQL" but is not a supported interchange format. **Cryptographic erasure:** no service encrypts under a per-deployment data key that could be destroyed, so erasure today means destroying volumes, snapshots and backups, attested by the supplier and the cloud provider (NW-5 makes cryptographic erasure possible for the encrypted fields). |

## 5. Shared responsibility

| Control area | Supplier | NHS Wales controller |
|---|---|---|
| Controller duties and DPIA | Supplies the architecture, this spec, and the evidence pack (§7) as processor input to the DPIA. | Is the UK GDPR controller; completes and signs off the DPIA (DPO and SIRO). |
| UK cloud and backups | Provisions, hardens, monitors and backs up the UK environment; manages cloud sub-processors (NWS-G4, NWS-H1). | Approves the hosting location; reviews supplier and cloud assurance through WIGTK. |
| Code and patching | Tracks upstream advisories, builds and patches releases within the SLA (NWS-A2), commissions the annual pen test (NWS-A1). | Reviews pen-test summaries; approves maintenance windows. |
| Identity and provisioning | Maintains the OIDC integration, claim mapping (`AUTH_OIDC_CLAIM_MAP`), attribute vocabulary and ABAC policy templates (NWS-A3, NWS-A4). | Manages accounts, MFA and leaver revocation in Entra ID / NADEX; assigns HR, manager and payroll roles through the groups the claim map reads. |
| Sensitive-field governance | Enforces ABAC, field groups, field encryption and export restrictions in the application. | Decides who may see sickness, disciplinary, pay and assessment fields; reviews access through the audit and disclosure reports. |
| Incident response | Monitors 24/7, contains, and notifies NHS Wales within the contractual window (NWS-A7). | Notifies the ICO and Welsh Government; communicates with staff. |
| Retention and exit | Provides the retention and purge mechanism (NWS-A8) and runs the exit export and certified destruction (NWS-A9). | Sets retention schedules; verifies the exported data at exit. |

## 6. Deployment settings for an NHS Wales instance

These are the settings an NHS Wales deployment MUST have. Every one of
them defaults to the permissive value, so an instance missing one is
silently out of conformance; NW-15 adds a boot-time check. `<P>` is the
service prefix (`WORKER`, `PERSON`, `ORGANIZATION`, `LINK_GRAPH`); see
[configuration.md](../../agents/share/configuration.md) for each. A `<P>_` setting applies where that service reads it: audit reads and address retention are person and worker only, bulk S3 is person only (organization stores bulk artifacts locally), and the event transport is not a link-graph setting.

| Setting | Value | Why |
|---|---|---|
| `APP_ENV` | `production` | Enables the production refusals (dev seed, insecure DB). |
| `<P>_REQUIRE_AUTH` | `true` | Authentication and ABAC are **off** without it ([security.md §4](../../agents/share/security.md)). |
| `<P>_ABAC_POLICY_FILE` | the NHS Wales policy | The default policy grants every authenticated reader everything readable. |
| `<P>_PASETO_KEYS_URL` | the authentication-service key set, over HTTPS | Offline token verification. |
| `<P>_AUDIT_READS`, `<P>_AUDIT_FAIL_CLOSED` | `true`, `true` | Reads are audited, and a read whose audit row cannot be written is refused (NWS-A6). |
| `<P>_DATA_RESIDENCY`, `<P>_LAWFUL_BASIS`, `<P>_ART9_CONDITION` | `uk`; the controller's basis and Art. 9(2) condition | Recorded on every audited access; an export to a non-UK region is marked a transfer (NWS-H1). Person and worker. |
| `<P>_AUDIT_ADDRESS_RETENTION_DAYS` | the controller's decision | See §8. |
| `<P>_INTEGRITY_MAC_KEY_FILE` | a key from the secrets manager | Without it, row MACs are off and a database-level edit is undetectable. |
| `<P>_EVENT_TRANSPORT` | `outbox` | No committed change without its event. |
| `DB_URL` | `sslmode=verify-full`, UK host | NWS-A5, NWS-H1. |
| `TRUSTED_PROXIES` | the ingress addresses | Otherwise every audit row records the proxy's address. |
| `RATE_LIMIT_PER_MINUTE`, `LOCKOUT_AFTER_FAILURES` | set | Both are off by default. |
| `OTLP_ENDPOINT` | a UK collector, or unset | Spans leave the instance (§2.1). |
| `<P>_BULK_S3_REGION` | a UK region | Exports leave the database (§2.1). |
| `TOKEN_PRIVATE_KEY_FILE` | from the secrets manager | SEC-A1. |
| `AUTH_OIDC_ISSUER_URL`, `AUTH_OIDC_CLIENT_ID`, `AUTH_OIDC_CLIENT_SECRET_FILE`, `AUTH_OIDC_REDIRECT_URL`, `AUTH_OIDC_CLAIM_MAP` | the NHS Wales Entra ID tenant | NWS-A3. |
| `AUTH_MAGIC_LINK_ENABLED` | `false` | No local login (NWS-A3); boot refuses it without a complete OIDC configuration. Front ends: `PUBLIC_MAGIC_LINK_SIGNIN_ENABLED=false`, `PUBLIC_OIDC_SIGNIN_ENABLED=true`. |
| `AUTH_OIDC_JIT_PROVISIONING` | `false` unless the controller decides otherwise | Default off; an unknown staff member is refused and audited. |
| `AUTH_ATTRIBUTE_VOCABULARY_FILE` | the NHS Wales role vocabulary | A mistyped role is rejected rather than silently granting nothing. |

## 7. Supplier assurance pack

What the repository contributes to the pack. Certificates, the DPA,
staff vetting records and the incident plan come from the supplier's
organisation.

| Pack item | Source in this repository |
|---|---|
| Architecture and data-flow description | [spec/architecture](../architecture/index.md), [spec/dataflow](../dataflow/index.md); a UK data-flow and tenant-isolation diagram for the specific deployment is NW-11 |
| Security controls and threat model | [agents/share/security.md](../../agents/share/security.md) |
| Regulatory control map | [spec/compliance](../compliance/index.md) |
| This requirement-by-requirement status | this spec, §2–§4 |
| Software bill of materials | the `evidence` CI stage's rendered SBOM artefact, per release |
| Dependency advisory and licence policy | each crate's `deny.toml`; the `deny` CI stage |
| Configuration reference | [agents/share/configuration.md](../../agents/share/configuration.md) |
| Operational runbooks | [agents/share/runbooks/](../../agents/share/runbooks/) |
| Sub-processor candidates | §2.1 |

## 8. Open questions

- **Caller address on audit rows.** GDPR minimisation removes it after
  `<P>_AUDIT_ADDRESS_RETENTION_DAYS` (default 7); the brief asks for IP
  on the log, retained per NHS standards. This is the controller's
  decision to record in the DPIA. The supplier's task is only that both
  answers are configurable, which they are.
- **SAML 2.0.** Deferred for want of a vetted pure-Rust implementation
  ([authentication-sessions.md §7a](../../agents/share/authentication-sessions.md)).
  If a tender insists on SAML rather than accepting Entra ID over OIDC,
  that decision reopens as its own security review.
- **Where sickness, disciplinary and pay records live.** Either a new
  service in this family (which then carries NWS-A4/A5 from its first
  field) or a separate HR application reading worker identities. Not
  decided here.
- **Which figures are contractual.** The 24–48 h breach window and the
  14-day critical-patch window are the brief's; the contract sets them.

## 9. Cross-references

- [spec/compliance](../compliance/index.md) — the family-wide obligation → control map; this spec is its NHS Wales supplier view.
- [agents/share/compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md), [compliance-for-technology.md](../../agents/share/compliance-for-technology.md)
- [agents/share/security.md](../../agents/share/security.md) — invariants and the activation gate
- [agents/share/authorization-attributes.md](../../agents/share/authorization-attributes.md) — ABAC, obligations
- [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md) — sessions, PASETO, OIDC federation
- [agents/share/bulk-import-export.md](../../agents/share/bulk-import-export.md) — the exit-export mechanism
- [agents/share/privacy.md](../../agents/share/privacy.md) — masking and erasure
- root [`tasks.md`](../../tasks.md) Phase 15 — the NW-* tasks
