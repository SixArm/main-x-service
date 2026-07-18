//! Lifecycle state machines (CRM-D3): one transition table per
//! pipeline, all funnelled through [`check`] so every controller gives
//! the same `422`-shaped answer. Deals are the exception — their
//! stages are data (pipeline rows), validated in the controller
//! against the pipeline's stage list and terminal flags.

/// The legal lead transitions (CRM-R3).
pub const LEAD: &[(&str, &str)] = &[
    ("new", "contacted"),
    ("contacted", "qualified"),
    ("qualified", "converted"),
    ("new", "disqualified"),
    ("contacted", "disqualified"),
    ("qualified", "disqualified"),
];

/// The legal campaign transitions (CRM-R8).
pub const CAMPAIGN: &[(&str, &str)] = &[
    ("draft", "scheduled"),
    ("scheduled", "running"),
    ("running", "completed"),
    ("draft", "cancelled"),
    ("scheduled", "cancelled"),
    ("running", "cancelled"),
];

/// The legal ticket transitions (CRM-R10): `pending` =
/// waiting-on-customer; reopen from `resolved`; `closed` terminal.
pub const TICKET: &[(&str, &str)] = &[
    ("open", "pending"),
    ("pending", "open"),
    ("open", "resolved"),
    ("pending", "resolved"),
    ("resolved", "closed"),
    ("resolved", "open"),
];

/// The legal article transitions (CRM-R12). Published edits bump the
/// version in place (no status change).
pub const ARTICLE: &[(&str, &str)] = &[
    ("draft", "published"),
    ("published", "archived"),
];

/// Whether `table` permits moving `from → to`.
#[must_use]
pub fn permits(table: &[(&str, &str)], from: &str, to: &str) -> bool {
    table.iter().any(|(f, t)| *f == from && *t == to)
}

/// Check a transition, yielding the family `422` message on refusal.
///
/// # Errors
///
/// A human-readable refusal when the transition is not in the table.
pub fn check(kind: &str, table: &[(&str, &str)], from: &str, to: &str) -> Result<(), String> {
    if permits(table, from, to) {
        Ok(())
    } else {
        Err(format!("cannot move {kind} from {from:?} to {to:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tokens;

    /// Every `(from, to)` pair uses vocabulary tokens only.
    #[test]
    fn tables_use_vocabulary_tokens_only() {
        type Case = (&'static [(&'static str, &'static str)], &'static [&'static str]);
        let cases: &[Case] = &[
            (LEAD, tokens::LEAD_STATUSES),
            (CAMPAIGN, tokens::CAMPAIGN_STATUSES),
            (TICKET, tokens::TICKET_STATUSES),
            (ARTICLE, tokens::ARTICLE_STATUSES),
        ];
        for (table, vocab) in cases {
            for (from, to) in *table {
                assert!(tokens::is_token(vocab, from), "unknown token {from}");
                assert!(tokens::is_token(vocab, to), "unknown token {to}");
            }
        }
    }

    /// Leads: forward path only; disqualify from any live status;
    /// terminals are terminal; no skipping to converted.
    #[test]
    fn lead_matrix() {
        assert!(permits(LEAD, "new", "contacted"));
        assert!(permits(LEAD, "qualified", "converted"));
        for from in ["new", "contacted", "qualified"] {
            assert!(permits(LEAD, from, "disqualified"));
        }
        assert!(!permits(LEAD, "new", "converted"));
        assert!(!permits(LEAD, "contacted", "converted"));
        assert!(!permits(LEAD, "converted", "disqualified"));
        assert!(!permits(LEAD, "disqualified", "new"));
    }

    /// Campaigns: cancel from any live status; completed/cancelled
    /// terminal; no draft → running skip.
    #[test]
    fn campaign_matrix() {
        assert!(permits(CAMPAIGN, "draft", "scheduled"));
        assert!(permits(CAMPAIGN, "running", "completed"));
        assert!(permits(CAMPAIGN, "running", "cancelled"));
        assert!(!permits(CAMPAIGN, "draft", "running"));
        assert!(!permits(CAMPAIGN, "completed", "running"));
        assert!(!permits(CAMPAIGN, "cancelled", "scheduled"));
    }

    /// Tickets: pending bounces back to open; resolve from open or
    /// pending; reopen from resolved; closed terminal.
    #[test]
    fn ticket_matrix() {
        assert!(permits(TICKET, "open", "pending"));
        assert!(permits(TICKET, "pending", "open"));
        assert!(permits(TICKET, "pending", "resolved"));
        assert!(permits(TICKET, "resolved", "open"));
        assert!(permits(TICKET, "resolved", "closed"));
        assert!(!permits(TICKET, "open", "closed"));
        assert!(!permits(TICKET, "closed", "open"));
    }

    /// Articles: draft → published → archived, nothing else.
    #[test]
    fn article_matrix() {
        assert!(permits(ARTICLE, "draft", "published"));
        assert!(permits(ARTICLE, "published", "archived"));
        assert!(!permits(ARTICLE, "draft", "archived"));
        assert!(!permits(ARTICLE, "archived", "published"));
    }
}
