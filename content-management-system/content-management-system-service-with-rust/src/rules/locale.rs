//! Locale codes, the site's declared locale set, and fallback-chain
//! validity (CMS-R1, CMS-R14) — pure, DB-free.
//!
//! Codes follow the family vocabulary
//! (`agents/share/locales.md`): a two-letter ISO 639-1 language,
//! optionally with a two-letter region subtag, written
//! `language` or `language-REGION` (`fr`, `fr-CA`). The comparison is
//! **exact**: `fr-ca` is refused rather than silently normalized,
//! because a delivery route and a stored variant that disagree on case
//! would serve a 404 that nobody can explain.
//!
//! A **fallback chain** is the ordered list delivery walks when a
//! locale has no published variant. The chain is validated at write
//! time so an unwalkable chain cannot be stored: every hop must be a
//! declared locale, no hop may repeat (a cycle would never terminate),
//! and the chain must end at the site's default locale — otherwise a
//! request can fall off the end of the list with nothing to serve.
//!
//! [`resolve`] walks a chain against the locales that actually have
//! something published, and **always reports what it did**: which
//! locale was asked for, which one answers, and whether a fallback was
//! applied. A CMS that serves English under a `/fr/` URL without
//! saying so is not localized, it is lying — and readers discover the
//! lie faster than editors do.

use std::collections::BTreeSet;

/// Maximum locales one site may declare.
pub const MAX_LOCALES: usize = 64;

/// Maximum hops in one fallback chain.
pub const MAX_CHAIN_LEN: usize = 8;

/// Whether `code` is a well-formed locale code: `xx` or `xx-YY`, with a
/// lowercase two-letter language and an uppercase two-letter region.
#[must_use]
pub fn is_locale_code(code: &str) -> bool {
    let (language, region) = match code.split_once('-') {
        Some((language, region)) => (language, Some(region)),
        None => (code, None),
    };
    let language_ok = language.len() == 2 && language.bytes().all(|b| b.is_ascii_lowercase());
    let region_ok =
        region.is_none_or(|r| r.len() == 2 && r.bytes().all(|b| b.is_ascii_uppercase()));
    language_ok && region_ok
}

/// The declared locale configuration of a site, as submitted.
#[derive(Debug, Clone)]
pub struct LocaleConfig<'a> {
    /// The locale served when nothing else is asked for or available.
    pub default_locale: &'a str,
    /// Every locale this site publishes.
    pub locales: &'a [String],
    /// Per-locale ordered fallback chains (locale → the hops walked
    /// after it). A locale with no entry falls back to the default.
    pub fallback_chains: &'a [(String, Vec<String>)],
    /// Locales for which fallback is **refused** (`404` instead of
    /// another language) — safety notices, legal text.
    pub strict_locales: &'a [String],
}

