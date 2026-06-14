//! Auth mailer: the magic-link sign-in email plus loco's stock
//! welcome / forgot-password templates.
//!
//! The **magic-link** email is the one that matters here: it is rendered
//! from the dependency-light [`crate::i18n`] catalog (English + Welsh) so
//! no templating engine or on-disk template is needed on that path. The
//! `welcome` and `forgot` templates are loco scaffolding kept for
//! completeness; the passwordless flow never checks a password, so
//! `forgot_password` is not wired into any route today.
//!
//! In development there is no SMTP, so [`crate::controllers::auth`] logs
//! the link to the tracing console and treats a send failure as benign.
#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::models::users;

/// Embedded `welcome` email template directory (loco `mail_template`).
static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
/// Embedded `forgot`-password email template directory (loco scaffolding;
/// unused by the passwordless flow).
static forgot: Dir<'_> = include_dir!("src/mailers/auth/forgot");
// The magic-link email no longer renders from the on-disk template
// directory: it is localised via the dependency-light `crate::i18n`
// catalog (see `send_magic_link`). The `src/mailers/auth/magic_link/*.t`
// files are retained as the English reference copy.

/// Mailer for auth emails (magic-link, welcome, forgot). In development
/// the magic link is logged rather than sent (no SMTP configured).
pub struct Emailer {}
// Opt into loco's `Mailer` trait using its default `mail` / `opts` /
// `mail_template` implementations; no overrides are needed.
impl Mailer for Emailer {}
impl Emailer {
    /// Sending welcome email the the given user
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_welcome(ctx: &AppContext, user: &users::Model) -> Result<()> {
        Self::mail_template(
            ctx,
            &welcome,
            mailer::Args {
                to: user.email.clone(),
                locals: json!({
                  "name": user.name,
                  "verifyToken": user.email_verification_token,
                  "domain": ctx.config.server.full_url()
                }),
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }

    /// Sending forgot password email
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn forgot_password(ctx: &AppContext, user: &users::Model) -> Result<()> {
        Self::mail_template(
            ctx,
            &forgot,
            mailer::Args {
                to: user.email.clone(),
                locals: json!({
                  "name": user.name,
                  "resetToken": user.reset_token,
                  "domain": ctx.config.server.full_url()
                }),
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }

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
    ) -> Result<()> {
        let token = user
            .magic_link_token
            .clone()
            .ok_or_else(|| Error::string("the user model not contains magic link token"))?;
        let frontend =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
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
