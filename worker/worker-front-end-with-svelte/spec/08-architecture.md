## 8. Architecture

Since 2026-06-18 (T-22a) this is a **Backend-For-Frontend (BFF)**, per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
§6: the browser holds only the httpOnly `__Host-mxi_session` cookie and
never a bearer token; the SvelteKit **server** holds the session, exchanges
it for a short-lived PASETO, and is the only party that calls the entity
API or the authentication service.

```
+-------------------------------------------------------------+
|                     Browser (CSR/SPA)                       |
|  SvelteKit routes + Svelte 5 components                     |
|  WorkerRepository (lib/api/workers.ts)                      |
|  ApiClient (lib/api/client.ts) — base URL = same-origin proxy|
+---------------------------+-----------------------|---------+
                             | fetch, same-origin    | __Host-mxi_session
                             | (no token in JS)       | httpOnly cookie
                             v                        v
+-------------------------------------------------------------+
|             SvelteKit server (the BFF)                      |
|  hooks.server.ts       — reads the session cookie            |
|  routes/signin, /verify — per-app magic-link login pages     |
|  lib/server/session.ts — cookie name + attributes            |
|  lib/server/auth.ts    — session -> PASETO exchange          |
|  routes/api/proxy/[...path] — reverse proxy, injects PASETO  |
+---------------+---------------------------+-----------------+
                 | Authorization: Bearer     | magic-link + token
                 | <PASETO> (server-to-server)| exchange requests
                 v                           v
   +-----------------------------+  +-----------------------------+
   |   worker-service-with-loco  |  |   authentication-service     |
   |   Axum + SeaORM + Tantivy   |  |   (central SSO, PASETO mint) |
   +-----------------------------+  +-----------------------------+
```

Page code is unaware of the indirection: `ApiClient`'s base URL is the
same-origin `/api/proxy` (`lib/config.ts`), so `WorkerRepository` calls
look identical to the pre-BFF direct-to-service shape.

