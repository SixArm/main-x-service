## 6. Functional Requirements

Each requirement names its owning subproject. Endpoint detail:
[`AGENTS/restful.md`](../AGENTS/restful.md); verification detail:
[`AGENTS/verification.md`](../AGENTS/verification.md).

### 6.1 Magic-link issuance (service)

- **FR-1 — Sign up.** `POST /api/auth/signup {email, name?}` creates a
  passwordless account and issues a magic link. `name` defaults from
  the email local part when omitted or shorter than 2 characters. An
  already-registered email receives a fresh link. Always `200`.
- **FR-2 — Sign in.** `POST /api/auth/magic-link {email}` issues a
  magic link for an existing account. Unknown emails still get `200`
  (anti-enumeration), with nothing sent.
- **FR-3 — Link properties.** The link token is a random 32-character
  string, valid 5 minutes, single-use (cleared on redemption). The
  link targets `{FRONTEND_URL}/verify?token={token}`.

### 6.2 Email delivery (service)

- **FR-4 — Delivery.** The magic link is logged to the tracing console
  (authoritative in development; the mailer is disabled in
  `config/development.yaml`) and best-effort emailed via the
  `AuthMailer` in production (SMTP from loco config). The `welcome` /
  `forgot` emails render from the on-disk templates under
  [`src/mailers/auth/`](../authentication-service-with-loco/src/mailers/auth/);
  the magic-link email is localised (FR-4a).
- **FR-4a — Localised magic-link email (T-7).** The magic-link email
  subject + plain-text + HTML bodies are localised via the
  dependency-light catalog
  [`src/i18n.rs`](../authentication-service-with-loco/src/i18n.rs)
  (`magic_link_email(locale) -> EmailStrings`), not the on-disk
  template directory. Supported locales: **English (`en`)** and
  **Welsh (`cy`)** — Welsh chosen for the public-sector Welsh-language
  duty (§7, §12); more locales are added by extending the catalog.
  The locale is selected per request from the optional `locale` field
  on the signup / magic-link request body (`select_locale`); unknown,
  malformed, or absent input falls back to `en`, and a region subtag
  (`cy-GB`) is reduced to its primary language (`cy`). Locale affects
  **only** the rendered email language — the always-`200` response
  shape (FR-1/FR-2) is unchanged regardless of locale.

### 6.3 Session establishment + cross-service token (service)

- **FR-5 — Redemption establishes a session.**
  `GET /api/auth/magic-link/{token}` validates the unexpired token,
  clears it, marks the email verified (first time), **creates a
  `sessions` row** (§5.2), and sets the `__Host-mxi_session` cookie
  (HttpOnly, Secure, SameSite=Lax, `Path=/`) carrying the opaque `sid`.
  It **no longer returns a token** — the response is the current-user
  shape `{pid, name, email, is_verified}` (the credential is the
  `Set-Cookie`). It also issues a CSRF token for the session (§6.4c).
  Invalid / expired tokens → `401`. *(Mechanism unchanged per shared §7;
  only the outcome — session+cookie, not JWT — changes.)*
- **FR-6 — Cross-service token exchange.** `POST /token` (session
  cookie, CSRF-protected) exchanges the caller's valid session for a
  short-lived **PASETO v4.public** token (claims §5.3, `exp` ~5 min,
  footer `kid`) and returns it for use as `Authorization: Bearer
  v4.public.…` against entity services. No long-lived token exists; a
  revoked / expired session yields `401`.
- **FR-6a — Public-key publication.** `GET /.well-known/paseto-keys`
  returns the Ed25519 public key set (§5.4), pre-rendered at boot from
  the loaded key material — the JWKS analog peers fetch once and cache.

### 6.4 Session handling (service)

- **FR-7 — Current user.** `GET /api/auth/me` (session cookie) resolves
  the session, rejects an expired / revoked session (`401`), bumps
  `last_seen_at` / `idle_expires_at`, and returns `{pid, name, email}`.
- **FR-8 — Sign out.** `POST /api/auth/signout` (session cookie,
  CSRF-protected) sets `sessions.revoked_at` and clears the cookie.
  Cross-service tokens already minted (§6.3 FR-6) remain valid until
  their ~5-min `exp` — the documented tradeoff of offline verification,
  bounded by the short PASETO TTL.

### 6.4c CSRF (service)

- **FR-8f — CSRF protection.** Cookie-authenticated **state-changing**
  requests (`POST`/`PUT`/`PATCH`/`DELETE`, incl. `POST /token`, signout,
  and `DELETE /api/auth/account`) require a valid per-session CSRF token
  (synchroniser / double-submit, echoed in `X-CSRF-Token`), backstopped
  by an `Origin`/`Referer` allow-list. Safe methods (`GET`/`HEAD`) are
  exempt. `SameSite` on the session cookie is the first line. See
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  §4.

