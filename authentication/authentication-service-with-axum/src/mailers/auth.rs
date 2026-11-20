//! Auth mailer: the magic-link sign-in email.
//!
//! The **magic-link** email is the only one this crate sends: it is
//! rendered from the dependency-light [`crate::i18n`] catalog (English
//! + Welsh) so no templating engine or on-disk template is needed. The
//! legacy-scaffolded `welcome` / `forgot-password` mailer functions and
//! their embedded template directories were removed (2026-09-06,
//! `spec/index.md` §13) — the passwordless flow never checks a
//! password and no route ever called them; see the CHANGELOG and
//! `spec/index.md` §5 for the retained-then-removed history.
//!
//! In development there is no SMTP, so [`crate::controllers::auth`] logs
//! the link to the tracing console and treats a send failure as benign.

use axa::prelude::*;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::MultiPart,
    transport::smtp::authentication::Credentials,
};

use crate::models::users;

// The magic-link email does not render from an on-disk template
// directory: it is localised via the dependency-light `crate::i18n`
// catalog (see `send_magic_link`). The `src/mailers/auth/magic_link/*.t`
// files are retained as the English reference copy.

/// The sender used when `MAIL_FROM` is unset.
const DEFAULT_FROM: &str = "System <system@example.com>";

/// SMTP settings, from `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE`, `SMTP_USER`,
/// and `SMTP_PASSWORD`. Implicit-TLS-free STARTTLS is used when `secure`.
#[derive(Clone)]
struct Smtp {
    /// Server host.
    host: String,
    /// Server port.
    port: u16,
    /// Require TLS.
    secure: bool,
    /// Optional credentials as (user, password).
    credentials: Option<(String, String)>,
}

impl Smtp {
    /// Settings for `environment`. Only production sends by default;
    /// `SMTP_ENABLE` overrides that, so development and test stay offline
    /// unless asked otherwise.
    fn from_env(environment: &axa::Environment) -> Option<Self> {
        Self::from_lookup(environment, |name| std::env::var(name).ok())
    }

    /// [`Self::from_env`] through `lookup`, so it can be tested without the
    /// process environment.
    fn from_lookup(
        environment: &axa::Environment,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Option<Self> {
        let var = |name: &str| lookup(name).filter(|value| !value.is_empty());
        let enabled = var("SMTP_ENABLE").map_or_else(
            || environment.is_production_like(),
            |value| matches!(value.as_str(), "1" | "true" | "yes" | "on"),
        );
        enabled.then(|| Self {
            host: var("SMTP_HOST").unwrap_or_else(|| "localhost".to_string()),
            port: var("SMTP_PORT")
                .and_then(|port| port.parse().ok())
                .unwrap_or(587),
            secure: var("SMTP_SECURE").is_none_or(|value| value != "false"),
            credentials: var("SMTP_USER")
                .map(|user| (user, var("SMTP_PASSWORD").unwrap_or_default())),
        })
    }

    /// Build the transport.
    fn transport(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
        let mut builder = if self.secure {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.host).map_err(Error::wrap)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.host)
        }
        .port(self.port);
        if let Some((user, password)) = &self.credentials {
            builder = builder.credentials(Credentials::new(user.clone(), password.clone()));
        }
        Ok(builder.build())
    }
}

/// Build the multipart message, from `MAIL_FROM` or [`DEFAULT_FROM`].
fn message(to: &str, subject: String, text: String, html: String) -> Result<Message> {
    let from = std::env::var("MAIL_FROM").unwrap_or_else(|_| DEFAULT_FROM.to_string());
    message_from(&from, to, subject, text, html)
}

/// [`message`] with the sender given.
fn message_from(
    from: &str,
    to: &str,
    subject: String,
    text: String,
    html: String,
) -> Result<Message> {
    Message::builder()
        .from(from.parse().map_err(Error::wrap)?)
        .to(to.parse().map_err(Error::wrap)?)
        .subject(subject)
        .multipart(MultiPart::alternative_plain_html(text, html))
        .map_err(Error::wrap)
}

/// Mailer for the magic-link email. In development the magic link is
/// logged rather than sent (no SMTP configured).
pub struct Emailer {}

