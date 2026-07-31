//! Audience rules (CMS-R20, CMS-D11) — pure, DB-free.
//!
//! Personalization here reads **only what the caller states about the
//! request**: the locale, the channel, a declared audience tag, and
//! whether this is a preview. Not cookies, not IP addresses, not user
//! agents, not referrers, not behavioural history — and there is no
//! visitor identity anywhere in this service to attach any of that to.
//!
//! That is a design boundary, not an omission. A rule engine fed only
//! by what the caller asserts cannot become a tracking system by
//! accident; one that reads request metadata can, one config change at
//! a time. Personalization that requires profiling is a different
//! product with a different privacy review.
//!
//! Two further properties follow, and both matter at the edge:
//!
//! - **Evaluation reports which rules matched**, so a puzzled editor
//!   can see why a visitor got what they got.
//! - **Evaluation reports which context keys it consulted**, so the
//!   delivery layer can vary its `ETag` and `Vary` header by exactly
//!   those. A personalized response cached under a key that ignores
//!   the thing that personalized it is a data-leak mechanism.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The request-context keys a rule may read. Anything else is refused
/// at declaration time, which is where the boundary is enforced.
pub const CONTEXT_KEYS: &[&str] = &["locale", "channel", "audience_tag", "preview"];

/// The channels a delivery request may declare.
pub const CHANNELS: &[&str] = &["web", "app", "screen", "feed"];

/// Maximum conditions in one predicate.
pub const MAX_CONDITIONS: usize = 16;
/// Maximum accepted values per condition.
pub const MAX_VALUES: usize = 32;

/// A declarative predicate over the allow-listed request context.
///
/// Every listed key must match (conjunction); a key's value list is a
/// disjunction ("any of these"); a `!`-prefixed value negates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Predicate {
    /// Context key → accepted values.
    #[serde(flatten)]
    pub conditions: BTreeMap<String, Vec<String>>,
}

/// The request context a rule is evaluated against.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// The locale being served.
    pub locale: String,
    /// The declaring channel (`web`, `app`, `screen`, `feed`).
    pub channel: String,
    /// A tag the channel asserts about itself (a kiosk's location, a
    /// campaign) — asserted, never inferred.
    pub audience_tag: Option<String>,
    /// Whether this is a preview render.
    pub preview: bool,
}

impl Context {
    /// The value of one allow-listed key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "locale" => Some(self.locale.clone()),
            "channel" => Some(self.channel.clone()),
            "audience_tag" => self.audience_tag.clone(),
            "preview" => Some(self.preview.to_string()),
            _ => None,
        }
    }
}

/// Validate a predicate, returning every problem (empty ⇒ valid).
#[must_use]
pub fn validate(predicate: &Predicate) -> Vec<String> {
    let mut problems = Vec::new();
    if predicate.conditions.is_empty() {
        problems.push("a rule must state at least one condition".to_string());
    }
    if predicate.conditions.len() > MAX_CONDITIONS {
        problems.push(format!(
            "a rule may state at most {MAX_CONDITIONS} conditions"
        ));
    }
    for (key, values) in &predicate.conditions {
        if !CONTEXT_KEYS.contains(&key.as_str()) {
            problems.push(format!(
                "{key:?} is not a request-context key; personalization may read only \
                 {CONTEXT_KEYS:?} (no cookies, IPs, user agents, or referrers)"
            ));
        }
        if values.is_empty() {
            problems.push(format!("condition {key:?} lists no values"));
        }
        if values.len() > MAX_VALUES {
            problems.push(format!(
                "condition {key:?} lists more than {MAX_VALUES} values"
            ));
        }
        if key == "channel" {
            for value in values {
                let bare = value.strip_prefix('!').unwrap_or(value);
                if !CHANNELS.contains(&bare) {
                    problems.push(format!("channel {bare:?} is not one of {CHANNELS:?}"));
                }
            }
        }
    }
    problems
}

/// Whether `context` satisfies `predicate`.
///
/// A condition on a key the context does not carry (an absent
/// `audience_tag`) does **not** match — an absent value is not a
/// wildcard, because treating it as one would silently widen every
/// rule that mentions it.
#[must_use]
pub fn matches(predicate: &Predicate, context: &Context) -> bool {
    predicate.conditions.iter().all(|(key, values)| {
        let actual = context.get(key);
        values.iter().any(|value| match value.strip_prefix('!') {
            // Negation is satisfied by an absent value: "not the kiosk
            // in reception" is true of a caller that named no kiosk.
            Some(bare) => actual.as_deref() != Some(bare),
            None => actual.as_deref() == Some(value.as_str()),
        })
    })
}

/// The context keys a predicate consults — the exact set a cache key
/// and a `Vary` header must include.
#[must_use]
pub fn consulted_keys(predicate: &Predicate) -> BTreeSet<String> {
    predicate.conditions.keys().cloned().collect()
}

/// One rule as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Stable key, reported when it matches.
    pub key: String,
    /// The predicate.
    pub predicate: Predicate,
}

/// The outcome of evaluating a rule set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Evaluation {
    /// The keys of the rules that matched, in declaration order.
    pub matched: Vec<String>,
    /// Every context key any rule consulted — what the response must
    /// vary by.
    pub consulted: Vec<String>,
}

