//! Hash-at-rest for **bearer-equivalent secrets** (SEC-A9).
//!
//! Three server-side secrets are, in effect, bearer credentials: the
//! **magic-link token** (`users.magic_link_token`), the opaque **session
//! id** (`sessions.jid`, the value in the `__Host-mxi_session` cookie and
//! the PASETO `sid` claim), and the per-session **CSRF synchroniser token**
//! (`sessions.data.csrf`). Anyone who reads one from the database at rest
//! (a leaked backup, a SQL-injection read, an over-broad log) can replay it.
//! So the database stores only a **one-way hash**; the plaintext lives only
//! in transit (email link, cookie, header) and is never persisted.
//!
//! ## Why a fast SHA-256, not Argon2
//!
//! These tokens are **high-entropy random strings** (128–256 bits from the
//! CSPRNG), not user-chosen passwords. The threat a password hash defends
//! against — offline brute-force of a low-entropy secret — does not exist
//! here: guessing a 128-bit random token is infeasible regardless of the
//! hash's speed. What we need instead is a **deterministic, unsalted** hash
//! so the server can look a presented token up by its hash
//! (`WHERE magic_link_token = hash($presented)`) in one indexed query. A
//! salted/slow password hash (Argon2) is therefore the *wrong* tool: it
//! cannot be looked up by value, and its cost buys nothing against a
//! high-entropy input. SHA-256 gives preimage resistance (the stored hash
//! reveals nothing usable) with O(1) lookup.
//!
//! ## Encoding contract
//!
//! [`hash`] returns the **lowercase hex** SHA-256 of the secret's UTF-8
//! bytes. This is byte-for-byte identical to PostgreSQL
//! `encode(digest(secret, 'sha256'), 'hex')`, so the data-migration that
//! hashes existing rows in place (`m20220101_000009_*`) and this function
//! agree — a token hashed by the migration verifies against a token hashed
//! here, and vice-versa.

use sha2::{Digest, Sha256};
use std::fmt::Write as _;

/// One-way, deterministic hash of a bearer-equivalent secret for storage.
///
/// Returns the lowercase-hex SHA-256 of `secret` (64 chars). Deterministic,
/// so a presented plaintext can be hashed and looked up against the stored
/// hash. Never store the plaintext; hash it with this first.
#[must_use]
pub fn hash(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    // Lowercase hex, matching Postgres `encode(digest(...), 'hex')`.
    digest.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

#[cfg(test)]
mod tests {
    use super::hash;

    #[test]
    fn matches_known_sha256_vector() {
        // The canonical SHA-256 of the empty string and of "abc"
        // (FIPS 180-4 examples), lowercase hex — pins the exact encoding
        // the migration's `encode(digest(...),'hex')` must also produce.
        assert_eq!(
            hash(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn is_deterministic_and_fixed_width_lowercase_hex() {
        let a = hash("some-random-token-value");
        assert_eq!(a, hash("some-random-token-value"), "must be deterministic");
        assert_eq!(a.len(), 64, "SHA-256 hex is 64 chars");
        assert!(
            a.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "lowercase hex only"
        );
    }

    #[test]
    fn differs_per_input_and_hides_the_plaintext() {
        let secret = "plaintext-secret";
        let h = hash(secret);
        assert_ne!(h, secret, "the stored hash is not the plaintext");
        assert_ne!(hash("a"), hash("b"), "distinct inputs hash distinctly");
    }
}
