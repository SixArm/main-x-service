## 16. Open Questions

- **OQ-1**: SVAR DataGrid free tier is GPL-3.0. If this front-end ships in a commercial / proprietary deployment, what license tier do we need? Pro? Enterprise? Decision required before any production deploy.
- **OQ-2**: Should the create route call `check-duplicates` for an inline preview before the actual `create` POST, or rely solely on the 409 round-trip? Round-trip is simpler; preview is friendlier. Operator feedback needed.
- **OQ-3 (partially resolved)**: The BFF + httpOnly-cookie model itself is implemented (§8, §13 T-22): the browser holds only the `__Host-mxi_session` cookie, the SvelteKit server holds the session and attaches a short-lived PASETO server-side; no token in JS, no `localStorage`. Still open: (1) the UI has no explicit redirect-to-`/signin` on a `403`/`401` from `/api/proxy` — a page just shows its normal error state; (2) CSRF protection on mutating browser→BFF calls is not implemented at all (no synchroniser token, no `X-CSRF-Token` check).
- **OQ-4**: Drift policy: per project decision 2026-06-02 we keep API client + types in each front-end project. Revisit when the third sibling front-end ships if drift becomes painful.

