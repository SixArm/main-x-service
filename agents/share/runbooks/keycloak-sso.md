# Runbook: Keycloak as the enterprise sign-in (OIDC)

Keycloak (or any standards-compliant OpenID Connect provider) can be the
**front door** to the family without adding a Keycloak-specific crate to
any service. This is the operational side of
[`authentication-sessions.md`](../authentication-sessions.md) §7a (EV-2):
`authentication-service` is the one OIDC relying party; a successful
Keycloak sign-in ends in the same Postgres session, cookie and PASETO
issuance a magic-link sign-in does, so **no entity service changes** for this path and
every peer keeps verifying PASETO offline.

> **Why the crate is `axum-keycloak-auth`, not `loco-keycloak-auth`.** The
> wrapper is 160 lines of glue written against `loco-rs ^0.15`
> (`Keycloak::from_context(&AppContext)`): unusable from a loco 1.x service,
> and depending on it would put a second loco (0.15, default features,
> ~1,600 packages) in every service's tree. What it wraps is
> `axum-keycloak-auth` (no loco dependency, `axum 0.8`, `jsonwebtoken` 9 on
> `ring`), which passes `cargo deny` cleanly. The verifier crate uses that
> directly. (An earlier version of this note said the wrapper pulled in the
> `rsa` crate; that did not reproduce: there is no `rsa` in either tree.)

There are **two independent ways** to use Keycloak, and a deployment may use
either or both:

1. **Keycloak as the sign-in front door** (steps 1 to 5 below): people click
   "Sign in with SSO", authentication-service federates with Keycloak, and
   everything downstream stays PASETO. No entity-service change.
2. **Services accept Keycloak access tokens directly** (the section
   "Direct bearer tokens" at the end): an API client or gateway that already
   holds a Keycloak access token calls a service with it. Off by default,
   behind each service's `keycloak` cargo feature.

## What you are wiring

```
browser ─▶ front-end /signin/sso ─▶ auth-service /api/auth/oidc/login
        ◀────────── redirect ───────────────┘
browser ─▶ Keycloak (authorization-code + PKCE) ─▶ auth-service /api/auth/oidc/callback
        verifies the ID token (signature, issuer, audience, nonce), maps claims
        to ABAC attributes, mints a single-use bridge token
browser ─▶ front-end /verify?token=… ─▶ session cookie on the front-end's origin
```

The front-end half is a plain browser navigation (not a `fetch`), because
the browser itself has to visit Keycloak and come back.

## Steps, in order

### 1. Build the auth service with the `oidc` feature

The feature is **off by default** and the shipped `Containerfile` runs
`cargo build --release --bin authentication-service-cli` with no features,
so the stock image has no OIDC routes at all (`/api/auth/oidc/login` is
404). Build with `--features oidc`.

**Know what that build carries.** `--features oidc` brings in
`openidconnect`, which depends on the `rsa` crate: `cargo deny --features
oidc check` fails on **RUSTSEC-2023-0071** (the Marvin timing attack, which
`security.md` §7 records as removed family-wide on 2026-08-21). This is not
new (it fails at the commit that introduced `oidc`), and CI's `deny` stage
runs only default features, so it does not show up there. The advisory
concerns RSA private-key operations, and this service only *verifies* the
provider's RS256 signatures, so exposure is probably small, but that is a
judgement, not a verified result: a deployment that enables `oidc` should
make that call deliberately (or accept it in its own `deny.toml`) rather than
inherit it silently.

### 2. Create the Keycloak client

In the realm (e.g. `mxi`):

| Setting | Value |
|---|---|
| Client type | OpenID Connect |
| Client authentication | **On** (a confidential client: the service holds a secret) |
| Standard flow | On. Direct access grants, implicit: **off** |
| Valid redirect URIs | exactly `https://<auth-host>/api/auth/oidc/callback` |
| Web origins | leave empty (no browser CORS call is made) |
| PKCE method | `S256` (the service always sends a challenge) |
| Client scopes | default `openid`, `email`, `profile` (the service requests all three) |

Copy the client secret from the **Credentials** tab.

**Verified email.** The service takes the ID token's `email` as the
sign-in identity and does **not** read `email_verified`. Make Keycloak the
gatekeeper: enable *Verify email* on the realm and do not let users edit
their email address without re-verification, or a user could assert
someone else's address.

### 3. Point the auth service at it

| Variable | Value |
|---|---|
| `AUTH_OIDC_ISSUER_URL` | `https://<keycloak-host>/realms/<realm>` (discovery is `{issuer}/.well-known/openid-configuration`) |
| `AUTH_OIDC_CLIENT_ID` | the client id |
| `AUTH_OIDC_CLIENT_SECRET_FILE` | path to a file holding the secret (preferred; a file outranks the environment) or `AUTH_OIDC_CLIENT_SECRET` |
| `AUTH_OIDC_REDIRECT_URL` | `https://<auth-host>/api/auth/oidc/callback` (must equal the Keycloak redirect URI exactly) |
| `AUTH_OIDC_JIT_PROVISIONING` | unset (default **off**): an email with no account is a named, audited `403`. Set `true` only if Keycloak should control account creation |
| `AUTH_ALLOWED_FRONTENDS` | comma-separated exact origins of every front-end that offers SSO (`https://person.example.org,https://worker.example.org,…`). An origin not listed falls back to `FRONTEND_URL` |
| `FRONTEND_URL` | the default landing front-end |

Unset `AUTH_OIDC_ISSUER_URL` keeps the whole feature dormant.

### 4. Map Keycloak claims to ABAC attributes