### 6.4a Audit trail (service)

- **FR-8a — Auth-event audit (T-10).** Every authentication endpoint
  writes a best-effort `auth_events` row — `signup`,
  `magic_link_requested`, `magic_link_redeemed`, `signout` — capturing
  the event, the normalised email and/or subject pid when known, and an
  outcome `detail` (`rate_limited` / `unknown_email` /
  `invalid_or_expired` / `issued` / `created` / `existing` / `ok` /
  `rejected`). Writes never fail the request and never store a token or
  secret. The row may distinguish outcomes the response deliberately
  hides — the anti-enumeration shape (FR-1/FR-2) holds at the wire.
- **FR-8b — Audit query.** `GET /api/auth/audit/recent` returns the
  newest 100 `auth_events` (`AuthEvent[]`). Deliberately unauthenticated
  (operator-facing system feed), mirroring the family `/audit/recent`
  pattern (§9, §12); the per-subject right-of-access view is FR-8e.

### 6.4b GDPR subject rights (service)

- **FR-8c — Right of access (Art. 15).** `GET /api/auth/account/export`
  (session cookie) returns a JSON document of everything the service holds about
  the authenticated subject: their `users` row (`pid`, `email`, `name`,
  `email_verified_at`, timestamps), their `sessions`
  (jid, issuance/expiry/revocation, user_agent), and their `auth_events`
  audit trail (matched by pid *or* email). It excludes the password
  hash, api key, and any token / key material. A GDPR-erased account is
  treated as gone (`401`).
- **FR-8d — Right to erasure (Art. 17).** `DELETE /api/auth/account`
  (session cookie, CSRF-protected) **soft-deletes + anonymises**: stamps `users.deleted_at`,
  replaces `email` with a `pid`-keyed unroutable tombstone
  (`deleted+<pid>@invalid`, keeps `UNIQUE(email)`) and `name` with
  `"deleted user"`, clears magic-link material, revokes all the
  subject's sessions, and records an `account_erased` audit row. The row
  survives so referential history + the audit trail keep integrity.
  Post-erasure `/me` and the export return `401`; any already-minted
  cross-service PASETO expires within its ~5-min TTL. Idempotent.
- **FR-8e — Per-subject audit.** `GET /api/auth/account/audit` (session cookie)
  returns only the authenticated subject's own `auth_events` rows
  (matched by pid or email), newest first — the right-of-access
  counterpart to the open system-wide FR-8b.

### 6.5 Verifier library (verifier)

- **FR-9 — Construction.** `Verifier::from_paseto_keys_value(&keys,
  issuer, audience)` builds a verifier from an in-memory Ed25519
  key-set document (§5.4), loading keys indexed by `kid`, skipping
  non-Ed25519 entries, and permitting an empty key set (boots before
  the key source is reachable; rejects everything until refreshed).
  With the `fetch` feature, `Verifier::from_paseto_keys_url(url, issuer,
  audience)` fetches over HTTPS. *(These replace the RS256
  `from_jwks_value` / `from_jwks_url`; see shared §5.)*
- **FR-10 — Verification.** `Verifier::verify(token)` selects the key
  by footer `kid`, checks the PASETO v4.public (Ed25519) signature, and
  enforces `iss`, `aud`, and `exp`. Returns the `Claims` (§5.3) or a
  typed `VerifyError` (`Keys` / `MissingKid` / `UnknownKid` / `Paseto` /
  `Fetch`).

### 6.6 Front-end flows (front-end)

- **FR-11 — Routes (BFF).** `/signup` posts FR-1; `/signin` posts
  FR-2; `/verify` consumes `?token=` via FR-5 through the SvelteKit
  server, which receives the `Set-Cookie` and forwards the
  `__Host-mxi_session` cookie to the browser (no token stored in JS),
  then redirects to `/`; `/` loads FR-7 and offers sign-out (FR-8). The
  browser never holds a token and never calls an entity service
  directly: mutating calls go through the front-end's own server (BFF),
  which exchanges the session for a PASETO (FR-6) server-side (shared
  §6). On `401` the session cookie is cleared. `localStorage` is no
  longer used for credentials.
- **FR-12 — Localised UI (T-7).** All user-facing UI strings (nav,
  sign-up / sign-in / link-sent / verify / account / sign-out / error
  messages) come from a dependency-light strings catalog
  (`src/lib/i18n.svelte.ts`: a per-locale map + reactive `t(key)`
  accessor over a `$state` current-locale). Supported locales match
  FR-4a: **English (`en`)** + **Welsh (`cy`)**; an unknown key or
  locale falls back to `en`. A locale switcher in the layout persists
  the choice to `localStorage` (`mxi.auth.locale`), and the chosen
  locale is sent as the `locale` field on the signup / magic-link
  request (FR-4a) so the email matches the UI language.
