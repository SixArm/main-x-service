# authentication-verifier — documentation index

Offline PASETO v4.public verification for Main X Index peer services:
fetch the [authentication-service](../authentication-service-with-loco)
Ed25519 public key(s) once, then verify bearer tokens locally — no
shared secret, no introspection hop. (v0.2.0 pivots from RS256 JWT; see
[authentication-sessions.md](../../agents/share/authentication-sessions.md) §5.)

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; golden rules; layout. |
| [README.md](./README.md) | Quick start, API summary, paseto-keys/`kid` contract. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
boot          ──>  GET /.well-known/paseto-keys  (once, from the auth service)
                      │  Verifier::from_paseto_keys_value(&keys, issuer, audience)
                      │  (or Verifier::from_paseto_keys_url(...) with feature "fetch")
per request   ──>  verifier.verify(bearer_token)   // PASETO v4.public, footer kid
                      │  -> Claims { sub: pid, iss, aud, iat, nbf, exp, sid, attrs, scope/roles }
                      │  policy.evaluate(&claims, action, entity) -> Decision  // ABAC
key rotation  ──>  on VerifyError::UnknownKid, refetch the key set and rebuild —
                      or hold a ReloadableVerifier and `store()` a fresh one on a
                      timer, so rotation needs no restart (v0.8.0)
```

## Relationship to the entity

This is the **verification** third of the authentication entity
(service issues, verifier verifies, front-end signs users in). The
`Claims` struct and `kid` derivation are mirrored with the service by
convention and pinned by the service crate's cross-crate contract
test. See the entity docs: [spec](../spec/index.md) ·
[verification guide](../AGENTS/verification.md) ·
[subprojects](../AGENTS/subprojects.md).
