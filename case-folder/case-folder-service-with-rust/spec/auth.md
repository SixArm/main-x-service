# Authentication (Loco edition)

> Part of the [Loco edition specification](index.md). Cross-cutting flow,
> token claims, and config matrix: [root auth](../../spec/auth.md).

The Loco edition is the authority for sessions. It mints + verifies
signed JWTs, sends the magic-link email, and guards the domain API.

## Endpoints

| Method | Route               | Body              | Success                                  | Failure                          |
| ------ | ------------------- | ----------------- | ---------------------------------------- | -------------------------------- |
| POST   | `/api/auth/request` | `{ email }`       | `200 { sent: true, magic_link? }`        | `422` missing email              |
| POST   | `/api/auth/verify`  | `{ token }`       | `200 { user }` + `Set-Cookie cts_session`| `401` invalid/expired token      |
| GET    | `/api/auth/me`      | —                 | `200 { user }`                           | `401` no/invalid session         |
| POST   | `/api/auth/logout`  | —                 | `204` + cookie cleared                   | —                                |

`magic_link` is present in the `request` response only when
`auth.expose_magic_link` is set (dev/test). `/api/auth/*` and `/healthz`
are exempt from the session guard.

## Code layout

```
src/
├── auth/
│   ├── mod.rs        AuthConfig (from config settings), AuthState,
│   │                 Identity, Claims, token encode/decode, cookie +
│   │                 header parsing, identity_from_headers()
│   └── mailer.rs     Mailer trait + LogMailer (writes link to the log)
├── controllers/
│   └── auth.rs       request / verify / me / logout handlers
└── initializers/
    └── auth.rs       builds AuthState from ctx.config.settings,
                      injects Extension<Arc<AuthState>>, and layers the
                      session guard middleware over /api/* (gated by
                      require_session)
```

## Guard

A `from_fn` middleware (added in the auth initializer, capturing the
`Arc<AuthState>`) runs on every request. It passes through when:

- `require_session` is `false` (the `test` env), or
- the path is `/healthz`, doesn't start with `/api/`, or starts with
  `/api/auth/`.

Otherwise it requires a valid `session` JWT from the `cts_session`
cookie or an `Authorization: Bearer` header, returning
`401 { "error": "Authentication required" }` if absent/invalid.

`GET /api/auth/me` and `POST /api/auth/logout` validate the session
themselves regardless of `require_session`.

## Dependencies

- `jsonwebtoken = "9"` (HS256). No SMTP crate yet — `LogMailer` writes
  the link to the tracing log; a real `SmtpMailer` is a production task.

## Config

`settings.auth` in `config/{development,test,production}.yaml`; see the
[root config matrix](../../spec/auth.md#configuration-per-environment).
The `secret` must be overridden in production (env `AUTH_SECRET`).

## Tests

`tests/requests/auth.rs`: `request` returns a dev magic link →
`verify` sets a session → `me` returns the user; bad token → `401`;
`me` with no session → `401`; `logout` → `204` clearing the
`cts_session` cookie (`Max-Age=0`). A shared
`tests/requests.rs::auth_header()`
mints a session token so other suites *could* run guarded, though the
`test` env keeps `require_session: false` so existing domain tests are
unchanged.
