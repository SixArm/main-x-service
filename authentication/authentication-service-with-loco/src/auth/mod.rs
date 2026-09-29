//! Authentication primitives: the access-token [`Claims`] shape and the
//! [`AuthUser`] extractor that authenticates the bearer-gated routes.
//!
//! * With the default-on `paseto` Cargo feature, `AuthUser` verifies a
//!   PASETO v4.public bearer and the [`paseto`] submodule (token
//!   issuance, key set, publication) is compiled in.
//! * Without it there is **no token issuance**; `AuthUser` instead
//!   authenticates the `__Host-mxi_session` cookie session (with the CSRF
//!   synchroniser token required on unsafe methods), yielding the same
//!   [`Claims`] built from the session.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "paseto")]
mod paseto;
#[cfg(feature = "paseto")]
pub use paseto::*;

#[cfg(not(feature = "paseto"))]
mod session;

/// Access-token claims. `sub` carries the user's public id (`pid`). The
/// field set is a byte-for-byte contract with the peer-side
/// `authentication_verifier::Claims`, pinned by the cross-crate contract
/// test (`tests/sign_verify_contract.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user `pid` (UUID string).
    pub sub: String,
    /// User email, for convenience at the edge.
    pub email: String,
    /// Display name.
    pub name: String,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: String,
    /// Expiry (unix seconds).
    pub exp: i64,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Not-before (unix seconds); omitted from the wire form when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    /// Session id — the originating server-side session (`sessions.jid`),
    /// enabling correlation and revocation.
    pub sid: String,
    /// Granted scopes, if any.
    #[serde(default)]
    pub scope: Vec<String>,
    /// Granted roles, if any.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Subject attributes for ABAC authorization — a string→strings map
    /// minted from the user's assigned attributes (e.g.
    /// `access: ["write"]`, `dept: ["cardiology"]`, `svc: ["true"]` for
    /// machine peers). Multi-valued keys mean "has each of these values";
    /// policies match set-membership; unknown attributes are inert
    /// (forward-compatible). Absent on the wire (old tokens) ⇒ empty map
    /// — no re-issue needed. Evaluated by the peer-side
    /// `authentication_verifier::abac` engine per
    /// `agents/share/authorization-attributes.md` §2–§3.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, Vec<String>>,
}

/// Authenticated caller for the bearer-gated routes. Yields the verified
/// [`Claims`]. Stateless with the `paseto` feature (revocation is
/// enforced separately by handlers that consult the `sessions` table);
/// session-cookie backed without it.
pub struct AuthUser(pub Claims);
