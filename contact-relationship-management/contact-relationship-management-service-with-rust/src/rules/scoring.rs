//! Deterministic lead scoring (CRM-R3, CRM-D5): a fixed rule table,
//! recomputed on change, with a per-rule breakdown in the output —
//! explainable, never ML. Weights are config-tunable via
//! [`Weights`]; the rule *set* is fixed in v1.

/// Freemail domains that do NOT earn the corporate-email points.
pub const FREEMAIL: &[&str] = &[
    "gmail.com",
    "yahoo.com",
    "outlook.com",
    "hotmail.com",
    "icloud.com",
    "aol.com",
    "proton.me",
    "protonmail.com",
];

/// Tunable rule weights (defaults per spec `sales-automation.md`).
#[derive(Debug, Clone, Copy)]
pub struct Weights {
    /// `source = referral`.
    pub referral: i32,
    /// `source = campaign` with an attributed campaign.
    pub campaign: i32,
    /// A known contact (`contact_ref` set).
    pub known_contact: i32,
    /// Corporate (non-freemail) email domain.
    pub corporate_domain: i32,
    /// Recent activity (full points at ≤7 days, decaying to 0 at 30).
    pub recent_activity: i32,
    /// Three or more activities in total.
    pub activity_volume: i32,
    /// A campaign click on record.
    pub campaign_click: i32,
    /// An unsubscribe on record (negative).
    pub unsubscribe: i32,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            referral: 20,
            campaign: 10,
            known_contact: 15,
            corporate_domain: 10,
            recent_activity: 15,
            activity_volume: 10,
            campaign_click: 10,
            unsubscribe: -30,
        }
    }
}

/// The scoring inputs — plain facts, derived by the caller.
#[allow(clippy::struct_excessive_bools)] // independent boolean facts
#[derive(Debug, Clone, Default)]
pub struct LeadFacts {
    /// The lead's `source` token.
    pub source: String,
    /// Whether a campaign is attributed (`campaign_pid` set).
    pub campaign_attributed: bool,
    /// Whether the lead is linked to a known contact.
    pub known_contact: bool,
    /// The lead's email domain (lowercased), when known.
    pub email_domain: Option<String>,
    /// Days since the most recent activity, when any exists.
    pub days_since_last_activity: Option<i64>,
    /// Total activities on the lead.
    pub activity_count: i64,
    /// A campaign click on record.
    pub campaign_click: bool,
    /// An unsubscribe on record.
    pub unsubscribed: bool,
}

/// One rule's contribution in the breakdown.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RuleScore {
    /// Stable rule id.
    pub rule: String,
    /// Points contributed (0 when the rule did not fire).
    pub points: i32,
}

/// The score plus its explanation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoreBreakdown {
    /// The clamped 0–100 score.
    pub score: i32,
    /// The label: `hot` (≥70), `warm` (≥40), `cold`.
    pub label: &'static str,
    /// Per-rule contributions (every rule listed, fired or not).
    pub rules: Vec<RuleScore>,
}

/// Recent-activity points: full at ≤7 days, linear decay to 0 at 30.
#[must_use]
pub fn recency_points(days: i64, full: i32) -> i32 {
    if days <= 7 {
        full
    } else if days >= 30 {
        0
    } else {
        // Linear over the 23-day window, ceiling — stays ≥1 until the
        // window closes at day 30.
        let remaining = 30 - days; // 1..=22
        i32::try_from((i64::from(full) * remaining + 22) / 23).unwrap_or(0)
    }
}

