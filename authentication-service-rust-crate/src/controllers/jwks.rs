//! Public key publication for offline token verification.
//!
//! Peer services fetch `/.well-known/jwks.json` once and verify every
//! incoming RS256 token locally — no shared secret, no introspection
//! round-trip to this service on the hot path.

use loco_rs::prelude::*;

/// JSON Web Key Set — the RSA public key(s) used to sign access tokens.
#[debug_handler]
async fn jwks() -> Result<Response> {
    format::json(crate::auth::keys().jwks.clone())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/.well-known")
        .add("/jwks.json", get(jwks))
}
