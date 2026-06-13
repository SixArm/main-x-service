## 4. Glossary

| Term | Meaning |
|---|---|
| **Magic link** | A one-time, short-lived URL containing an opaque 32-character token that signs a user in without a password. Expires after 5 minutes (`MAGIC_LINK_EXPIRATION_MIN`); cleared on consumption (single-use). |
| **JWT** | JSON Web Token (RFC 7519) — the signed access token issued on magic-link redemption and sent as `Authorization: Bearer <jwt>` to every service in the federation. |
| **RS256** | RSA + SHA-256 asymmetric signature algorithm (RFC 7518). The private key signs at the auth service; peers verify with the public key only — no shared secret. |
| **JWKS** | JSON Web Key Set (RFC 7517) — the public keys published at `/.well-known/jwks.json`, used by peers to verify token signatures offline. |
| **kid** | Key id, stamped into each token header and each JWK. Derived as the base64url-encoded SHA-256 digest of the RSA public modulus, so it is stable per key and lets verifiers select the right key during rotation. |
| **Claims** | The JWT payload: `sub`, `email`, `name`, `iss`, `aud`, `exp`, `iat`, `jti`. See §5.3. |
| **pid** | A user's public UUID, carried as the token `sub`. Distinct from the internal auto-increment `id`. |
| **jti / jid** | The JWT id (a UUID per token), stored as `sessions.jid` to enable revocation. |
| **Relying party** | A peer service that accepts this entity's tokens — it embeds the verifier crate and trusts the JWKS. |
| **Offline verification** | Verifying a token's signature and claims locally against a cached JWKS, with no call back to the auth service on the request hot path. |
| **Token lifetime** | The access-token TTL: `exp - iat`, default 3600 s (`JWT_EXPIRATION`). Deliberately short — it bounds how long a revoked token stays valid at peers. |
| **Refresh token** | Not implemented — access tokens only (§16 OQ-1). |
| **Session** | (Service) a `sessions` row per issued token — the unit of revocation. (Front-end) the client-side `{token, user}` held in `localStorage`. |
| **Anti-enumeration** | `signup` and `magic-link` always return `200`, whether or not the email exists, so the API never reveals account existence. |
