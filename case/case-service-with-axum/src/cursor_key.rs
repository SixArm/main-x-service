//! The secret that signs this service's page cursors, through
//! `axa::cursor_pagination`.
//!
//! | Variable | Meaning |
//! |---|---|
//! | `CURSOR_KEY` | At least 32 bytes. Signs every `?cursor=`, so a caller cannot make one up, edit one, or carry one to another list |
//!
//! In production the service does not start without it, because every
//! replica must sign with the same key or a cursor from one is refused by
//! the next. Elsewhere an unset key is a random one for this process, so a
//! cursor stops working when the process restarts.

use std::sync::OnceLock;

use axa::cursor_pagination::{CursorKey, MIN_KEY_LEN};

static KEY: OnceLock<CursorKey> = OnceLock::new();

/// The key cursors are signed with: the one [`init_from_env`] read, or a
/// random one for this process when nothing called it, as in a unit test.
pub fn key() -> &'static CursorKey {
    KEY.get_or_init(random)
}

/// Read `CURSOR_KEY` once, at boot.
///
/// # Errors
///
/// When the key is shorter than [`MIN_KEY_LEN`] bytes, or unset in
/// production.
pub fn init_from_env() -> axa::Result<()> {
    let key = from(
        std::env::var("CURSOR_KEY").ok().as_deref(),
        axa::Environment::from_env().is_production_like(),
    )?;
    // A second call, as from a second router in one test process, keeps
    // the first key, so cursors already handed out still verify.
    let _ = KEY.set(key);
    Ok(())
}

/// The key `value` names, where `production` says whether a missing one
/// stops the service.
fn from(value: Option<&str>, production: bool) -> axa::Result<CursorKey> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => CursorKey::new(value.as_bytes())
            .map_err(|error| axa::Error::Message(format!("CURSOR_KEY is not usable: {error}"))),
        None if production => Err(axa::Error::Message(format!(
            "CURSOR_KEY must be set in production, to at least {MIN_KEY_LEN} bytes, \
             the same on every replica"
        ))),
        None => {
            tracing::warn!(
                "CURSOR_KEY is not set, so cursors are signed with a random key \
                 that this process forgets when it stops"
            );
            Ok(random())
        }
    }
}

/// A key from the operating system's random source.
fn random() -> CursorKey {
    let mut secret = [0u8; MIN_KEY_LEN];
    getrandom::fill(&mut secret).expect("the operating system's random source failed");
    CursorKey::new(&secret).expect("a key of MIN_KEY_LEN bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_needs_a_key() {
        assert!(from(None, true).is_err());
        assert!(from(Some("  "), true).is_err());
    }

    #[test]
    fn elsewhere_a_missing_key_is_random() {
        assert!(from(None, false).is_ok());
    }

    #[test]
    fn a_short_key_is_refused_everywhere() {
        assert!(from(Some("short"), false).is_err());
        assert!(from(Some("short"), true).is_err());
    }

    #[test]
    fn a_long_enough_key_is_taken() {
        assert!(from(Some(&"k".repeat(MIN_KEY_LEN)), true).is_ok());
    }
}
