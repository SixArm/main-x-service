## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest (DB), soft delete |
| GDPR Art. 15 | `GET /api/persons/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Person resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |
| ISO/IEC 42001:2023 | AIMS controls (where matcher tuning is ML-driven) |

Domain-specific compliance:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).
Technology compliance:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).

