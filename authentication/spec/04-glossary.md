## 4. Glossary

| Term | Meaning |
|---|---|
| **Magic link** | A one-time, short-lived URL containing an opaque 32-character token that signs a user in without a password. Expires after 5 minutes (`MAGIC_LINK_EXPIRATION_MIN`); cleared on consumption (single-use). The **outcome** of redemption is now a server-side session + cookie, not a token. |
| **Session** | The **human login**: a server-side `sessions` row (§5.2) keyed by an opaque `sid`, carried to the browser only as the `__Host-mxi_session` httpOnly cookie. The unit of revocation. Has idle + absolute TTLs. **Not** a JWT. |
| **`__Host-mxi_session`** | The session cookie: `HttpOnly; Secure; SameSite=Lax; Path=/`, `__Host-` prefix (host-locked, HTTPS-only). Carries only the opaque `sid`; browser JS can never read it. |
| **sid** | The opaque, high-entropy session id (primary key of `sessions`). Rotated on privilege change to prevent fixation. |
| **PASETO** | Platform-Agnostic Security Token. The cross-service credential is **v4.public** (Ed25519-signed, asymmetric) — the short-lived (~5 min) signed assertion that replaces the JWT for offline peer verification. The private key signs at the auth service; peers verify with the public key only. |
| **CSRF token** | A per-session synchroniser / double-submit token required on cookie-authenticated `POST`/`PUT`/`PATCH`/`DELETE` (shared §4); echoed in `X-CSRF-Token` and compared server-side. |
| **kid** | Key id, carried in the PASETO **footer** and in each published key entry. Selects the verifying Ed25519 key during rotation. |
| **Claims** | The PASETO payload: `sub`, `iss`, `aud`, `iat`, `nbf`, `exp` (~5 min), `sid`, `scope`/`roles`. See §5.3. |
| **pid** | A user's public UUID, carried as the token `sub` and as `sessions.user_pid`. Distinct from the internal auto-increment `id`. |
| **Relying party** | A peer service that accepts this entity's PASETO tokens — it embeds the verifier crate and trusts the published Ed25519 keys. |
| **Offline verification** | Verifying a PASETO token's signature and claims locally against the cached Ed25519 key set, with no call back to the auth service on the request hot path. |
| **Token lifetime** | The PASETO TTL: `exp - iat`, default ~5 min. Deliberately short — it bounds how long a token outlives a revoked session at peers. |
| **BFF** | Backend-For-Frontend: each SvelteKit front-end's own server holds the session cookie and exchanges it (`POST /token`) for a PASETO it sends to entity services server-side, so no token ever reaches browser JS (shared §6). |
| **Anti-enumeration** | `signup` and `magic-link` always return `200`, whether or not the email exists, so the API never reveals account existence. |
| **JWT / RS256 / JWKS (decommissioned)** | The previous model: an RS256 JSON Web Token sent as `Authorization: Bearer`, verified against `/.well-known/jwks.json`. **Superseded** by sessions + PASETO (this pivot); retained here only to read prior history. |
