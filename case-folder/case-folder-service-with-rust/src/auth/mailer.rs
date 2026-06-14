//! Email delivery for magic links.
//!
//! The repo ships [`LogMailer`], which writes the link to the tracing
//! log so the sign-in flow works end-to-end with no SMTP server —
//! mirroring the in-process upstream stubs. A real `SmtpMailer` is a
//! production task (see `spec/auth.md`).

use crate::auth::Identity;

/// Sends magic-link sign-in emails. Infallible at this layer — delivery
/// failures are logged, never surfaced to the caller, so we don't leak
/// whether an email matched a known identity.
pub trait Mailer: Send + Sync {
    fn send_magic_link(&self, to: &Identity, link: &str);
}

/// Writes the magic link to the log instead of sending real email.
pub struct LogMailer;

impl Mailer for LogMailer {
    /// Logs the recipient email and magic link at `INFO` level instead
    /// of sending real email, so the sign-in flow works without SMTP.
    fn send_magic_link(&self, to: &Identity, link: &str) {
        tracing::info!(
            email = %to.email,
            magic_link = %link,
            "magic-link email (LogMailer) — open this link to sign in"
        );
    }
}
