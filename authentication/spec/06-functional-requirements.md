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
  [`src/mailers/auth/`](../authentication-service-rust-crate/src/mailers/auth/);
  the magic-link email is localised (FR-4a).
- **FR-4a — Localised magic-link email (T-7).** The magic-link email
  subject + plain-text + HTML bodies are localised via the
  dependency-light catalog
  [`src/i18n.rs`](../authentication-service-rust-crate/src/i18n.rs)
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

### 6.3 Token issuance (service)

- **FR-5 — Redemption.** `GET /api/auth/magic-link/{token}` validates
  the unexpired token, clears it, marks the email verified (first
  time), signs an RS256 access token with the claims in §5.3, records
  a `sessions` row (`jid` = `jti`), and returns
  `{token, pid, name, email, is_verified}`. Invalid / expired tokens →
  `401`.
- **FR-6 — JWKS publication.** `GET /.well-known/jwks.json` returns
  the RSA public key set (§5.4), pre-rendered at boot from the loaded
  key material.

### 6.4 Session handling (service)

- **FR-7 — Current user.** `GET /api/auth/me` (bearer) verifies the
  token, rejects a locally revoked session (`401 "session signed
  out"`), and returns `{pid, name, email}`.
- **FR-8 — Sign out.** `POST /api/auth/signout` (bearer) sets
  `sessions.revoked_at`. Revocation is **local**: peers honour cached
  tokens until `exp` — the documented tradeoff of offline
  verification, bounded by the short TTL (NFR-4).

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
  (bearer) returns a JSON document of everything the service holds about
  the authenticated subject: their `users` row (`pid`, `email`, `name`,
  `email_verified_at`, timestamps), their `sessions`
  (jid, issuance/expiry/revocation, user_agent), and their `auth_events`
  audit trail (matched by pid *or* email). It excludes the password
  hash, api key, and any token / key material. A GDPR-erased account is
  treated as gone (`401`).
- **FR-8d — Right to erasure (Art. 17).** `DELETE /api/auth/account`
  (bearer) **soft-deletes + anonymises**: stamps `users.deleted_at`,
  replaces `email` with a `pid`-keyed unroutable tombstone
  (`deleted+<pid>@invalid`, keeps `UNIQUE(email)`) and `name` with
  `"deleted user"`, clears magic-link material, revokes all the
  subject's sessions, and records an `account_erased` audit row. The row
  survives so referential history + the audit trail keep integrity.
  Post-erasure the bearer token still verifies cryptographically until
  `exp`, but `/me` and the export return `401`. Idempotent.
- **FR-8e — Per-subject audit.** `GET /api/auth/account/audit` (bearer)
  returns only the authenticated subject's own `auth_events` rows
  (matched by pid or email), newest first — the right-of-access
  counterpart to the open system-wide FR-8b.

### 6.5 Verifier library (verifier)

- **FR-9 — Construction.** `Verifier::from_jwks_value(&jwks, issuer,
  audience)` builds a verifier from an in-memory JWKS, loading RSA
  keys indexed by `kid`, skipping non-RSA entries, and permitting an
  empty key set (boots before the JWKS source is reachable; rejects
  everything until refreshed). With the `fetch` feature,
  `Verifier::from_jwks_url(url, issuer, audience)` fetches over HTTPS.
- **FR-10 — Verification.** `Verifier::verify(token)` selects the key
  by header `kid`, checks the RS256 signature, and enforces `iss`,
  `aud`, and `exp`. Returns the `Claims` (§5.3) or a typed
  `VerifyError` (`Jwks` / `MissingKid` / `UnknownKid` / `Jwt` /
  `Fetch`).

### 6.6 Front-end flows (front-end)

- **FR-11 — Routes.** `/signup` posts FR-1; `/signin` posts FR-2;
  `/verify` consumes `?token=` via FR-5, stores `{token, pid, name,
  email}` in `localStorage` (`mxi.auth.token`, `mxi.auth.user`), and
  redirects to `/`; `/` loads FR-7 and offers sign-out (FR-8 +
  session clear). On `401` the session is cleared.
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
