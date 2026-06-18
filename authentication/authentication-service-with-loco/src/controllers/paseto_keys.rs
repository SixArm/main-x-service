//! Public key publication for offline token verification.
//!
//! Peer services fetch `/.well-known/paseto-keys` once and verify every
//! incoming PASETO `v4.public` token locally — no shared secret, no
//! introspection round-trip to this service on the hot path. The document
//! is an OKP/Ed25519 JWK set the `authentication-verifier` crate parses.

use loco_rs::prelude::*;

/// The Ed25519 public key(s) used to sign access tokens.
#[debug_handler]
async fn paseto_keys() -> Result<Response> {
    format::json(crate::auth::keys().paseto_keys.clone())
}

/// Route for `GET /.well-known/paseto-keys` — the public key set used by
/// peer services to verify PASETO `v4.public` tokens offline.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/.well-known")
        .add("/paseto-keys", get(paseto_keys))
}
