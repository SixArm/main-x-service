//! Segment evaluation (CRM-R7, CRM-D6): a declarative filter over
//! contact facts, evaluated in pure code, with the **consent
//! AND-gate built into the evaluator** — a segment cannot be
//! expressed that bypasses `marketing_consent = granted`.

use serde::{Deserialize, Serialize};

/// The declarative filter (all present clauses AND together).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Contact statuses to include (empty = any).
    #[serde(default)]
    pub statuses: Vec<String>,
    /// Preferred channels to include (empty = any).
    #[serde(default)]
    pub channels: Vec<String>,
    /// Account tiers to include (empty = any; contacts with no
    /// account match only when this is empty).
    #[serde(default)]
    pub account_tiers: Vec<String>,
}

/// The contact facts the evaluator consumes.
#[derive(Debug, Clone)]
pub struct ContactFacts {
    /// `marketing_consent` state token.
    pub consent: String,
    /// Contact status token.
    pub status: String,
    /// Preferred channel token.
    pub channel: String,
    /// The linked account's tier, when any.
    pub account_tier: Option<String>,
}

/// Whether the contact is in the segment. **Consent is `ANDed`
/// structurally**: a non-`granted` contact never matches, whatever
/// the filter says.
#[must_use]
pub fn matches(filter: &Filter, facts: &ContactFacts) -> bool {
    if facts.consent != "granted" {
        return false;
    }
    let status_ok = filter.statuses.is_empty() || filter.statuses.contains(&facts.status);
    let channel_ok = filter.channels.is_empty() || filter.channels.contains(&facts.channel);
    let tier_ok = filter.account_tiers.is_empty()
        || facts
            .account_tier
            .as_ref()
            .is_some_and(|tier| filter.account_tiers.contains(tier));
    status_ok && channel_ok && tier_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(consent: &str) -> ContactFacts {
        ContactFacts {
            consent: consent.to_string(),
            status: "active".to_string(),
            channel: "email".to_string(),
            account_tier: Some("customer".to_string()),
        }
    }

    /// The consent gate is structural: no filter matches a withdrawn
    /// or never-consented contact — even the empty filter.
    #[test]
    fn consent_gate_cannot_be_expressed_away() {
        let all = Filter::default();
        assert!(matches(&all, &facts("granted")));
        assert!(!matches(&all, &facts("withdrawn")));
        assert!(!matches(&all, &facts("never")));
    }

    /// Clauses AND; empty clauses are wildcards; a tier clause
    /// excludes account-less contacts.
    #[test]
    fn clauses_and_together() {
        let filter = Filter {
            statuses: vec!["active".to_string()],
            channels: vec![],
            account_tiers: vec!["customer".to_string()],
        };
        assert!(matches(&filter, &facts("granted")));
        let mut inactive = facts("granted");
        inactive.status = "inactive".to_string();
        assert!(!matches(&filter, &inactive));
        let mut no_account = facts("granted");
        no_account.account_tier = None;
        assert!(!matches(&filter, &no_account));
        // With no tier clause the account-less contact matches.
        let loose = Filter {
            statuses: vec!["active".to_string()],
            ..Filter::default()
        };
        assert!(matches(&loose, &no_account));
    }
}