/// Score a lead from its facts (CRM-D5): deterministic, clamped, with
/// the full per-rule breakdown.
#[must_use]
pub fn score(facts: &LeadFacts, weights: &Weights) -> ScoreBreakdown {
    fn fired(rule: &str, fired: bool, points: i32) -> RuleScore {
        RuleScore {
            rule: rule.to_string(),
            points: if fired { points } else { 0 },
        }
    }
    let corporate = facts
        .email_domain
        .as_deref()
        .is_some_and(|d| !d.is_empty() && !FREEMAIL.contains(&d));
    let recency = facts
        .days_since_last_activity
        .map_or(0, |days| recency_points(days, weights.recent_activity));
    let rules = vec![
        fired("source_referral", facts.source == "referral", weights.referral),
        fired(
            "source_campaign",
            facts.source == "campaign" && facts.campaign_attributed,
            weights.campaign,
        ),
        fired("known_contact", facts.known_contact, weights.known_contact),
        fired("corporate_domain", corporate, weights.corporate_domain),
        RuleScore {
            rule: "recent_activity".to_string(),
            points: recency,
        },
        fired("activity_volume", facts.activity_count >= 3, weights.activity_volume),
        fired("campaign_click", facts.campaign_click, weights.campaign_click),
        fired("unsubscribe", facts.unsubscribed, weights.unsubscribe),
    ];
    let total: i32 = rules.iter().map(|r| r.points).sum();
    let score = total.clamp(0, 100);
    let label = if score >= 70 {
        "hot"
    } else if score >= 40 {
        "warm"
    } else {
        "cold"
    };
    ScoreBreakdown { score, label, rules }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> LeadFacts {
        LeadFacts {
            source: "web".to_string(),
            ..LeadFacts::default()
        }
    }

    /// The breakdown lists every rule, sums to the clamped score, and
    /// labels hot/warm/cold at the documented thresholds.
    #[test]
    fn breakdown_sums_and_labels() {
        let cold = score(&facts(), &Weights::default());
        assert_eq!(cold.score, 0);
        assert_eq!(cold.label, "cold");
        assert_eq!(cold.rules.len(), 8);

        let hot = score(
            &LeadFacts {
                source: "referral".to_string(),
                known_contact: true,
                email_domain: Some("bigco.example".to_string()),
                days_since_last_activity: Some(2),
                activity_count: 4,
                campaign_click: true,
                ..LeadFacts::default()
            },
            &Weights::default(),
        );
        // 20 + 15 + 10 + 15 + 10 + 10 = 80.
        assert_eq!(hot.score, 80);
        assert_eq!(hot.label, "hot");
        let sum: i32 = hot.rules.iter().map(|r| r.points).sum();
        assert_eq!(sum, 80);

        let warm = score(
            &LeadFacts {
                source: "referral".to_string(),
                known_contact: true,
                days_since_last_activity: Some(40),
                ..facts()
            },
            &Weights::default(),
        );
        assert_eq!(warm.score, 35 /* 20 + 15 */);
        assert_eq!(warm.label, "cold");
    }

    /// Freemail earns nothing; corporate does; unknown domain earns
    /// nothing.
    #[test]
    fn corporate_domain_rule() {
        let base = facts();
        for freemail in ["gmail.com", "hotmail.com"] {
            let s = score(
                &LeadFacts { email_domain: Some(freemail.to_string()), ..base.clone() },
                &Weights::default(),
            );
            assert_eq!(s.score, 0, "{freemail}");
        }
        let s = score(
            &LeadFacts { email_domain: Some("initech.example".to_string()), ..base },
            &Weights::default(),
        );
        assert_eq!(s.score, 10);
    }

    /// Recency decays linearly: full ≤7d, 0 ≥30d, monotone between.
    #[test]
    fn recency_decay() {
        assert_eq!(recency_points(0, 15), 15);
        assert_eq!(recency_points(7, 15), 15);
        assert_eq!(recency_points(30, 15), 0);
        assert_eq!(recency_points(365, 15), 0);
        let mut previous = 15;
        for days in 8..30 {
            let points = recency_points(days, 15);
            assert!(points <= previous, "monotone at {days}");
            assert!(points > 0, "positive inside the window at {days}");
            previous = points;
        }
    }

    /// The unsubscribe penalty floors at zero (clamp), never negative.
    #[test]
    fn unsubscribe_clamps_at_zero() {
        let s = score(
            &LeadFacts { unsubscribed: true, ..facts() },
            &Weights::default(),
        );
        assert_eq!(s.score, 0);
        // But it drags a warm lead down.
        let dragged = score(
            &LeadFacts {
                source: "referral".to_string(),
                known_contact: true,
                campaign_click: true,
                unsubscribed: true,
                ..facts()
            },
            &Weights::default(),
        );
        assert_eq!(dragged.score, 15 /* 20+15+10-30 */);
    }
}
