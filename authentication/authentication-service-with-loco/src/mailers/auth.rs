//! Auth mailer: the magic-link sign-in email.
//!
//! The **magic-link** email is the only one this crate sends: it is
//! rendered from the dependency-light [`crate::i18n`] catalog (English
//! + Welsh) so no templating engine or on-disk template is needed. The
//! loco-scaffolded `welcome` / `forgot-password` mailer functions and
//! their embedded template directories were removed (2026-09-06,
//! `spec/index.md` §13) — the passwordless flow never checks a
//! password and no route ever called them; see the CHANGELOG and
//! `spec/index.md` §5 for the retained-then-removed history.
//!
//! In development there is no SMTP, so [`crate::controllers::auth`] logs
//! the link to the tracing console and treats a send failure as benign.

use loco_rs::prelude::*;

use crate::models::users;

// The magic-link email does not render from an on-disk template
// directory: it is localised via the dependency-light `crate::i18n`
// catalog (see `send_magic_link`). The `src/mailers/auth/magic_link/*.t`
// files are retained as the English reference copy.

/// Mailer for the magic-link email. In development the magic link is
/// logged rather than sent (no SMTP configured).
pub struct Emailer {}
// Opt into loco's `Mailer` trait using its default `mail` / `opts`
// implementations; no overrides are needed.
impl Mailer for Emailer {}
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
    pub async fn send_magic_link(
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
        let opts = Self::opts();
        Self::mail(
            ctx,
            &mailer::Email {
                from: Some(opts.from),
                to: user.email.clone(),
                reply_to: opts.reply_to,
                subject,
                text,
                html,
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }
}