/// Validate a site's locale configuration, returning every problem
/// found (empty ⇒ valid). Never panics and never partially accepts: a
/// caller maps a non-empty result to `422`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn validate(config: &LocaleConfig<'_>) -> Vec<String> {
    let mut problems = Vec::new();

    if !is_locale_code(config.default_locale) {
        problems.push(format!(
            "default_locale {:?} is not a locale code (expected `xx` or `xx-YY`)",
            config.default_locale
        ));
    }
    if config.locales.is_empty() {
        problems.push("locales must declare at least one locale".to_string());
    }
    if config.locales.len() > MAX_LOCALES {
        problems.push(format!("locales exceeds {MAX_LOCALES} entries"));
    }

    let mut seen = BTreeSet::new();
    for locale in config.locales {
        if !is_locale_code(locale) {
            problems.push(format!(
                "locales entry {locale:?} is not a locale code (expected `xx` or `xx-YY`)"
            ));
        }
        if !seen.insert(locale.as_str()) {
            problems.push(format!("locales entry {locale:?} is duplicated"));
        }
    }
    if !seen.contains(config.default_locale) {
        problems.push(format!(
            "default_locale {:?} must be one of the declared locales",
            config.default_locale
        ));
    }

    for locale in config.strict_locales {
        if !seen.contains(locale.as_str()) {
            problems.push(format!(
                "strict_locales entry {locale:?} is not a declared locale"
            ));
        }
    }
    // A strict default locale is a contradiction only if it also names a
    // chain; the default is where chains end, and refusing fallback *to*
    // it would make every chain unwalkable.
    if config
        .strict_locales
        .iter()
        .any(|l| l == config.default_locale)
    {
        problems.push(format!(
            "default_locale {:?} cannot be strict: every fallback chain ends there",
            config.default_locale
        ));
    }

    let mut chained = BTreeSet::new();
    for (locale, chain) in config.fallback_chains {
        if !seen.contains(locale.as_str()) {
            problems.push(format!(
                "fallback_chains key {locale:?} is not a declared locale"
            ));
        }
        if !chained.insert(locale.as_str()) {
            problems.push(format!("fallback_chains key {locale:?} is duplicated"));
        }
        if chain.len() > MAX_CHAIN_LEN {
            problems.push(format!(
                "fallback_chains[{locale}] exceeds {MAX_CHAIN_LEN} hops"
            ));
        }
        if chain.is_empty() {
            problems.push(format!("fallback_chains[{locale}] must not be empty"));
            continue;
        }
        let mut walked = BTreeSet::new();
        walked.insert(locale.as_str());
        for hop in chain {
            if !seen.contains(hop.as_str()) {
                problems.push(format!(
                    "fallback_chains[{locale}] hop {hop:?} is not a declared locale"
                ));
            }
            if !walked.insert(hop.as_str()) {
                problems.push(format!(
                    "fallback_chains[{locale}] repeats {hop:?}: a chain must not cycle"
                ));
            }
        }
        if chain.last().map(String::as_str) != Some(config.default_locale) {
            problems.push(format!(
                "fallback_chains[{}] must end at the default locale {:?}",
                locale, config.default_locale
            ));
        }
        if config.strict_locales.iter().any(|s| s == locale) {
            problems.push(format!(
                "fallback_chains[{locale}] is set but {locale:?} is strict: strict locales refuse fallback"
            ));
        }
    }

    problems
}

/// What a locale request resolved to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Resolution {
    /// The locale the caller asked for.
    pub locale_requested: String,
    /// The locale that answers, if any.
    pub locale_served: Option<String>,
    /// Whether the answer came from a fallback rather than the
    /// requested locale itself.
    pub fallback_applied: bool,
    /// The hops actually walked, in order — so a puzzled editor can see
    /// the path rather than infer it.
    pub chain_walked: Vec<String>,
    /// Why nothing was served, when nothing was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<&'static str>,
}

impl Resolution {
    /// Whether anything can be served.
    #[must_use]
    pub const fn is_served(&self) -> bool {
        self.locale_served.is_some()
    }
}

