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
  `AuthMailer` `magic_link` template in production (SMTP from loco
  config). Templates: `magic_link`, `welcome`, `forgot` under
  [`src/mailers/auth/`](../authentication-service-rust-crate/src/mailers/auth/).

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
