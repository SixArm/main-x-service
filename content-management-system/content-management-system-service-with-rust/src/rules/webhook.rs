//! Webhook signing and delivery policy (CMS-R23, CMS-D12) — pure,
//! DB-free.
//!
//! ## Why webhooks rather than plugins
//!
//! The "plugins" a CMS is expected to have are, here, declared outbound
//! subscriptions. Loading third-party code into a service that forbids
//! `unsafe`, gates every input, and refuses unverified security code
//! would forfeit exactly the properties this project exists to
//! demonstrate — and would do it in the one process that holds every
//! site's unpublished content.
//!
//! ## Signing
//!
//! Each delivery carries an HMAC-SHA256 over `{timestamp}.{body}`,
//! keyed by the subscription's shared secret. The timestamp is inside
//! the signed material, so a captured delivery cannot be replayed
//! later against a receiver that checks freshness — signing the body
//! alone would leave exactly that hole.
//!
//! The secret is stored recoverably (unlike a preview token, which is
//! hashed) for the unavoidable reason that the receiver must hold the
//! same secret to verify. It is returned once at registration and by no
//! read afterwards.
//!
//! ## What this module does not decide
//!
//! Whether a given host *should* be reachable is a network-egress
//! question, and pretending an application-level allow-list settles it
//! would be false comfort. What is enforced here: **HTTPS only,
//! loopback excepted**, no credentials in the URL, and (at the call
//! site) no redirects followed, a timeout, and a response-size cap.
//!
//! The loopback exception is the family's existing rule for
//! server-side fetches (`agents/share/security.md` invariant 7, as
//! applied to PASETO key fetches): plain HTTP to `127.0.0.1` never
//! leaves the host, so the confidentiality argument for requiring TLS
//! does not apply, and requiring it there would make a local receiver
//! untestable without a certificate.

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

/// The header carrying the signature.
pub const SIGNATURE_HEADER: &str = "x-cms-signature";
/// The header carrying the signed timestamp.
pub const TIMESTAMP_HEADER: &str = "x-cms-timestamp";
/// The header carrying the event id, so a receiver can dedupe.
pub const EVENT_ID_HEADER: &str = "x-cms-event-id";

/// How many attempts a delivery gets before it is abandoned.
pub const MAX_ATTEMPTS: i32 = 5;

/// Consecutive failures after which a subscription is deactivated.
///
/// A receiver that has been broken for a long time is not helped by
/// more traffic, and an endpoint that 404s forever should stop costing
/// the sender a request per event.
pub const FAILURE_DEACTIVATION_THRESHOLD: i32 = 20;

/// Sign `body` for `timestamp` with `secret`, returning the hex digest.
///
/// The signed material is `{timestamp}.{body}` — the timestamp is
/// **inside** the signature so it cannot be altered by whoever relays
/// the request.
///
/// # Panics
///
/// Cannot: HMAC accepts a key of any length, so the only error
/// `new_from_slice` can report is unreachable for this construction.
/// It is `expect`ed rather than propagated so callers are not made to
/// handle a variant that cannot occur.
#[must_use]
pub fn sign(secret: &str, timestamp: i64, body: &str) -> String {
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts a key of any length");
    mac.update(format!("{timestamp}.{body}").as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut acc, byte| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

/// Verify a signature in constant time.
///
/// Provided so a receiver written against this service — and this
/// crate's own tests — check the signature the same way. Comparing hex
/// strings with `==` would leak timing; `Mac::verify_slice` does not.
///
/// # Panics
///
/// Cannot, for the same reason as [`sign`].
#[must_use]
pub fn verify(secret: &str, timestamp: i64, body: &str, signature: &str) -> bool {
    let Ok(expected) = decode_hex(signature) else {
        return false;
    };
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts a key of any length");
    mac.update(format!("{timestamp}.{body}").as_bytes());
    mac.verify_slice(&expected).is_ok()
}

/// Decode a hex string, refusing anything malformed.
fn decode_hex(hex: &str) -> Result<Vec<u8>, ()> {
    if !hex.len().is_multiple_of(2) {
        return Err(());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2).ok_or(())?, 16).map_err(|_| ()))
        .collect()
}

