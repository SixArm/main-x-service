## 12. Compliance

- **No PII logging**: never `console.log` Event values.
- **Soft-delete confirm**: the UI requires confirm before `DELETE`; the service handles the soft-delete itself.
- **GDPR**: export endpoint exists in the service; UI deferred (roadmap).
- **HIPAA**: audit-log view exposes the trail, including `entry.user_id` where the service populates it. BFF sign-in is landed (§13 T-23a); CSRF (§13 T-23b) and any front-end-side ABAC/masking are still deferred.