/// Resolve `requested` against the locales that currently have
/// something published.
///
/// The rules, in order:
///
/// 1. A locale the site does not declare resolves to nothing
///    (`undeclared`) — it is a 404, not an invitation to guess.
/// 2. If the requested locale has published content, it answers, and
///    no fallback is reported.
/// 3. A **strict** locale refuses fallback (`strict`): for safety
///    notices and legal text, showing another language is worse than
///    showing nothing.
/// 4. Otherwise the declared chain is walked (or, with no declared
///    chain, the default locale is tried) and the first hop with
///    published content answers, with `fallback_applied` set.
#[must_use]
pub fn resolve(config: &LocaleConfig<'_>, requested: &str, published: &[String]) -> Resolution {
    let has_published = |locale: &str| published.iter().any(|l| l == locale);
    let declared = |locale: &str| config.locales.iter().any(|l| l == locale);
    let mut resolution = Resolution {
        locale_requested: requested.to_string(),
        locale_served: None,
        fallback_applied: false,
        chain_walked: Vec::new(),
        refusal: None,
    };

    if !declared(requested) {
        resolution.refusal = Some("undeclared");
        return resolution;
    }
    if has_published(requested) {
        resolution.locale_served = Some(requested.to_string());
        return resolution;
    }
    if config.strict_locales.iter().any(|l| l == requested) {
        resolution.refusal = Some("strict");
        return resolution;
    }

    // A locale with no declared chain still falls back to the default;
    // declaring the obvious chain for every locale would be noise.
    let chain: Vec<String> = config
        .fallback_chains
        .iter()
        .find(|(locale, _)| locale == requested)
        .map_or_else(
            || vec![config.default_locale.to_string()],
            |(_, chain)| chain.clone(),
        );

    for hop in chain {
        resolution.chain_walked.push(hop.clone());
        if has_published(&hop) {
            resolution.locale_served = Some(hop);
            resolution.fallback_applied = true;
            return resolution;
        }
    }
    resolution.refusal = Some("nothing_published");
    resolution
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn chains(entries: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
        entries
            .iter()
            .map(|(locale, chain)| ((*locale).to_string(), strings(chain)))
            .collect()
    }

    #[test]
    fn locale_code_shape_is_exact() {
        assert!(is_locale_code("en"));
        assert!(is_locale_code("fr-CA"));
        assert!(is_locale_code("zh"));
        // Case matters in both halves, and neither half may be a
        // different length.
        assert!(!is_locale_code("EN"));
        assert!(!is_locale_code("fr-ca"));
        assert!(!is_locale_code("eng"));
        assert!(!is_locale_code("e"));
        assert!(!is_locale_code(""));
        assert!(!is_locale_code("fr-"));
        assert!(!is_locale_code("fr-CAN"));
    }

    #[test]
    fn a_well_formed_configuration_validates() {
        let locales = strings(&["en", "fr", "fr-CA"]);
        let fallbacks = chains(&[("fr-CA", &["fr", "en"]), ("fr", &["en"])]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &locales,
            fallback_chains: &fallbacks,
            strict_locales: &[],
        };
        assert!(validate(&config).is_empty(), "{:?}", validate(&config));
    }

    #[test]
    fn the_default_locale_must_be_declared() {
        let locales = strings(&["fr"]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &locales,
            fallback_chains: &[],
            strict_locales: &[],
        };
        let problems = validate(&config);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("must be one of the declared locales"));
    }

    #[test]
    fn every_hop_must_be_declared_and_the_chain_must_end_at_the_default() {
        let locales = strings(&["en", "fr"]);
        // `de` is undeclared, and the chain never reaches `en`.
        let fallbacks = chains(&[("fr", &["de"])]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &locales,
            fallback_chains: &fallbacks,
            strict_locales: &[],
        };
        let problems = validate(&config);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("is not a declared locale"))
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("must end at the default locale"))
        );
    }

    /// A chain that revisits a locale would loop forever at resolution
    /// time; it is refused at write time instead (CMS-D10's posture,
    /// applied to locales).
    #[test]
    fn a_chain_may_not_cycle() {
        let locales = strings(&["en", "fr", "fr-CA"]);
        let fallbacks = chains(&[("fr-CA", &["fr", "fr-CA", "en"])]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &locales,
            fallback_chains: &fallbacks,
            strict_locales: &[],
        };
        assert!(
            validate(&config)
                .iter()
                .any(|p| p.contains("must not cycle"))
        );
    }

    /// A chain that starts at its own key is the degenerate cycle.
    #[test]
    fn a_chain_may_not_start_with_its_own_locale() {
        let locales = strings(&["en", "fr"]);
        let fallbacks = chains(&[("fr", &["fr", "en"])]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &locales,
            fallback_chains: &fallbacks,
            strict_locales: &[],
        };
        assert!(
            validate(&config)
                .iter()
                .any(|p| p.contains("must not cycle"))
        );
    }

    #[test]
    fn strict_locales_must_be_declared_and_carry_no_chain() {
        let locales = strings(&["en", "fr"]);
        let fallbacks = chains(&[("fr", &["en"])]);
        let strict = strings(&["fr", "de"]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &locales,
            fallback_chains: &fallbacks,
            strict_locales: &strict,
        };
        let problems = validate(&config);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("\"de\" is not a declared locale"))
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("strict locales refuse fallback"))
        );
    }

    /// Making the default strict would leave every chain with nowhere
    /// to end.
    #[test]
    fn the_default_locale_cannot_be_strict() {
        let locales = strings(&["en"]);
        let strict = strings(&["en"]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &locales,
            fallback_chains: &[],
            strict_locales: &strict,
        };
        assert!(
            validate(&config)
                .iter()
                .any(|p| p.contains("cannot be strict"))
        );
    }

    #[test]
    fn caps_and_duplicates_are_reported() {
        let many: Vec<String> = (0..=MAX_LOCALES).map(|i| format!("l{i}")).collect();
        let config = LocaleConfig {
            default_locale: "en",
            locales: &many,
            fallback_chains: &[],
            strict_locales: &[],
        };
        assert!(validate(&config).iter().any(|p| p.contains("exceeds")));

        let dupes = strings(&["en", "en"]);
        let config = LocaleConfig {
            default_locale: "en",
            locales: &dupes,
            fallback_chains: &[],
            strict_locales: &[],
        };
        assert!(
            validate(&config)
                .iter()
                .any(|p| p.contains("is duplicated"))
        );
    }

    // ---- resolution -----------------------------------------------

    fn a_site<'a>(
        locales: &'a [String],
        fallbacks: &'a [(String, Vec<String>)],
        strict: &'a [String],
    ) -> LocaleConfig<'a> {
        LocaleConfig {
            default_locale: "en",
            locales,
            fallback_chains: fallbacks,
            strict_locales: strict,
        }
    }

    #[test]
    fn a_locale_with_published_content_answers_for_itself() {
        let locales = strings(&["en", "fr"]);
        let config = a_site(&locales, &[], &[]);
        let resolution = resolve(&config, "fr", &strings(&["en", "fr"]));
        assert_eq!(resolution.locale_served.as_deref(), Some("fr"));
        assert!(!resolution.fallback_applied);
        assert!(resolution.chain_walked.is_empty());
        assert!(resolution.is_served());
    }

    /// The honesty rule: a fallback is served *and reported*.
    #[test]
    fn a_fallback_is_reported_not_hidden() {
        let locales = strings(&["en", "fr", "fr-CA"]);
        let fallbacks = chains(&[("fr-CA", &["fr", "en"])]);
        let config = a_site(&locales, &fallbacks, &[]);

        // `fr` is published: the chain stops at the first hop.
        let resolution = resolve(&config, "fr-CA", &strings(&["en", "fr"]));
        assert_eq!(resolution.locale_served.as_deref(), Some("fr"));
        assert!(resolution.fallback_applied);
        assert_eq!(resolution.chain_walked, vec!["fr".to_string()]);

        // Only `en` is published: the walk is visible.
        let resolution = resolve(&config, "fr-CA", &strings(&["en"]));
        assert_eq!(resolution.locale_served.as_deref(), Some("en"));
        assert_eq!(
            resolution.chain_walked,
            vec!["fr".to_string(), "en".to_string()]
        );
    }

    /// A locale with no declared chain still falls back to the default.
    #[test]
    fn an_undeclared_chain_falls_back_to_the_default() {
        let locales = strings(&["en", "fr"]);
        let config = a_site(&locales, &[], &[]);
        let resolution = resolve(&config, "fr", &strings(&["en"]));
        assert_eq!(resolution.locale_served.as_deref(), Some("en"));
        assert!(resolution.fallback_applied);
    }

    /// Strict locales refuse fallback: showing another language is
    /// worse than showing nothing.
    #[test]
    fn a_strict_locale_refuses_to_fall_back() {
        let locales = strings(&["en", "fr"]);
        let strict = strings(&["fr"]);
        let config = a_site(&locales, &[], &strict);
        let resolution = resolve(&config, "fr", &strings(&["en"]));
        assert!(!resolution.is_served());
        assert_eq!(resolution.refusal, Some("strict"));
        assert!(resolution.chain_walked.is_empty());

        // ...but it still answers for itself when it has content.
        let resolution = resolve(&config, "fr", &strings(&["en", "fr"]));
        assert_eq!(resolution.locale_served.as_deref(), Some("fr"));
    }

    #[test]
    fn an_undeclared_locale_is_not_guessed_at() {
        let locales = strings(&["en"]);
        let config = a_site(&locales, &[], &[]);
        let resolution = resolve(&config, "de", &strings(&["en"]));
        assert!(!resolution.is_served());
        assert_eq!(resolution.refusal, Some("undeclared"));
    }

    #[test]
    fn nothing_published_anywhere_serves_nothing() {
        let locales = strings(&["en", "fr"]);
        let config = a_site(&locales, &[], &[]);
        let resolution = resolve(&config, "fr", &[]);
        assert!(!resolution.is_served());
        assert_eq!(resolution.refusal, Some("nothing_published"));
        assert_eq!(resolution.chain_walked, vec!["en".to_string()]);
    }

    /// Resolution never loops, even if a cyclic chain somehow reached
    /// storage: the walk is bounded by the chain it was given.
    #[test]
    fn resolution_is_bounded_by_the_chain() {
        let locales = strings(&["en", "fr"]);
        let fallbacks = chains(&[("fr", &["fr", "fr", "en"])]);
        let config = a_site(&locales, &fallbacks, &[]);
        let resolution = resolve(&config, "fr", &strings(&["en"]));
        assert_eq!(resolution.locale_served.as_deref(), Some("en"));
        assert_eq!(resolution.chain_walked.len(), 3);
    }
}
