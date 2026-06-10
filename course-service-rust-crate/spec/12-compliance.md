## 12. Compliance

- **GDPR**: right of access via `GET /courses/{id}/export`; right to
  erasure via soft-delete + masked-view. Audit log records who
  accessed each record.
- **FERPA**: where instances carry instructor or student data, the
  masked view conceals personal identifiers; the audit log preserves
  access trail.
- **WCAG**: out of scope for the service (front-end concern).

