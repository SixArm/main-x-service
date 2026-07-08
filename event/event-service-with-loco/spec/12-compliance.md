## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA (when records carry PHI) | Audit log, access tracking, encryption-at-rest, soft delete |
| GDPR Art. 15 | `GET /api/events/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Stub; mapping pending (§6.8, OQ-1) |
| ISO/IEC 27001 | Operational controls (deployment-side) |

