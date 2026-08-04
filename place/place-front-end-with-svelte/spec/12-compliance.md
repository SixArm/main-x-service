## 12. Compliance

- **No PII logging**: never `console.log` Place values.
- **Soft-delete confirm**: the UI requires confirm before `DELETE`; the service handles the soft-delete itself.
- **GDPR**: export endpoint exists in the service; UI deferred (roadmap).
- **HIPAA**: audit-log view exposes the trail, including `user_id` attribution where the service supplies it (`/places/[id]/audit`). T-22 (BFF auth) has landed; enforcement still depends on the service's `PLACE_REQUIRE_AUTH` gate (default-off).

