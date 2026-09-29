//! Offline PASETO v4.public token verification against the
//! authentication-service's published Ed25519 keys.
//!
//! # The offline verification model
//!
//! The [`authentication-service`] is the federation's single auth
//! provider. A user session lives server-side (a Postgres-backed cookie
//! session); from that session the service mints short-lived **PASETO
//! v4.public** access tokens and publishes its **Ed25519 public keys** at
//! `/.well-known/paseto-keys`. Every other service verifies those tokens
//! *offline*: fetch the key set once at boot, build a `Verifier`, then
//! call `Verifier::verify` per request. There is no shared secret and
//! no per-request introspection call.
//!
//! "Offline" is the key property and the reason this crate exists.
//! Because v4.public is *asymmetric* (Ed25519), the signer (auth-service)
//! holds the private key and the verifiers (every peer service) hold only
//! the public key. A peer can therefore confirm a token's authenticity
//! with no network round-trip on the hot path, no shared symmetric secret
//! to distribute, and no introspection endpoint to depend on for
//! availability. The only network interaction is the one-time (or rare)
//! key fetch, available out of band or via the optional
//! [`fetch`](crate#features) feature.
//!
//! This replaces the crate's previous RS256-JWT + JWKS design (≤ 0.1.x):
//! PASETO is a versioned, misuse-resistant token format with no algorithm
//! agility, so the "alg confusion" / `alg=none` class of JWT attacks does
//! not exist. See [`agents/share/authentication-sessions.md`] in the
//! monorepo for the family-wide design.
//!
//! # Authorization (ABAC)
//!
//! Since 0.3 the crate is also the family's shared **authorization**
//! foundation: verified [`Claims`] carry the subject's attributes in the
//! [`attrs`](Claims::attrs) claim, and the [`abac`] module provides the
//! pure policy engine ([`Policy`], [`Rule`], [`Action`],
//! [`Policy::evaluate`] → [`Decision`]) that the nine entity services
//! call from their blanket `/api/*` guards. See
//! [`agents/share/authorization-attributes.md`] for the design.
//!
//! # Security properties
//!
//! - **Asymmetric trust.** Verifiers never possess signing material, so a
//!   compromised peer cannot mint tokens — it can only verify them.
//! - **No algorithm agility.** The token header is the literal string
//!   `v4.public`; there is no `alg` field to downgrade and no `none`.
//! - **Key selection by `kid`.** The verifying key is chosen by the
//!   token's (authenticated) footer `kid`; a forged or stale `kid` simply
//!   matches no known key.
//! - **Issuer / audience / expiry enforcement.** Beyond a valid
//!   signature, every token must carry the expected `iss`, the expected
//!   `aud`, an unexpired `exp`, and (if present) a satisfied `nbf`.
//!
//! # Features
//!
//! - `paseto` (**on by default**) — PASETO v4.public verification:
//!   `Verifier`, `ReloadableVerifier`. Build with
//!   `default-features = false` for a deployment whose only bearer
//!   credential is a Keycloak access token; [`Claims`] and the [`abac`]
//!   engine are always available. Doctests in this page use `Verifier`,
//!   so run `cargo test --doc` with the default features.
//! - `keycloak` (off by default) — adds the [`keycloak`] module: verify
//!   a Keycloak (OIDC) access token and map it onto [`Claims`], so the
//!   ABAC engine treats it exactly like a PASETO token.
//! - `fetch` (off by default; implies `paseto`) — adds `Verifier::from_paseto_keys_url`,
//!   which pulls the key set over HTTPS via `reqwest` (rustls). With the
//!   feature off the crate does no I/O and the caller supplies the key set
//!   as a [`serde_json::Value`].
//!
//! # Example
//!
//! ```no_run
//! # use authentication_verifier::Verifier;
//! // Normally the keys come from the auth-service; an empty set here
//! // keeps the doctest offline and dependency-free.
//! let keys: serde_json::Value = serde_json::json!({ "keys": [] });
//! let verifier = Verifier::from_paseto_keys_value(&keys, "authentication-service", "main-x-service")?;
//! let claims = verifier.verify("v4.public...")?;
//! println!("authenticated subject: {}", claims.sub);
//! # Ok::<(), authentication_verifier::VerifyError>(())
//! ```
//!
//! [`authentication-service`]: https://github.com/sixarm/authentication-service-with-loco
//! [`agents/share/authentication-sessions.md`]: https://github.com/sixarm/main-x-service
//! [`agents/share/authorization-attributes.md`]: https://github.com/sixarm/main-x-service

