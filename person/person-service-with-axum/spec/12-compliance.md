## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest (DB), soft delete |
| GDPR Art. 15 | `GET /api/persons/{id}/export` |
| GDPR Art. 17 | `POST /api/persons/{id}/erase`: erasure by redaction, the audit chain still verifying |
| GDPR Art. 6 / Art. 9 / Ch. V | `PERSON_LAWFUL_BASIS`, `PERSON_ART9_CONDITION`, `PERSON_DATA_RESIDENCY`, `PERSON_TRANSFER_SAFEGUARD`, each `undeclared` until set; reported at `GET /api/compliance` and stamped on every audited access, which also records `cross_border` when `X-Destination-Region` lies outside the declared residency. Declares and records; does not block. |
| HL7 FHIR R5 | Person resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |
| ISO/IEC 42001:2023 | AIMS controls (where matcher tuning is ML-driven) |

Domain-specific compliance:
[`agents/share/compliance-for-healthcare.md`](../../../agents/share/compliance-for-healthcare.md).
Technology compliance:
[`agents/share/compliance-for-technology.md`](../../../agents/share/compliance-for-technology.md).

