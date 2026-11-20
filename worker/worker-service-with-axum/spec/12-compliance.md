## 12. Compliance

| Standard | Mechanism |
|---|---|
| HIPAA | Audit log, access tracking, encryption-at-rest, soft delete |
| GDPR Art. 15 | `GET /api/workers/{id}/export` |
| GDPR Art. 17 | `POST /api/workers/{id}/erase`: erasure by redaction, the audit chain still verifying |
| GDPR Art. 6 / Art. 9 / Ch. V | `WORKER_LAWFUL_BASIS`, `WORKER_ART9_CONDITION`, `WORKER_DATA_RESIDENCY`, `WORKER_TRANSFER_SAFEGUARD`, each `undeclared` until set; reported at `GET /api/compliance` and stamped on every audited access, which also records `cross_border` when `X-Destination-Region` lies outside the declared residency. Declares and records; does not block. |
| HL7 FHIR R5 | `Practitioner` resource bidirectional conversion |
| ISO/IEC 27001 | Operational controls (deployment-side) |

Domain-specific compliance:
[`agents/share/compliance-for-healthcare.md`](../../../agents/share/compliance-for-healthcare.md).