// Reject `unsafe` outright: this crate touches only safe, allocation-light
// code paths and a security library has no business reaching for `unsafe`.
#![forbid(unsafe_code)]
// Opt into clippy's pedantic lints to keep the public-facing library tidy.
#![warn(clippy::pedantic)]
// A published library must document every public item; fail the build if
// any item lacks a doc comment.
#![deny(missing_docs)]

use std::collections::BTreeMap;

// `serde` derives let `Claims` deserialize from the token payload (and
// serialize again, which the tests rely on to mint tokens).
use serde::{Deserialize, Serialize};

pub mod abac;

// PASETO v4.public verification (`Verifier`, `ReloadableVerifier`). On by
// default; turn it off (`default-features = false`) for a deployment whose
// only bearer credential is a Keycloak/OIDC access token (`keycloak`).
#[cfg(feature = "paseto")]
mod paseto;
#[cfg(feature = "paseto")]
pub use paseto::{ReloadableVerifier, Verifier};

// Keycloak (OIDC) access-token verification, mapped onto the same `Claims`.
#[cfg(feature = "keycloak")]
pub mod keycloak;

// A local OIDC provider for tests (never for production builds).
#[cfg(feature = "test-idp")]
pub mod test_idp;

// Re-export the ABAC engine types at the crate root so callers can use
// `authentication_verifier::{Policy, Action, ...}` alongside `Verifier`
// and `Claims` without spelling the module path.
pub use abac::{Action, ActionPattern, Decision, Effect, Policy, ReloadablePolicy, Rule};

/// Verified token claims. Mirrors the auth-service `Claims` exactly so a
/// token signed there round-trips here. `sub` carries the user `pid`.
///
/// The field set is a contract with the auth-service: the service defines
/// an identical struct, and changing one without the other breaks token
/// round-tripping. `exp` / `iat` / `nbf` are unix seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user `pid` (UUID string); the stable identifier a
    /// peer service keys its authorization on.
    pub sub: String,
    /// User email, surfaced for convenience at the edge; not used for
    /// authorization decisions.
    pub email: String,
    /// Human-readable display name carried alongside the subject.
    pub name: String,
    /// Issuer (`iss`) — the auth-service that minted the token. Checked
    /// against the verifier's configured issuer.
    pub iss: String,
    /// Audience (`aud`) — the intended recipient service. Checked against
    /// the verifier's configured audience so a token issued for one peer
    /// cannot be replayed against another.
    pub aud: String,
    /// Expiry (`exp`), unix seconds. Tokens at or past this instant are
    /// rejected. Issued ~5 minutes out (the session is the durable thing).
    pub exp: i64,
    /// Issued-at (`iat`), unix seconds — when the token was minted.
    pub iat: i64,
    /// Not-before (`nbf`), unix seconds. When present, tokens before this
    /// instant are rejected. Omitted from the wire form when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    /// Session id (`sid`) — the originating server-side session, so a
    /// token can be correlated back to (and revoked with) its session.
    pub sid: String,
    /// Granted scopes, if any. Empty when the token carries none.
    ///
    /// **Deprecated for authorization** (kept on the wire for
    /// compatibility; removal is a future major): the ABAC guard ignores
    /// `scope` and decides from [`attrs`](Self::attrs) instead. See
    /// `agents/share/authorization-attributes.md` §3.
    #[serde(default)]
    pub scope: Vec<String>,
    /// Granted roles, if any. Empty when the token carries none.
    ///
    /// **Deprecated for authorization** (kept on the wire for
    /// compatibility; removal is a future major): the ABAC guard ignores
    /// `roles` and decides from [`attrs`](Self::attrs) instead — a role,
    /// where one is wanted, is just another attribute (`role=editor`).
    /// See `agents/share/authorization-attributes.md` §3.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Subject attributes for ABAC authorization — a string→strings map
    /// minted by the auth-service from the user's assigned attributes
    /// (e.g. `access: ["write"]`, `dept: ["cardiology"]`,
    /// `svc: ["true"]` for machine peers). Multi-valued keys mean "has
    /// each of these values"; policies match set-membership; unknown
    /// attributes are inert (forward-compatible). Absent on the wire
    /// (old tokens) ⇒ empty map — no re-issue needed. Evaluated by the
    /// [`abac`] engine per `agents/share/authorization-attributes.md`
    /// §2–§3, alongside the pseudo-attributes `sub` and `email`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, Vec<String>>,
}

