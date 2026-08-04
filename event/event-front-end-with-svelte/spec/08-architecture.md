## 8. Architecture

The browser never holds a token — it carries only the httpOnly
`__Host-mxi_session` cookie. The SvelteKit server is a BFF: it holds the
session, exchanges it server-side for a short-lived PASETO, and proxies
every entity-API call. See
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
and §13 T-23a/T-23b (CSRF not yet done).

```
                +-----------------------------+
                |        Browser (SPA)        |
                |  +-----------------------+  |
                |  |  SvelteKit routes     |  |
                |  |  + Svelte 5 components|  |
                |  +----------+------------+  |
                |             |               |
                |             v               |
                |  +-----------------------+  |
                |  |  EventRepository       |  |
                |  |  (lib/api/events.ts)   |  |
                |  +----------+------------+  |
                |             |               |
                |             v               |
                |  +-----------------------+  |
                |  |  ApiClient             |  |
                |  |  (lib/api/client.ts)   |  |
                |  +----------+------------+  |
                +-------------|---------------+
                              | same-origin fetch,
                              | __Host-mxi_session cookie only
                              v
                +-----------------------------+
                |   SvelteKit server (BFF)     |
                |   /api/proxy/[...path]       |
                |   src/lib/server/{session,   |
                |     auth,config}.ts          |
                +---------+---------+----------+
                          |         |
        Authorization:    |         | session -> PASETO
        Bearer <paseto>   |         | (POST /api/auth/token)
                          v         v
        +-----------------------------+   +-----------------------------+
        |   event-service-with-loco   |   |   authentication-service    |
        |   Axum + SeaORM + Tantivy   |   |   (SSO, magic-link, PASETO) |
        +-----------------------------+   +-----------------------------+
```