`AUTH_OIDC_CLAIM_MAP` (or `AUTH_OIDC_CLAIM_MAP_FILE`, which wins) is JSON
`{"<claim>": "<attribute key>"}`, e.g. `{"roles":"access","dept":"dept"}`.
The mapper only reads **top-level claims that are a string or an array of
strings**, and every mapped key and value passes the same validation and
`AUTH_ATTRIBUTE_VOCABULARY` gate as an operator assignment; anything else
is skipped and named in the audit trail, never fatal.

Keycloak does not emit those by default, so add **protocol mappers** on
the client (Client scopes → `<client>-dedicated` → Add mapper):

| Want | Mapper type | Token claim name |
|---|---|---|
| Realm roles as `access` | *User Realm Role* | `roles` (multivalued, add to ID token) |
| Groups as `dept` | *Group Membership* (**Full group path: off**) | `dept` (add to ID token) |
| A user attribute | *User Attribute* | any name (add to ID token) |

Realm roles otherwise sit in the nested `realm_access.roles` object, which
the mapper cannot read: that is the usual reason "roles never arrive". The
values must be short lowercase tokens (`write`, `admin`, …) to pass
validation, so name Keycloak roles accordingly.

### 5. Turn the link on in each front-end

Every front-end carries a "Sign in with SSO" link on `/signin`, hidden
unless `PUBLIC_OIDC_SIGNIN_ENABLED=true` in **that front-end's**
environment (read at runtime via `$env/dynamic/public`; restart the
server after changing it). Magic link stays the default.

## Verify each step took effect

```sh
# 1. feature compiled in and configured: not 404
curl -si "https://<auth-host>/api/auth/oidc/login?return_url=https://person.example.org" | head -3
#    expect 303/302 to <keycloak>/protocol/openid-connect/auth?...code_challenge=...

# 2. discovery reachable from the auth service's network
curl -s "$AUTH_OIDC_ISSUER_URL/.well-known/openid-configuration" | jq .issuer

# 3. end to end: click "Sign in with SSO", authenticate, land signed in.
#    Then read the attributes that arrived:
#    the `user_attributes` task: op:show email:<you>
```

## Symptom → check → action

| Symptom | Check | Action |
|---|---|---|
| `/api/auth/oidc/login` is 404 | build features; `AUTH_OIDC_ISSUER_URL` set? | build with `--features oidc`; set the variables |
| No "Sign in with SSO" link | front-end `PUBLIC_OIDC_SIGNIN_ENABLED` | set to `true`, restart |
| Keycloak: "Invalid parameter: redirect_uri" | client Valid redirect URIs vs `AUTH_OIDC_REDIRECT_URL` | make them identical, scheme and path included |
| `403` after a good Keycloak login | unknown email and JIT off | create the account, or enable `AUTH_OIDC_JIT_PROVISIONING` deliberately |
| Signed in but read-only | claim map / mappers; audit trail lists skipped claims | add the mapper (step 4); check the vocabulary |
| Lands on the wrong app | `AUTH_ALLOWED_FRONTENDS` | list the exact origin |
| ID token rejected | issuer URL has a trailing slash or wrong realm; clock skew | match `iss` in the token exactly |

## Not covered

SAML 2.0 (evaluated and deferred, `authentication-sessions.md` §7a), single
logout (signing out of the app does not end the Keycloak session), and
several simultaneous identity providers (one per deployment in v1).

## Direct bearer tokens: the `keycloak` feature on each service

Independent of the SSO front door above. Build a service with
`--features keycloak` and set `<ENTITY>_KEYCLOAK_*`; its blanket guard then
accepts `Authorization: Bearer <Keycloak access token>` in addition to a
PASETO. A token beginning `v4.` is a PASETO; anything else is offered to
Keycloak. Both yield the same `Claims`, so the ABAC policy, masking and
audit are identical.

| Variable | Meaning |
|---|---|
| `<ENTITY>_KEYCLOAK_URL` | server base URL, `https` (loopback excepted). **Unset ⇒ off** |
| `<ENTITY>_KEYCLOAK_REALM` | realm name (required once the URL is set) |
| `<ENTITY>_KEYCLOAK_AUDIENCES` | comma-separated accepted `aud` (required, at least one) |
| `<ENTITY>_KEYCLOAK_ROLE_MAP` / `_ROLE_MAP_FILE` | JSON `{"<role>": {"<attr>": ["<value>"]}}`; a client role is `"<client>:<role>"`. The file wins |
| `<ENTITY>_KEYCLOAK_REQUIRE_VERIFIED_EMAIL` | default on; `false`/`off` disables |

What the verifier enforces, fail-closed: HTTPS-only URL; the token's `iss`
must equal `{url}/realms/{realm}` (the engine checks signature and audience
but not issuer); at least one audience; `email_verified` when an `email` is
present; and **an unmapped Keycloak role grants nothing**, so an
unconfigured realm yields read-only callers under the default policy. A
half-configured Keycloak logs an error and leaves Keycloak off (its tokens
are refused); the service still boots. Add a client audience mapper in
Keycloak so access tokens carry your `aud` (Client scopes → Add mapper →
*Audience*), and mapper(s) for roles as in step 4, since the same
`realm_access` / `resource_access` claims are read here.

**PASETO off.** `--no-default-features --features keycloak` builds a service
with no PASETO code: only Keycloak tokens are accepted. The front-ends'
BFF exchanges a session for a PASETO (`authentication-sessions.md` §6), so a
front-end pointed at a Keycloak-only service will get 401 for every call;
turn PASETO off only where every caller holds a Keycloak token.

Not covered: the front-ends do not hold a Keycloak access token (the OIDC
flow ends in the auth service's session), so browser sessions keep using the
PASETO path. Direct bearer acceptance is for API clients and gateways.
