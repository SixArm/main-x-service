## 12. Compliance

- **No PII logging**: never `console.log` Place values.
- **Soft-delete confirm**: the UI requires confirm before `DELETE`; the service handles the soft-delete itself.
- **GDPR**: export endpoint exists in the service; UI deferred (roadmap).
- **HIPAA**: audit-log view exposes the trail; user-attribution requires auth (deferred).