/// Mint a subscription secret: 256 bits, hex-encoded.
#[must_use]
pub fn mint_secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Hosts for which plain HTTP is accepted, because the request never
/// leaves the machine.
const LOOPBACK_HOSTS: [&str; 3] = ["127.0.0.1", "localhost", "[::1]"];

/// Whether an authority is loopback (with or without a port).
fn is_loopback(authority: &str) -> bool {
    let host = authority
        .rsplit_once(':')
        .map_or(authority, |(host, port)| {
            if port.chars().all(|c| c.is_ascii_digit()) && !port.is_empty() {
                host
            } else {
                authority
            }
        });
    LOOPBACK_HOSTS.contains(&host)
}

/// Why a URL was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlRefusal {
    /// Not `https://`.
    NotHttps,
    /// Carries a username or password.
    HasCredentials,
    /// Not parseable as a URL at all.
    Malformed,
}

impl UrlRefusal {
    /// The message a caller gets.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::NotHttps => {
                "a webhook URL must be https (loopback excepted): a signed payload sent in \
                 clear text is still readable by anyone on the path"
            }
            Self::HasCredentials => {
                "a webhook URL must not embed credentials; use the signature to authenticate"
            }
            Self::Malformed => "that is not a usable URL",
        }
    }
}

/// Check a webhook URL.
///
/// # Errors
///
/// The [`UrlRefusal`], for the caller to turn into a `422`.
pub fn check_url(url: &str) -> Result<(), UrlRefusal> {
    let url = url.trim();
    let (rest, secure) = match url.strip_prefix("https://") {
        Some(rest) => (rest, true),
        None => match url.strip_prefix("http://") {
            Some(rest) => (rest, false),
            None => return Err(UrlRefusal::Malformed),
        },
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if !secure && !is_loopback(authority) {
        return Err(UrlRefusal::NotHttps);
    }
    if authority.is_empty() {
        return Err(UrlRefusal::Malformed);
    }
    if authority.contains('@') {
        return Err(UrlRefusal::HasCredentials);
    }
    if authority.contains(char::is_whitespace) {
        return Err(UrlRefusal::Malformed);
    }
    Ok(())
}

/// Whether a subscription wants this event kind.
///
/// An empty subscription list means **all kinds**: a webhook that
/// declared nothing wants everything, which is the least surprising
/// reading of an empty filter.
#[must_use]
pub fn wants(subscribed: &[String], kind: &str) -> bool {
    subscribed.is_empty() || subscribed.iter().any(|candidate| candidate == kind)
}

/// Seconds to wait before attempt `attempt` (1-based).
///
/// Exponential with a ceiling: 0, 30, 120, 480, 1920 — fast enough that
/// a brief outage recovers within minutes, slow enough that a broken
/// receiver is not hammered.
#[must_use]
pub fn backoff_secs(attempt: i32) -> i64 {
    match attempt {
        ..=1 => 0,
        2 => 30,
        3 => 120,
        4 => 480,
        _ => 1920,
    }
}

/// Whether an HTTP status counts as delivered.
///
/// Any 2xx is success. A 4xx other than 408/429 is **not** retried:
/// the receiver understood the request and rejected it, so repeating it
/// unchanged is noise.
#[must_use]
pub const fn is_success(status: u16) -> bool {
    status >= 200 && status < 300
}

/// Whether a failed status is worth retrying.
#[must_use]
pub const fn is_retryable(status: u16) -> bool {
    match status {
        408 | 429 => true,
        400..=499 => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_round_trips_and_is_stable() {
        let secret = mint_secret();
        let signature = sign(&secret, 1_800_000_000, "{\"a\":1}");
        assert_eq!(signature.len(), 64);
        assert_eq!(signature, sign(&secret, 1_800_000_000, "{\"a\":1}"));
        assert!(verify(&secret, 1_800_000_000, "{\"a\":1}", &signature));
    }

    /// The timestamp is inside the signed material, so a captured
    /// delivery cannot be replayed later with a fresh timestamp.
    #[test]
    fn the_timestamp_is_covered_by_the_signature() {
        let secret = mint_secret();
        let body = "{\"a\":1}";
        let signature = sign(&secret, 1_800_000_000, body);
        assert!(!verify(&secret, 1_800_000_001, body, &signature));
    }

    #[test]
    fn a_wrong_secret_or_body_or_signature_fails() {
        let secret = mint_secret();
        let body = "{\"a\":1}";
        let signature = sign(&secret, 100, body);
        assert!(!verify(&mint_secret(), 100, body, &signature));
        assert!(!verify(&secret, 100, "{\"a\":2}", &signature));
        assert!(!verify(&secret, 100, body, "deadbeef"));
        // Malformed signatures are refused, never panicking.
        for bad in ["", "zz", "abc", &"f".repeat(63)] {
            assert!(!verify(&secret, 100, body, bad), "{bad:?}");
        }
    }

    #[test]
    fn urls_must_be_https_and_carry_no_credentials() {
        assert!(check_url("https://hooks.example.test/cms").is_ok());
        assert!(check_url("https://hooks.example.test").is_ok());
        assert_eq!(
            check_url("http://hooks.example.test/cms"),
            Err(UrlRefusal::NotHttps)
        );
        // Loopback is the family's standing exception for server-side
        // fetches: the request never leaves the machine.
        assert!(check_url("http://127.0.0.1:9000/hook").is_ok());
        assert!(check_url("http://localhost/hook").is_ok());
        assert!(check_url("http://[::1]:8080/hook").is_ok());
        // ...and it is the *host* that earns it, not a lookalike.
        assert_eq!(
            check_url("http://127.0.0.1.evil.test/hook"),
            Err(UrlRefusal::NotHttps)
        );
        assert_eq!(
            check_url("http://localhost.evil.test/hook"),
            Err(UrlRefusal::NotHttps)
        );
        assert_eq!(
            check_url("https://user:pass@hooks.example.test/cms"),
            Err(UrlRefusal::HasCredentials)
        );
        for bad in [
            "",
            "hooks.example.test",
            "ftp://x.test",
            "https://",
            "https:// x",
        ] {
            assert!(check_url(bad).is_err(), "{bad:?} should be refused");
        }
        // Every refusal explains itself.
        for refusal in [
            UrlRefusal::NotHttps,
            UrlRefusal::HasCredentials,
            UrlRefusal::Malformed,
        ] {
            assert!(refusal.message().len() > 20);
        }
    }

    /// An empty filter means "everything" — the least surprising
    /// reading, and the one that does not silently deliver nothing.
    #[test]
    fn an_empty_subscription_wants_every_kind() {
        assert!(wants(&[], "variant_published"));
        let only = vec!["variant_published".to_string()];
        assert!(wants(&only, "variant_published"));
        assert!(!wants(&only, "asset_uploaded"));
    }

    #[test]
    fn backoff_grows_and_then_flattens() {
        let schedule: Vec<i64> = (1..=6).map(backoff_secs).collect();
        assert_eq!(schedule, vec![0, 30, 120, 480, 1920, 1920]);
        assert!(schedule.windows(2).all(|pair| pair[1] >= pair[0]));
        // Even a nonsensical attempt number is handled.
        assert_eq!(backoff_secs(0), 0);
        assert_eq!(backoff_secs(-3), 0);
    }

    /// A receiver that understood and rejected the request is not
    /// helped by the same request again.
    #[test]
    fn client_errors_are_not_retried_except_the_two_that_mean_try_again() {
        assert!(is_success(200));
        assert!(is_success(204));
        assert!(!is_success(302));
        assert!(!is_success(500));

        assert!(!is_retryable(400));
        assert!(!is_retryable(401));
        assert!(!is_retryable(404));
        assert!(!is_retryable(422));
        assert!(is_retryable(408), "request timeout means try again");
        assert!(is_retryable(429), "rate limited means try again later");
        assert!(is_retryable(500));
        assert!(is_retryable(503));
    }
}
