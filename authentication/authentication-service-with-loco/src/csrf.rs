//! CSRF protection for **cookie-authenticated mutating requests**
//! (`agents/share/authentication-sessions.md` §4).
//!
//! The model is a **synchroniser token**: a random token is minted when a
//! session is established, **stored server-side in the session**
//! (`sessions.data.csrf`), and delivered to the client in a
//! **non-httpOnly** cookie (`__Host-mxi_csrf`) it can read plus the
//! login-response body. On a cookie-authenticated state-changing request
//! (today `POST /api/auth/token`) the client echoes the token in the
//! `X-CSRF-Token` header; the server compares it — **constant-time** —
//! against the token stored in the session and rejects a mismatch with
//! `403`. A cross-site attacker cannot read the token (same-origin
//! policy) nor set the custom header, so a forged request fails.
//!
//! This is stronger than a bare double-submit cookie (the server trusts
//! the session copy, not the cookie), and it composes with the existing
//! `Origin` allow-list backstop and `SameSite=Lax`.
//!
//! Pure helpers (build / clear / parse / compare / generate), so they are
//! unit-testable without a server or database.

/// Header the client echoes the CSRF token in on a mutating request.
pub const CSRF_HEADER: &str = "x-csrf-token";

/// Cookie name for delivering the CSRF token to the client. The
/// `__Host-` prefix host-locks it (`Secure`, `Path=/`, no `Domain`); it
/// is **not** `HttpOnly`, so the front-end/BFF can read it to echo in the
/// [`CSRF_HEADER`].
pub const CSRF_COOKIE: &str = "__Host-mxi_csrf";

/// Length (chars) of a generated CSRF token — 32 chars of the loco
/// alphanumeric random alphabet (~190 bits), plenty for an unguessable
/// per-session token.
const CSRF_TOKEN_LEN: usize = 32;

/// Mint a fresh, unguessable CSRF token for a new session.
#[must_use]
pub fn generate_token() -> String {
    loco_rs::hash::random_string(CSRF_TOKEN_LEN)
}

/// Build the `Set-Cookie` value delivering the CSRF token to the client:
/// `Secure`, `SameSite=Lax`, host-locked, **not** `HttpOnly` (so JS/the
/// BFF can read it and echo it in the header).
#[must_use]
pub fn set_csrf(token: &str) -> String {
    format!("{CSRF_COOKIE}={token}; Secure; SameSite=Lax; Path=/")
}

/// Build the `Set-Cookie` value that clears the CSRF cookie (logout).
#[must_use]
pub fn clear_csrf() -> String {
    format!("{CSRF_COOKIE}=; Secure; SameSite=Lax; Path=/; Max-Age=0")
}

/// Extract the CSRF token from a request `Cookie` header value (the full
/// `a=1; b=2` string). Returns the first `__Host-mxi_csrf` value, or
/// `None` if absent/empty. (The server validates against the session
/// copy, not this; provided for completeness / tests.)
#[must_use]
pub fn read_csrf(cookie_header: &str) -> Option<String> {
    let prefix = format!("{CSRF_COOKIE}=");
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|pair| pair.strip_prefix(&prefix))
        .map(str::to_string)
        .filter(|v| !v.is_empty())
}

/// Constant-time equality of the expected (session-stored) token and the
/// token the client provided in the header. Constant-time in the length
/// of `expected` to avoid leaking it via timing; a length mismatch is a
/// non-match. An empty expected token never matches (a session without a
/// CSRF token cannot be satisfied by an empty header).
#[must_use]
pub fn matches(expected: &str, provided: &str) -> bool {
    let (expected, provided) = (expected.as_bytes(), provided.as_bytes());
    if expected.is_empty() || expected.len() != provided.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (a, b) in expected.iter().zip(provided.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_the_expected_length_and_vary() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.chars().count(), CSRF_TOKEN_LEN);
        assert_ne!(a, b, "two generated tokens must differ");
    }

    #[test]
    fn set_csrf_is_readable_secure_host_locked() {
        let c = set_csrf("tok-123");
        assert!(c.starts_with("__Host-mxi_csrf=tok-123"));
        assert!(c.contains("Secure"));
        assert!(c.contains("SameSite=Lax"));
        assert!(c.contains("Path=/"));
        // Deliberately readable by JS/the BFF — NOT HttpOnly.
        assert!(!c.contains("HttpOnly"));
        // __Host- forbids a Domain attribute.
        assert!(!c.to_lowercase().contains("domain="));
    }

    #[test]
    fn clear_csrf_expires_immediately() {
        assert!(clear_csrf().contains("Max-Age=0"));
    }

    #[test]
    fn read_csrf_finds_the_value_among_others() {
        assert_eq!(
            read_csrf("foo=1; __Host-mxi_csrf=abc; bar=2").as_deref(),
            Some("abc")
        );
        assert_eq!(read_csrf("foo=1").as_deref(), None);
        assert_eq!(read_csrf("__Host-mxi_csrf="), None);
    }

    #[test]
    fn matches_is_exact_and_rejects_empty_or_mismatched() {
        assert!(matches("secret-token", "secret-token"));
        assert!(!matches("secret-token", "secret-toke")); // shorter
        assert!(!matches("secret-token", "secret-tokenX")); // longer
        assert!(!matches("secret-token", "another-token"));
        // An empty expected token can never be satisfied.
        assert!(!matches("", ""));
        assert!(!matches("", "anything"));
    }
}
