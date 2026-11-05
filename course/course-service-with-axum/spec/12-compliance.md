## 12. Compliance

- **GDPR**: right of access via `GET /courses/{id}/export`; right to
  erasure via soft-delete + masked-view. Audit log records who
  accessed each record.
- **FERPA**: where instances carry instructor or student data, the
  masked view conceals personal identifiers; the audit log preserves
  access trail.
- **WCAG**: out of scope for the service (front-end concern).
- **Row-level integrity (T-24, default off)**: each `Course` record
  and each `audit_log` row carries three stored values over the same
  pre-image — SHA-256, SHA3-256, and a keyed HMAC-SHA256 MAC (via the
  shared `integrity-mac` crate) — verified on demand via
  `GET /api/records/verify` and `GET /api/audit/verify`
  (`src/compliance/`). The two digests are unkeyed (their pre-image
  format is published here and in code, so anyone with SQL access can
  recompute them); the **MAC is the one an adversary holding only the
  database cannot forge**. There is **no hash chain** here (unlike
  person / worker / care-pathway / case), so deletion of a whole row
  is not detected — only content tampering on a surviving row. With no
  `COURSE_INTEGRITY_MAC_KEY[_FILE]` configured, no MAC is written and
  rows report `mac_absent` rather than as mismatches; a soft-deleted
  course clears its digests and reports `unhashed`, not tampered.

