## 8. Architecture

The browser is a BFF client, not a direct API caller: it holds only the
opaque `__Host-mxi_session` httpOnly cookie (never a bearer token) and
calls exclusively same-origin routes. The SvelteKit **server** — never
bundled into the browser — reads that cookie, exchanges it for a
short-lived PASETO server-side, and forwards to the person service with
`Authorization: Bearer …`. See
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
and `AGENTS.md` for the pattern; `src/lib/server/{config,session,auth}.ts`
+ `src/hooks.server.ts` + `src/routes/api/proxy/[...path]/+server.ts` for
the implementation.

```
                +---------------------------------------------------+
                |               Browser (SPA)                       |
                |  SvelteKit routes + Svelte 5 components            |
                |  PersonRepository (lib/api/persons.ts)             |
                |  ApiClient (lib/api/client.ts)                     |
                |  -> fetches ONLY same-origin /api/proxy/*          |
                |  -> holds __Host-mxi_session (httpOnly; no JS read)|
                +--------------------------|--------------------------+
                                           | HTTP + session cookie
                                           v
                +---------------------------------------------------+
                |         SvelteKit server (the BFF)                 |
                |  hooks.server.ts     - reads the session cookie    |
                |  routes/api/proxy/*  - reverse proxy; drops the    |
                |                        browser's cookie, injects   |
                |                        Authorization: Bearer <PASETO>
                |  routes/signin, /verify - magic-link login/verify  |
                +----------|-------------------------|----------------+
                           | session -> PASETO        | Bearer PASETO
                           v                           v
                +-----------------------+   +-----------------------+
                | authentication-service|   | person-service-with-loco|
                | magic-link + sessions |   | Axum/loco + SeaORM +   |
                | + POST /api/auth/token|   | Tantivy                |
                +-----------------------+   +-----------------------+
```

CSRF protection for mutating browser→BFF calls (the synchroniser token
in `authentication-sessions.md` §4) is not yet implemented — see §13
T-22 and §16 OQ-3.

