//! Lightweight, dependency-free localisation for user-facing emails.
//!
//! As a UK governmental-style single sign-on provider, the service has a
//! Welsh-language duty (Welsh Language (Wales) Measure 2011 / the public
//! sector's "treat Welsh no less favourably than English" expectation),
//! so the magic-link email ships in **English (`en`)** and **Welsh
//! (`cy`)**. See [`agents/share/locales.md`] for the family-wide locale
//! list; more locales are added by extending [`magic_link_email`].
//!
//! The catalog is a pure function over a locale string and is therefore
//! testable without a database or a running app. Locale selection
//! ([`select_locale`]) normalises caller input (request body field or
//! `Accept-Language`) to a supported tag, falling back to `en` for
//! anything unknown.
//!
//! [`agents/share/locales.md`]: ../../../agents/share/locales.md

/// The default locale used whenever the caller's preference is missing,
/// malformed, or unsupported.
pub const DEFAULT_LOCALE: &str = "en";

/// The locales for which user-facing copy is currently translated.
/// Extend this (and [`magic_link_email`]) to add a locale.
pub const SUPPORTED_LOCALES: &[&str] = &["en", "cy"];

/// Localised strings for one email: a `subject` line plus `text` and
/// `html` bodies. The bodies carry a single `{link}` placeholder that the
/// caller substitutes with the absolute magic-link URL — the URL itself
/// is locale-independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailStrings {
    /// Subject header.
    pub subject: &'static str,
    /// Plain-text body, with a `{link}` placeholder for the magic-link URL.
    pub text: &'static str,
    /// HTML body, with a `{link}` placeholder for the magic-link URL.
    pub html: &'static str,
}

impl EmailStrings {
    /// Render the bodies by substituting `{link}` with the given URL.
    /// Returns `(subject, text, html)` ready for an outgoing email.
    #[must_use]
    pub fn render(&self, link: &str) -> (String, String, String) {
        (
            self.subject.to_string(),
            self.text.replace("{link}", link),
            self.html.replace("{link}", link),
        )
    }
}

/// Return the magic-link email copy for `locale`, falling back to
/// [`DEFAULT_LOCALE`] (`en`) for any unsupported or unknown tag.
///
/// The lookup is case-insensitive and tolerates region subtags
/// (`cy-GB`, `en_US`) by matching on the primary language subtag.
#[must_use]
pub fn magic_link_email(locale: &str) -> EmailStrings {
    match normalise_locale(locale).as_deref() {
        Some("cy") => EmailStrings {
            subject: "Eich dolen hud i fewngofnodi",
            text: "Helo,\n\nCliciwch y ddolen isod i fewngofnodi. Mae'r ddolen \
                   yn dod i ben yn fuan ac yn gweithio unwaith yn unig:\n\n{link}\n\n\
                   Os na wnaethoch chi ofyn am hyn, gallwch anwybyddu'r e-bost hwn.",
            html: "<html>\n<body>\n<p>Helo,</p>\n<p>Cliciwch y ddolen isod i \
                   fewngofnodi. Mae'r ddolen yn dod i ben yn fuan ac yn gweithio \
                   unwaith yn unig:</p>\n<p><a href=\"{link}\">Mewngofnodi</a></p>\n\
                   <p>Os na wnaethoch chi ofyn am hyn, gallwch anwybyddu'r e-bost \
                   hwn.</p>\n</body>\n</html>",
        },
        // `en` and any unsupported locale.
        _ => EmailStrings {
            subject: "Your magic sign-in link",
            text: "Hello,\n\nClick the link below to sign in. The link expires \
                   shortly and works only once:\n\n{link}\n\n\
                   If you did not request this, you can ignore this email.",
            html: "<html>\n<body>\n<p>Hello,</p>\n<p>Click the link below to \
                   sign in. The link expires shortly and works only once:</p>\n\
                   <p><a href=\"{link}\">Sign in</a></p>\n\
                   <p>If you did not request this, you can ignore this email.</p>\n\
                   </body>\n</html>",
        },
    }
}

