# Cross-service token signing keys

Source of truth:
[`agents/share/authentication-sessions.md`](../../../../agents/share/authentication-sessions.md).
The authentication service signs cross-service tokens as **PASETO
v4.public** (Ed25519) and publishes the public key(s) at
`/.well-known/paseto-keys`. (RS256 JWT + JWKS were the previous model and
are **decommissioned** — the old dev RSA keypair has been removed.)

> **No key files are committed.** Development uses a built-in Ed25519 dev
> seed baked into [`src/auth/mod.rs`](../../src/auth/mod.rs) (`DEV_SEED`),
> so `cargo test` and local runs work offline with a stable key. In
> production, supply the seed from the environment (below); never rely on
> the dev seed. This directory now holds only this doc.

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `TOKEN_PRIVATE_KEY_SEED` | — | Primary Ed25519 signing seed, 32 bytes base64url (no pad). Takes precedence over the file var. |
| `TOKEN_PRIVATE_KEY_FILE` | — | Path to a file holding the same base64url seed. |
| `TOKEN_ADDITIONAL_PUBLIC_KEYS` | — | Comma-separated base64url 32-byte Ed25519 **verify-only** public keys (rotated-out keys). |
| `TOKEN_ISSUER` | `authentication-service` | `iss` claim + key-set issuer. |
| `TOKEN_AUDIENCE` | `main-x-service` | `aud` claim — the federation audience. |
| `TOKEN_EXPIRATION` | `300` | Access-token lifetime (seconds). Deliberately short — the cookie session is the durable thing. |

Unset seed env ⇒ the built-in dev seed (development only). The **primary**
key signs every new token; **additional** keys are verify-only — recently
rotated-out public keys whose already-issued tokens are still within their
lifetime. The key set at `/.well-known/paseto-keys` publishes the whole set
(primary first), each under its `kid = base64url(SHA-256(public key bytes))`.

## Generating a key

A v4.public signing key is a 32-byte Ed25519 seed. Any source of 32 random
bytes works; encode it base64url (no padding):

```bash
# 32 random bytes → base64url (no padding):
head -c 32 /dev/urandom | basenc --base64url | tr -d '='
```

Set the result as `TOKEN_PRIVATE_KEY_SEED` (or write it to a file named by
`TOKEN_PRIVATE_KEY_FILE`). The service derives the public key and `kid` at
boot and publishes them.

## Zero-downtime rotation runbook

Rotation is **operator-driven and config-driven** — no database, no
auto-rotation scheduler (a planned follow-up). The grace window keeps
already-issued tokens valid, so there is no downtime.

1. **Generate** a new seed (as above). Derive its public key (the service
   logs/publishes it, or compute it from the seed with any Ed25519 tool).
2. **Promote** the new seed to primary and **retain the old public key**
   as a verify-only additional key:

   ```bash
   export TOKEN_PRIVATE_KEY_SEED=<new base64url seed>
   export TOKEN_ADDITIONAL_PUBLIC_KEYS=<old base64url public key>
   ```

3. **Restart** the service. The key set now publishes both keys (new
   primary first, old key second). New tokens are signed by the new key;
   tokens signed by the old key still verify against the retained old
   public key. Peers refresh the key set at their next boot or on the
   first `UnknownKid` and trust both `kid`s.
4. **Wait** at least the max access-token lifetime (`TOKEN_EXPIRATION`,
   default `300` seconds) so every token signed by the old key has expired.
5. **Retire** the old key: drop it from `TOKEN_ADDITIONAL_PUBLIC_KEYS` and
   restart. The grace window in step 4 guarantees no live token is
   orphaned; afterwards a token signed by the old key is rejected
   (unknown `kid`).

See the entity spec
[`spec/08-architecture.md`](../../../spec/08-architecture.md) and the
service spec [§8](../../spec/index.md) for the architectural detail.