/// Evaluate every rule against one context.
#[must_use]
pub fn evaluate(rules: &[Rule], context: &Context) -> Evaluation {
    let mut consulted = BTreeSet::new();
    let mut matched = Vec::new();
    for rule in rules {
        consulted.extend(consulted_keys(&rule.predicate));
        if matches(&rule.predicate, context) {
            matched.push(rule.key.clone());
        }
    }
    Evaluation {
        matched,
        consulted: consulted.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn predicate(pairs: &[(&str, &[&str])]) -> Predicate {
        Predicate {
            conditions: pairs
                .iter()
                .map(|(key, values)| {
                    (
                        (*key).to_string(),
                        values.iter().map(ToString::to_string).collect(),
                    )
                })
                .collect(),
        }
    }

    fn context(locale: &str, channel: &str, tag: Option<&str>) -> Context {
        Context {
            locale: locale.to_string(),
            channel: channel.to_string(),
            audience_tag: tag.map(ToString::to_string),
            preview: false,
        }
    }

    /// The boundary: a rule may not read anything the caller did not
    /// assert about the request.
    #[test]
    fn only_allow_listed_context_keys_are_accepted() {
        assert!(validate(&predicate(&[("locale", &["fr"])])).is_empty());
        for key in ["cookie", "ip", "user_agent", "referrer", "visitor_id"] {
            let problems = validate(&predicate(&[(key, &["x"])]));
            assert!(
                problems
                    .iter()
                    .any(|p| p.contains("not a request-context key")),
                "{key} should be refused"
            );
            assert!(
                problems.iter().any(|p| p.contains("no cookies")),
                "the refusal explains the boundary"
            );
        }
    }

    #[test]
    fn empty_and_oversized_predicates_are_refused() {
        assert!(!validate(&Predicate::default()).is_empty());
        assert!(!validate(&predicate(&[("locale", &[])])).is_empty());
        let many: Vec<String> = (0..=MAX_VALUES).map(|i| format!("v{i}")).collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        assert!(!validate(&predicate(&[("locale", &refs)])).is_empty());
    }

    #[test]
    fn unknown_channels_are_refused() {
        assert!(validate(&predicate(&[("channel", &["screen"])])).is_empty());
        assert!(!validate(&predicate(&[("channel", &["fax"])])).is_empty());
    }

    #[test]
    fn conditions_are_conjunctive_and_values_disjunctive() {
        let rule = predicate(&[("locale", &["fr", "fr-CA"]), ("channel", &["web"])]);
        assert!(matches(&rule, &context("fr", "web", None)));
        assert!(matches(&rule, &context("fr-CA", "web", None)));
        // Right locale, wrong channel.
        assert!(!matches(&rule, &context("fr", "app", None)));
        // Right channel, wrong locale.
        assert!(!matches(&rule, &context("en", "web", None)));
    }

    /// An absent value is not a wildcard: treating it as one would
    /// silently widen every rule that mentions the key.
    #[test]
    fn an_absent_context_value_does_not_match() {
        let rule = predicate(&[("audience_tag", &["reception-kiosk"])]);
        assert!(matches(
            &rule,
            &context("en", "screen", Some("reception-kiosk"))
        ));
        assert!(!matches(&rule, &context("en", "screen", None)));
        assert!(!matches(&rule, &context("en", "screen", Some("other"))));
    }

    /// Negation *is* satisfied by an absent value: "not the reception
    /// kiosk" is true of a caller that named no kiosk.
    #[test]
    fn negation_is_satisfied_by_absence() {
        let rule = predicate(&[("audience_tag", &["!reception-kiosk"])]);
        assert!(matches(&rule, &context("en", "web", None)));
        assert!(matches(&rule, &context("en", "web", Some("other"))));
        assert!(!matches(
            &rule,
            &context("en", "web", Some("reception-kiosk"))
        ));
    }

    #[test]
    fn preview_is_a_context_value_like_any_other() {
        let rule = predicate(&[("preview", &["true"])]);
        let mut ctx = context("en", "web", None);
        assert!(!matches(&rule, &ctx));
        ctx.preview = true;
        assert!(matches(&rule, &ctx));
    }

    /// The cache-safety property: evaluation reports every key any rule
    /// consulted, so the response can vary by exactly those.
    #[test]
    fn evaluation_reports_matches_and_the_keys_it_consulted() {
        let rules = vec![
            Rule {
                key: "french".to_string(),
                predicate: predicate(&[("locale", &["fr"])]),
            },
            Rule {
                key: "kiosk".to_string(),
                predicate: predicate(&[("channel", &["screen"]), ("audience_tag", &["lobby"])]),
            },
        ];
        let evaluation = evaluate(&rules, &context("fr", "web", None));
        assert_eq!(evaluation.matched, vec!["french".to_string()]);
        assert_eq!(
            evaluation.consulted,
            vec![
                "audience_tag".to_string(),
                "channel".to_string(),
                "locale".to_string()
            ],
            "every consulted key is reported, including from rules that did not match"
        );

        let evaluation = evaluate(&rules, &context("fr", "screen", Some("lobby")));
        assert_eq!(evaluation.matched, vec!["french", "kiosk"]);
    }

    #[test]
    fn an_empty_rule_set_matches_nothing_and_consults_nothing() {
        let evaluation = evaluate(&[], &context("en", "web", None));
        assert!(evaluation.matched.is_empty());
        assert!(evaluation.consulted.is_empty());
    }
}