/// Normalise raw locale input to a supported primary-language tag, or
/// `None` if unsupported. Lower-cases, trims, and drops any region /
/// script subtag (`cy-GB` → `cy`, `EN_us` → `en`).
fn normalise_locale(locale: &str) -> Option<String> {
    let primary = locale
        .trim()
        .split([',', ';'])
        .next()
        .unwrap_or("")
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if SUPPORTED_LOCALES.contains(&primary.as_str()) {
        Some(primary)
    } else {
        None
    }
}

/// Choose a supported locale from an optional caller preference.
///
/// The preference is the request body's optional `locale` field (the
/// mechanism the service uses — see service spec §6). Unknown, malformed,
/// or absent input maps to [`DEFAULT_LOCALE`]. A region subtag is
/// accepted and reduced to its primary language (`cy-GB` → `cy`).
#[must_use]
pub fn select_locale(preference: Option<&str>) -> String {
    preference
        .and_then(normalise_locale)
        .unwrap_or_else(|| DEFAULT_LOCALE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_strings_are_the_english_copy() {
        let en = magic_link_email("en");
        assert_eq!(en.subject, "Your magic sign-in link");
        assert!(en.text.contains("Click the link below"));
        assert!(en.html.contains("Sign in</a>"));
    }

    #[test]
    fn welsh_strings_are_the_welsh_copy() {
        let cy = magic_link_email("cy");
        assert_eq!(cy.subject, "Eich dolen hud i fewngofnodi");
        assert!(cy.text.contains("Cliciwch y ddolen"));
        assert!(cy.html.contains("Mewngofnodi</a>"));
    }

    #[test]
    fn unknown_locale_falls_back_to_english() {
        // A real, listed locale we have not translated, plus pure garbage,
        // both fall back to the English copy.
        for unknown in ["zz", "fr", "de", "", "not-a-locale"] {
            assert_eq!(
                magic_link_email(unknown),
                magic_link_email("en"),
                "locale {unknown:?} should fall back to en"
            );
        }
    }

    #[test]
    fn region_subtag_is_reduced_to_primary_language() {
        assert_eq!(magic_link_email("cy-GB"), magic_link_email("cy"));
        assert_eq!(magic_link_email("EN_us"), magic_link_email("en"));
    }

    #[test]
    fn render_substitutes_the_link_in_both_bodies() {
        let link = "https://example.test/verify?token=abc";
        let (subject, text, html) = magic_link_email("en").render(link);
        assert_eq!(subject, "Your magic sign-in link");
        assert!(text.contains(link));
        assert!(html.contains(&format!("href=\"{link}\"")));
        assert!(!text.contains("{link}"), "placeholder must be consumed");
        assert!(!html.contains("{link}"), "placeholder must be consumed");
    }

    #[test]
    fn select_locale_maps_input_to_a_supported_tag() {
        assert_eq!(select_locale(Some("cy")), "cy");
        assert_eq!(select_locale(Some("CY")), "cy");
        assert_eq!(select_locale(Some("cy-GB")), "cy");
        assert_eq!(select_locale(Some("en")), "en");
    }

    #[test]
    fn select_locale_falls_back_for_missing_or_unsupported() {
        assert_eq!(select_locale(None), "en");
        assert_eq!(select_locale(Some("")), "en");
        assert_eq!(select_locale(Some("fr")), "en");
        assert_eq!(select_locale(Some("garbage")), "en");
    }

    #[test]
    fn supported_locales_each_have_distinct_copy() {
        // Guards against accidentally shipping a locale that silently
        // renders English (i.e. a missing translation).
        for &loc in SUPPORTED_LOCALES {
            if loc == DEFAULT_LOCALE {
                continue;
            }
            assert_ne!(
                magic_link_email(loc),
                magic_link_email(DEFAULT_LOCALE),
                "locale {loc} must have its own copy"
            );
        }
    }
}
