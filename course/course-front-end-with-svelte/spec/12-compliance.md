## 12. Compliance

- **No PII logging**: never `console.log` Course values.
- **Soft-delete confirm**: the UI requires confirm before `DELETE`; the service handles the soft-delete itself.
- **GDPR**: export endpoint exists in the service; UI deferred (roadmap).
- **HIPAA**: audit-log view exposes the trail; BFF sign-in exists (2026-06-18) but the UI does not yet gate routes or attribute actions to the signed-in user (§13 T-24 follow-up).

