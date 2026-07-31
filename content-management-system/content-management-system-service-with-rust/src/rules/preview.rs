//! Preview tokens (CMS-R22, CMS-D7) — the pure half: minting,
//! hashing, and deciding whether a presented token may be honoured.
//!
//! Unpublished content is the thing this service most needs to keep in:
//! pre-publication disclosure is its signature harm. A preview share is
//! therefore deliberately awkward:
//!
//! - **Scoped to one (variant, revision).** A link that follows
//!   "whatever is latest" keeps working after the share was forgotten,
//!   and starts showing content nobody meant to share.
//! - **Short-lived**, with a default measured in minutes.
//! - **Stored as a hash.** A stolen database yields no working links,
//!   and an operator reading the table cannot impersonate a share.
//! - **Revocable**, immediately.
//!
//! The token itself is 256 bits from the OS CSPRNG (via v4 UUIDs, which
//! are `getrandom`-backed), hex-encoded. That is not a hand-rolled
//! random source: the family already trusts the same generator for
//! every `pid`, and using it here keeps one dependency instead of two.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Default token lifetime, in seconds.
pub const DEFAULT_TTL_SECS: i64 = 900;
/// The longest lifetime an operator may ask for (one day).
pub const MAX_TTL_SECS: i64 = 86_400;

/// Mint a fresh token: 256 bits, hex-encoded.
///
/// Returned to the caller exactly once; only its hash is stored.
#[must_use]
pub fn mint() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// The stored form of a token.
#[must_use]
pub fn hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().fold(String::new(), |mut acc, byte| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// Clamp a requested lifetime into the permitted range.
#[must_use]
pub fn clamp_ttl(requested: Option<i64>) -> i64 {
    requested
        .unwrap_or(DEFAULT_TTL_SECS)
        .clamp(60, MAX_TTL_SECS)
}

/// Why a presented token was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// No such token (or it was never issued).
    Unknown,
    /// Past its expiry.
    Expired,
    /// Withdrawn by an operator.
    Revoked,
    /// Presented for a revision it was not minted for.
    WrongRevision,
}

impl Refusal {
    /// A message safe to return to whoever presented the token.
    ///
    /// Deliberately uniform: an unknown token and an expired one get
    /// the same answer, so the endpoint cannot be used to test whether
    /// a guessed token ever existed.
    #[must_use]
    pub const fn public_message(self) -> &'static str {
        "this preview link is not valid (it may have expired or been withdrawn)"
    }
}

/// The state of a stored token, as the caller resolved it.
#[derive(Debug, Clone, Copy)]
pub struct Stored {
    /// When it expires.
    pub expires_at: chrono::DateTime<chrono::FixedOffset>,
    /// Whether it has been revoked.
    pub revoked: bool,
    /// The revision it was minted for.
    pub revision_pid: uuid::Uuid,
}

/// Decide whether a presented token may be honoured for `revision`.
///
/// # Errors
///
/// The [`Refusal`], for the caller to audit and translate into the
/// uniform public message.
pub fn check(
    stored: Option<Stored>,
    revision: uuid::Uuid,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<(), Refusal> {
    let Some(stored) = stored else {
        return Err(Refusal::Unknown);
    };
    if stored.revoked {
        return Err(Refusal::Revoked);
    }
    if stored.expires_at <= now {
        return Err(Refusal::Expired);
    }
    if stored.revision_pid != revision {
        return Err(Refusal::WrongRevision);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn at(offset_secs: i64) -> chrono::DateTime<chrono::FixedOffset> {
        (Utc.timestamp_opt(1_800_000_000, 0).unwrap() + Duration::seconds(offset_secs)).into()
    }

    #[test]
    fn a_minted_token_is_long_and_never_repeats() {
        let a = mint();
        let b = mint();
        assert_eq!(a.len(), 64, "256 bits, hex-encoded");
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// The stored form is a hash: a stolen table yields no working
    /// links.
    #[test]
    fn hashing_is_stable_and_not_reversible_by_inspection() {
        let token = mint();
        assert_eq!(hash(&token), hash(&token));
        assert_ne!(hash(&token), token);
        assert_eq!(hash(&token).len(), 64);
        assert_ne!(hash("a"), hash("b"));
    }

    #[test]
    fn lifetimes_are_clamped_into_the_permitted_range() {
        assert_eq!(clamp_ttl(None), DEFAULT_TTL_SECS);
        assert_eq!(clamp_ttl(Some(300)), 300);
        // A one-second token is useless; a one-year token is a leak.
        assert_eq!(clamp_ttl(Some(1)), 60);
        assert_eq!(clamp_ttl(Some(i64::MAX)), MAX_TTL_SECS);
        assert_eq!(clamp_ttl(Some(-5)), 60);
    }

    fn stored(revision: uuid::Uuid, expires_in: i64, revoked: bool) -> Stored {
        Stored {
            expires_at: at(expires_in),
            revoked,
            revision_pid: revision,
        }
    }

    #[test]
    fn a_live_token_for_its_own_revision_is_honoured() {
        let revision = uuid::Uuid::new_v4();
        assert!(check(Some(stored(revision, 600, false)), revision, at(0)).is_ok());
    }

    #[test]
    fn expiry_revocation_and_scope_are_each_refused() {
        let revision = uuid::Uuid::new_v4();
        assert_eq!(check(None, revision, at(0)), Err(Refusal::Unknown));
        assert_eq!(
            check(Some(stored(revision, 600, true)), revision, at(0)),
            Err(Refusal::Revoked)
        );
        assert_eq!(
            check(Some(stored(revision, -1, false)), revision, at(0)),
            Err(Refusal::Expired)
        );
        // The property that matters most: a token minted for one
        // revision does not follow the content forward.
        assert_eq!(
            check(
                Some(stored(revision, 600, false)),
                uuid::Uuid::new_v4(),
                at(0)
            ),
            Err(Refusal::WrongRevision)
        );
    }

    /// Expiry is exclusive at the boundary: a token is dead the instant
    /// it expires, not a second later.
    #[test]
    fn expiry_is_exact() {
        let revision = uuid::Uuid::new_v4();
        assert_eq!(
            check(Some(stored(revision, 0, false)), revision, at(0)),
            Err(Refusal::Expired)
        );
        assert!(check(Some(stored(revision, 1, false)), revision, at(0)).is_ok());
    }

    /// Every refusal gives the same answer, so the endpoint cannot be
    /// used to probe whether a guessed token ever existed.
    #[test]
    fn refusals_are_indistinguishable_to_the_caller() {
        let messages: Vec<&str> = [
            Refusal::Unknown,
            Refusal::Expired,
            Refusal::Revoked,
            Refusal::WrongRevision,
        ]
        .into_iter()
        .map(Refusal::public_message)
        .collect();
        assert!(messages.windows(2).all(|pair| pair[0] == pair[1]));
        // ...and it says nothing about which revision or site was meant.
        assert!(!messages[0].contains("revision"));
    }
}
