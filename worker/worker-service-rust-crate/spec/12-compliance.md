## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest, soft delete |
| GDPR Art. 15 | `GET /api/workers/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Practitioner resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Domain-specific compliance:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

