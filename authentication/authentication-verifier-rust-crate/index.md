# authentication-verifier — documentation index

Offline RS256 JWT verification for Main X Index peer services: fetch
the [authentication-service](../authentication-service-rust-crate)
JWKS once, then verify bearer tokens locally — no shared secret, no
introspection hop.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; golden rules; layout. |
| [README.md](./README.md) | Quick start, API summary, JWKS/`kid` contract. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
boot          ──>  GET /.well-known/jwks.json   (once, from the auth service)
                      │  Verifier::from_jwks_value(&jwks, issuer, audience)
                      │  (or Verifier::from_jwks_url(...) with feature "fetch")
per request   ──>  verifier.verify(bearer_token)
                      │  -> Claims { sub: pid, email, name, iss, aud, exp, iat, jti }
key rotation  ──>  on VerifyError::UnknownKid, refetch the JWKS and rebuild
```

## Relationship to the entity

This is the **verification** third of the authentication entity
(service issues, verifier verifies, front-end signs users in). The
`Claims` struct and `kid` derivation are mirrored with the service by
convention and pinned by the service crate's cross-crate contract
test. See the entity docs: [spec](../spec/index.md) ·
[verification guide](../AGENTS/verification.md) ·
[subprojects](../AGENTS/subprojects.md).
