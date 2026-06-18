//! The `__Host-mxi_session` session cookie.
//!
//! The durable session is server-side; the browser holds only an opaque
//! session id in an httpOnly, Secure, host-locked cookie (it cannot read
//! it from JS). See [`agents/share/authentication-sessions.md`] §3. The
//! SvelteKit front-ends adopt a BFF: their server holds this cookie and
//! exchanges it for a short-lived PASETO via `POST /api/auth/token`.
//!
//! These are pure string helpers (build / clear / parse), so they are
//! unit-testable without a server or a database.
//!
//! [`agents/share/authentication-sessions.md`]: https://github.com/sixarm/main-x-service

/// Cookie name. The `__Host-` prefix requires `Secure`, `Path=/`, and no
/// `Domain`, which host-locks the cookie (no subdomain can set or read it).
pub const SESSION_COOKIE: &str = "__Host-mxi_session";

/// Build the `Set-Cookie` value that establishes the session, carrying the
/// opaque session id `sid`. `HttpOnly` (no JS access), `Secure` (HTTPS
/// only), `SameSite=Lax`, host-locked via the `__Host-` prefix.
#[must_use]
pub fn set_session(sid: &str) -> String {
    format!("{SESSION_COOKIE}={sid}; HttpOnly; Secure; SameSite=Lax; Path=/")
}

/// Build the `Set-Cookie` value that clears the session (logout): empty
/// value with `Max-Age=0` so the browser drops it immediately. Same
/// attributes as [`set_session`] so the browser matches and replaces it.
#[must_use]
pub fn clear_session() -> String {
    format!("{SESSION_COOKIE}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0")
}

/// Extract the session id from a request `Cookie` header value (the full
/// `a=1; b=2` string). Returns the first `__Host-mxi_session` value, or
/// `None` if absent/empty.
#[must_use]
pub fn read_session(cookie_header: &str) -> Option<String> {
    let prefix = format!("{SESSION_COOKIE}=");
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|pair| pair.strip_prefix(&prefix))
        .map(str::to_string)
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_session_is_httponly_secure_host_locked() {
        let c = set_session("abc-123");
        assert!(c.starts_with("__Host-mxi_session=abc-123"));
        assert!(c.contains("HttpOnly"));
        assert!(c.contains("Secure"));
        assert!(c.contains("SameSite=Lax"));
        assert!(c.contains("Path=/"));
        // __Host- prefix forbids a Domain attribute.
        assert!(!c.to_lowercase().contains("domain="));
    }

    #[test]
    fn clear_session_expires_immediately() {
        let c = clear_session();
        assert!(c.contains("Max-Age=0"));
        assert!(c.contains("HttpOnly"));
    }

    #[test]
    fn read_session_finds_the_value_among_others() {
        assert_eq!(
            read_session("foo=1; __Host-mxi_session=sid-42; bar=2").as_deref(),
            Some("sid-42")
        );
        assert_eq!(read_session("__Host-mxi_session=only").as_deref(), Some("only"));
    }

    #[test]
    fn read_session_absent_or_empty_is_none() {
        assert_eq!(read_session("foo=1; bar=2"), None);
        assert_eq!(read_session(""), None);
        assert_eq!(read_session("__Host-mxi_session="), None);
    }
}