/// Failure modes for key-set loading and token verification.
///
/// Every fallible entry point returns this type, and every variant is a
/// *handled* outcome — the crate never panics on bad input, so a malformed
/// key set and a forged token are both ordinary `Err` values.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// The key-set document was missing or structurally invalid (no `keys`
    /// array, a key missing `kid` / `x`, or an `x` that is not a 32-byte
    /// base64url Ed25519 public key). Raised at load time, not per token.
    #[error("malformed key set: {0}")]
    Keys(String),
    /// The token was not a structurally valid `v4.public` token, or its
    /// footer could not be decoded as `{ "kid": ... }`. Distinct from a
    /// signature failure ([`Paseto`](Self::Paseto)).
    #[error("malformed token: {0}")]
    Malformed(String),
    /// The token footer carried no `kid`, so no key could be selected.
    /// All auth-service tokens stamp a footer `kid`, so this indicates a
    /// hand-built or non-conforming token.
    #[error("token footer has no kid")]
    MissingKid,
    /// No verification key matched the token's footer `kid` (stale cache,
    /// wrong issuer, or forgery). The wrapped `String` is the unmatched
    /// `kid`; on a legitimate stale cache a caller may refetch and retry.
    #[error("no verification key for kid {0:?}")]
    UnknownKid(String),
    /// PASETO parsing or Ed25519 signature verification failed. Carries the
    /// stringified underlying `rusty_paseto` error.
    #[error("token verification failed: {0}")]
    Paseto(String),
    /// The signature was valid but a registered claim did not satisfy the
    /// policy: wrong `iss`, wrong `aud`, expired `exp`, or unmet `nbf`.
    #[error("claim rejected: {0}")]
    Claim(String),
    /// The token's `kid` selected a key whose algorithm this build does
    /// not implement.
    ///
    /// Distinct from [`UnknownKid`](VerifyError::UnknownKid) on purpose.
    /// Both reject the token, but they mean different things to whoever
    /// is on call: `UnknownKid` says "I hold no key for this signer" and
    /// invites a key-set refetch; this says "I hold the key and cannot
    /// use it", which a refetch will never fix. It is the expected error
    /// during a partial algorithm rollout — the issuer has moved ahead of
    /// this verifier, and this binary needs upgrading.
    #[error("key {kid:?} uses unsupported algorithm {algorithm:?}")]
    UnsupportedAlgorithm {
        /// The `kid` that selected the key.
        kid: String,
        /// The algorithm label as advertised in the key set.
        algorithm: String,
    },

    /// A Keycloak access token was rejected or the Keycloak verifier could
    /// not be built (only with the `keycloak` feature): bad signature,
    /// wrong audience, expired, unreachable realm, invalid settings.
    #[cfg(feature = "keycloak")]
    #[error("keycloak: {0}")]
    Keycloak(String),

    /// Fetching the key set over HTTP failed (only with the `fetch`
    /// feature): transport error, non-2xx status, or undecodable body.
    #[cfg(feature = "fetch")]
    #[error("key set fetch failed: {0}")]
    Fetch(String),
}
