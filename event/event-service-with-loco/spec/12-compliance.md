## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA (when records carry PHI) | Audit log, access tracking, encryption-at-rest, soft delete |
| GDPR Art. 15 | `GET /api/events/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Live `Appointment` surface at `/fhir/Appointment{,/{id}}` + `/fhir/metadata` — best-effort schema.org/Event mapping (§6.8; OQ-1 resolved, §16) |
| Row-level integrity | SHA-256 + SHA3-256 digests + keyed HMAC-SHA256 MAC over each `Event` record and `audit_log` row (`src/compliance/`, the shared `integrity-mac` crate); `GET /api/records/verify` + `GET /api/audit/verify`; default off (§13, 2026-07-28) |
| ISO/IEC 27001 | Operational controls (deployment-side) |

