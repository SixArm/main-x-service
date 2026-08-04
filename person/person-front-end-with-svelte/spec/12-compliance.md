## 12. Compliance

- **No PII logging**: never `console.log` Person values.
- **Soft-delete confirm**: the UI requires confirm before `DELETE`; the service handles the soft-delete itself.
- **GDPR**: export endpoint exists in the service; UI deferred (roadmap).
- **HIPAA**: audit-log view exposes the trail. Sign-in (BFF magic-link, §13 T-22) is implemented, but the front-end does not yet attribute an audit entry to the signed-in operator in the UI — the audit payload comes from the service as-is.

