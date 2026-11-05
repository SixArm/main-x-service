//! Password hashing and random strings.
//!
//! The service is passwordless, so these guard the stored placeholder and
//! keep work constant across code paths (SEC-A5). Hashes are Argon2id with
//! the library's default parameters.

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{Error, SaltString},
};
use rand::{Rng, RngCore, distr::Alphanumeric};

/// Hash `password` with a fresh random salt.
///
/// # Errors
///
/// Fails when the hasher rejects its parameters.
pub fn hash_password(password: &str) -> Result<String, Error> {
    let mut salt = [0_u8; 16];
    rand::rng().fill_bytes(&mut salt);
    let salt = SaltString::encode_b64(&salt)?;
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

/// Whether `password` matches `hashed`. A malformed hash never matches.
#[must_use]
pub fn verify_password(password: &str, hashed: &str) -> bool {
    PasswordHash::new(hashed).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

/// A random alphanumeric string, from a CSPRNG seeded by the operating
/// system.
#[must_use]
pub fn random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_verifies_against_its_hash() {
        let hashed = hash_password("correct horse").unwrap();
        assert!(hashed.starts_with("$argon2id$"));
        assert!(verify_password("correct horse", &hashed));
        assert!(!verify_password("wrong", &hashed));
    }

    #[test]
    fn hashes_are_salted() {
        assert_ne!(hash_password("x").unwrap(), hash_password("x").unwrap());
    }

    #[test]
    fn a_malformed_hash_never_matches() {
        assert!(!verify_password("x", "not-a-hash"));
    }

    #[test]
    fn random_strings_have_the_requested_length_and_differ() {
        let first = random_string(32);
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(first, random_string(32));
    }
}
