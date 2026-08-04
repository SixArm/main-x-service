## 8. Architecture

Since T-22 (BFF auth), the browser no longer talks straight to the Place
Service — it calls the SvelteKit server's same-origin proxy, which holds
the session and injects a short-lived PASETO (see `AGENTS.md`
"Authentication — the BFF pattern"):

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
                |  |  PlaceRepository     |  |
                |  |  (lib/api/places.ts) |  |
                |  +----------+------------+  |
                |             |               |
                |             v               |
                |  +-----------------------+  |
                |  |  ApiClient            |  |
                |  |  (lib/api/client.ts)  |  |
                |  |  base = /api/proxy    |  |
                |  +----------+------------+  |
                +-------------|---------------+
                   __Host-mxi_session cookie
                   (httpOnly; no token in JS)
                              v
                +-----------------------------+
                |  SvelteKit server (BFF)      |
                |  /api/proxy/[...path]        |
                |  - exchanges session for a   |
                |    short-lived PASETO        |
                |    (lib/server/auth.ts)      |
                |  - forwards with             |
                |    Authorization: Bearer     |
                +-------------|---------------+
                              | HTTP JSON
                              v
                +-----------------------------+
                |   place-service-with-loco |
                |   Axum + SeaORM + Tantivy   |
                +-----------------------------+
```

