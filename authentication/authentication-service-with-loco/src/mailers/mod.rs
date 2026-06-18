//! Mailers. In development the magic link is logged to the tracing
//! console instead of being sent over SMTP.

/// Magic-link / welcome / forgot mailer.
pub mod auth;
