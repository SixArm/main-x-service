## 8. Architecture

The front-end runs its own SvelteKit server as a **Backend-For-Frontend
(BFF)**, per [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
§6: the browser holds only the httpOnly `__Host-mxi_session` cookie and
never sees a token or calls the Thing Service directly.

```
                +----------------------------------------+
                |            Browser (SPA)                |
                |  +-----------------------------------+  |
                |  |  SvelteKit routes                  |  |
                |  |  + Svelte 5 components              |  |
                |  +----------+--------------------------+  |
                |             |                           |
                |             v                           |
                |  +-----------------------+              |
                |  |  ThingRepository       |              |
                |  |  (lib/api/things.ts)   |              |
                |  +----------+-------------+              |
                |             |                             |
                |             v                             |
                |  +-----------------------+                |
                |  |  ApiClient             |                |
                |  |  (lib/api/client.ts)   |                |
                |  +----------+-------------+                |
                +-------------|-----------------------------+
                              | same-origin, __Host-mxi_session cookie
                              v
                +----------------------------------------+
                |     SvelteKit server (this project)      |
                |     hooks.server.ts -> locals.sessionId  |
                |  /api/proxy/[...path]  (reverse proxy):  |
                |    session -> PASETO (lib/server/auth.ts)|
                |    Authorization: Bearer <paseto>         |
                |  /signin, /verify  (magic-link BFF pages) |
                +-------------|-----------------------------+
                              | HTTP JSON, Bearer PASETO
                              v
                +-----------------------------+   +--------------------------+
                |   thing-service-with-loco    |   |  authentication-service   |
                |   Axum + SeaORM + Tantivy     |   |  magic-link, sessions,    |
                |                                |   |  PASETO issuance          |
                +-----------------------------+   +--------------------------+
```

