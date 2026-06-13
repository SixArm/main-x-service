## 12. Compliance

The entity targets a worldwide public governmental deployment, so the
technology-compliance baseline of
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
applies entity-wide, plus healthcare overlays
([`compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md))
when Events carry clinical context (`EncounterId` identifiers,
encounter-type Events).

### 12.1 Personal data in this entity

Event records look impersonal but are not: **attendee and party data
is personal data.** A `Party` carries a name, optionally an email,
and optionally an external ID into the person / worker /
organization entities. Identifier values (booking numbers,
confirmation codes, ticket numbers) often double as access tokens
and are treated as sensitive. An Event's time + place + attendee
list can reveal health, political, or religious information
(special-category data under GDPR Art. 9) — e.g. attendance at a
clinic encounter or a political hearing.

### 12.2 Mechanisms

| Standard | Mechanism |
|---|---|
| GDPR Art. 5 (minimisation) | Adapter drops party emails / attendee lists from matcher projection; matcher never scores `local_id`; no party data in logs (matcher forbids `Debug`-formatting records into traces) |
| GDPR Art. 9 (special category) | Masking on by default for public-facing reads; consent records per Event (`DataProcessing`, `DataSharing`, `Marketing`, `Research`) |
| GDPR Art. 15 (access) | `GET /api/v1/events/{id}/export` |
| GDPR Art. 17 (erasure) | Soft delete + consent revocation; erasure-vs-audit-retention tension is EOQ-4 |
| GDPR Art. 30 (records of processing) | `audit_log` with old/new JSON, user ID, IP, user agent, timestamp |
| UK DPA 2018 | Same controls as GDPR rows above; UK-specific lawful-basis documentation is a deployment-side artefact |
| ISO/IEC 27001 | Operational ISMS controls (deployment-side): non-root containers, health checks, encryption at rest, CI security scans (`security.yml`) |
| ISO/IEC 42001 | AI-management scope: matching is deterministic and explainable (per-field breakdowns, FR-17, NFR-13) — no opaque model; any future ML-based scoring (service roadmap) MUST enter via an AIMS impact assessment |
| HIPAA (overlay, when PHI present) | Audit trail, access tracking, soft delete, encryption at rest |

### 12.3 Per-subproject responsibilities

- **Service** — enforcement point for masking, export, consent,
  audit. All compliance-relevant behaviour is specified in
  [service spec §12](../event-service-rust-crate/spec/12-compliance.md).
- **Matcher** — by construction cannot leak: no IO, no logging, no
  network. Test fixtures use synthetic personal data only.
- **Front-end** — must not cache or persist personal data
  client-side; masked-view toggle and GDPR-export download are open
  UI tasks (front-end §13 T-19 / T-20). Until SSO lands (ET-5), the
  UI must be deployed only on trusted operator networks.
