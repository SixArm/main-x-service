## 16. Open Questions

- **OQ-1**: SVAR DataGrid free tier is GPL-3.0. If this front-end ships in a commercial / proprietary deployment, what license tier do we need? Pro? Enterprise? Decision required before any production deploy.
- **OQ-2**: Should the create route call `check-duplicates` for an inline preview before the actual `create` POST, or rely solely on the 409 round-trip? Round-trip is simpler; preview is friendlier. Operator feedback needed.
- **OQ-3**: When the service returns `403`/`401` (post-auth), how should the UI redirect? Auth follows the BFF + httpOnly-cookie model in [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md): the browser holds only the `__Host-mxi_session` cookie, the SvelteKit server holds the session and attaches a short-lived PASETO server-side; no token in JS, no `localStorage`, CSRF on mutating browser→BFF calls.
- **OQ-4**: Drift policy: per project decision 2026-06-02 we keep API client + types in each front-end project. Revisit when the third sibling front-end ships if drift becomes painful.

