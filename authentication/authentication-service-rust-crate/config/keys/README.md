# JWT signing keys

This directory holds the **development** RSA keypair the authentication
service uses to sign and verify RS256 access tokens:

- `jwt_private_dev.pem` — dev signing key (PKCS#8 private key).
- `jwt_public_dev.pem` — matching public key (published in the JWKS).

> **Dev only.** These are committed for a stable, restart-stable dev
> JWKS. In production, supply key material from the edges via env (see
> below); never ship these dev keys.

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `JWT_PRIVATE_KEY_FILE` | `config/keys/jwt_private_dev.pem` | Primary RSA private signing key (PEM). |
| `JWT_PUBLIC_KEY_FILE` | `config/keys/jwt_public_dev.pem` | Primary RSA public verification key (PEM). |
| `JWT_PRIVATE_KEY_PEM` / `JWT_PUBLIC_KEY_PEM` | — | Inline PEM for the primary key (takes precedence over the file vars). |
| `JWT_ADDITIONAL_PUBLIC_KEY_FILES` | — | Comma-separated paths to extra **verify-only** public keys. |
| `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` | — | Inline verify-only public PEMs (comma- or newline-separated). |

The **primary** key signs every new token; the **additional** keys are
verify-only — recently rotated-out public keys whose already-issued
tokens are still within their lifetime. The JWKS at
`/.well-known/jwks.json` publishes the whole set (primary first), each
under its `kid = base64url(SHA-256(public modulus))`.

Unset/empty additional vars ⇒ the service publishes and trusts exactly
one key (the original single-key behaviour, fully backward-compatible).

## Generating a keypair

```bash
# Private key (PKCS#8, RSA 2048):
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
  -out jwt_private.pem
# Public key:
openssl pkey -in jwt_private.pem -pubout -out jwt_public.pem
```

## Zero-downtime rotation runbook

Rotation is **operator-driven and config-driven** — no database, and no
auto-rotation scheduler (that is a planned follow-up). The grace window
keeps already-issued tokens valid, so there is no downtime.

1. **Generate** a new keypair (`jwt_private_new.pem` / `jwt_public_new.pem`).
2. **Promote** it to primary and **retain the old public key** as an
   additional verify-only key:

   ```bash
   export JWT_PRIVATE_KEY_FILE=config/keys/jwt_private_new.pem
   export JWT_PUBLIC_KEY_FILE=config/keys/jwt_public_new.pem
   export JWT_ADDITIONAL_PUBLIC_KEY_FILES=config/keys/jwt_public_old.pem
   ```

3. **Restart** the service. The JWKS now publishes both keys (new
   primary first, old key second). New tokens are signed by the new key;
   tokens signed by the old key still verify against the retained old
   public key. Peers refresh the JWKS at their next boot or on the first
   `UnknownKid` and trust both `kid`s.
4. **Wait** at least the max access-token lifetime (`JWT_EXPIRATION`,
   default `3600` seconds) so every token signed by the old key has
   expired.
5. **Retire** the old key: drop it from `JWT_ADDITIONAL_PUBLIC_KEY_FILES`
   and restart. The grace window in step 4 guarantees no live token is
   orphaned; afterwards a token signed by the old key is rejected
   (unknown `kid`).

See the entity spec
[`spec/08-architecture.md` §8.4](../../../spec/08-architecture.md) and the
service spec [§8](../../spec/index.md) for the architectural detail.