impl Emailer {
    /// Sends a magic link authentication email to the user, rendered in
    /// the given `locale` (falls back to English for an unsupported tag).
    ///
    /// The subject and bodies come from the dependency-light
    /// [`crate::i18n`] catalog rather than the on-disk template directory,
    /// so localisation is a pure-Rust lookup with no templating engine.
    /// The magic-link URL is locale-independent and points at the
    /// front-end `verify` route (matching the dev console log).
    ///
    /// # Errors
    ///
    /// When the user has no magic-link token, or email sending fails.
    pub fn send_magic_link(
        ctx: &AppContext,
        user: &users::Model,
        locale: &str,
        frontend: &str,
    ) -> Result<()> {
        let token = user
            .magic_link_token
            .clone()
            .ok_or_else(|| Error::string("the user model not contains magic link token"))?;
        // `frontend` is the per-request, allow-listed return base resolved by
        // the controller (so each operator front-end's magic link returns to
        // its own `/verify`); the dev console log uses the same base.
        let link = format!("{frontend}/verify?token={token}");

        let (subject, text, html) = crate::i18n::magic_link_email(locale).render(&link);
        // Built before spawning so a malformed address is reported here.
        let message = message(&user.email, subject, text, html)?;
        let Some(smtp) = Smtp::from_env(&ctx.environment) else {
            return Ok(());
        };
        let transport = smtp.transport()?;
        // Sent from a detached task for two reasons. A caller must not be
        // able to tell a known address from an unknown one by how long the
        // response took (SEC-A5). And the link is a live login token, so it
        // must not be written to a job table to wait for a worker (SEC-A9).
        // A crash mid-send loses this one email; the user can ask again.
        tokio::spawn(async move {
            if let Err(error) = transport.send(message).await {
                tracing::error!(%error, "magic link email failed to send");
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_FROM, Smtp, message_from};

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        }
    }

    /// Only production sends by default; development and test stay offline.
    #[test]
    fn sending_defaults_on_in_production_only() {
        assert!(Smtp::from_lookup(&axa::Environment::Production, lookup(&[])).is_some());
        assert!(Smtp::from_lookup(&axa::Environment::Development, lookup(&[])).is_none());
        assert!(Smtp::from_lookup(&axa::Environment::Test, lookup(&[])).is_none());
    }

    /// `SMTP_ENABLE` overrides the environment in either direction.
    #[test]
    fn smtp_enable_overrides_the_environment() {
        for on in ["1", "true", "yes", "on"] {
            let s = Smtp::from_lookup(
                &axa::Environment::Development,
                lookup(&[("SMTP_ENABLE", on)]),
            );
            assert!(s.is_some(), "{on}");
        }
        let off = Smtp::from_lookup(
            &axa::Environment::Production,
            lookup(&[("SMTP_ENABLE", "false")]),
        );
        assert!(off.is_none(), "production can be kept offline");
        let blank = Smtp::from_lookup(
            &axa::Environment::Development,
            lookup(&[("SMTP_ENABLE", "")]),
        );
        assert!(blank.is_none(), "a blank value is unset, not on");
    }

    #[test]
    fn defaults_are_localhost_587_starttls_no_credentials() {
        let s = Smtp::from_lookup(&axa::Environment::Production, lookup(&[])).unwrap();
        assert_eq!(
            (s.host.as_str(), s.port, s.secure),
            ("localhost", 587, true)
        );
        assert!(s.credentials.is_none());
    }

    #[test]
    fn settings_are_read_and_a_bad_port_falls_back() {
        let s = Smtp::from_lookup(
            &axa::Environment::Production,
            lookup(&[
                ("SMTP_HOST", "smtp.example.com"),
                ("SMTP_PORT", "2525"),
                ("SMTP_SECURE", "false"),
                ("SMTP_USER", "mailer"),
                ("SMTP_PASSWORD", "pw"),
            ]),
        )
        .unwrap();
        assert_eq!(
            (s.host.as_str(), s.port, s.secure),
            ("smtp.example.com", 2525, false)
        );
        assert_eq!(
            s.credentials,
            Some(("mailer".to_string(), "pw".to_string()))
        );
        let bad = Smtp::from_lookup(
            &axa::Environment::Production,
            lookup(&[("SMTP_PORT", "not-a-port"), ("SMTP_USER", "mailer")]),
        )
        .unwrap();
        assert_eq!(bad.port, 587);
        assert_eq!(bad.credentials, Some(("mailer".to_string(), String::new())));
    }

    /// Building a transport does no I/O, so both modes build offline.
    #[test]
    fn transport_builds_with_and_without_starttls() {
        let mut s = Smtp::from_lookup(
            &axa::Environment::Production,
            lookup(&[("SMTP_USER", "mailer"), ("SMTP_PASSWORD", "pw")]),
        )
        .unwrap();
        assert!(s.transport().is_ok(), "STARTTLS relay");
        s.secure = false;
        assert!(s.transport().is_ok(), "plain relay");
    }

    #[test]
    fn a_malformed_address_is_reported_before_sending() {
        let ok = message_from(
            DEFAULT_FROM,
            "a@example.com",
            "s".into(),
            "t".into(),
            "h".into(),
        );
        assert!(ok.is_ok());
        let bad_to = message_from(
            DEFAULT_FROM,
            "not an address",
            "s".into(),
            "t".into(),
            "h".into(),
        );
        assert!(bad_to.is_err());
        let bad_from = message_from(
            "nobody",
            "a@example.com",
            "s".into(),
            "t".into(),
            "h".into(),
        );
        assert!(bad_from.is_err());
    }
}
