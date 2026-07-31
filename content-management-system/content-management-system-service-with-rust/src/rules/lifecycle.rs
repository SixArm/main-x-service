//! The editorial lifecycle (CMS-R9, CMS-D4) — a pure transition table.
//!
//! ```text
//! draft ──submit──▶ in_review ──approve──▶ approved ──publish──▶ published
//!   ▲                   │                     │                     │
//!   │                   └──reject(reason)─────┘                     │
//!   └───────────────────────────────────────── unpublish(reason)────┘
//!
//! any ──archive(reason)──▶ archived ──restore(reason)──▶ draft
//! ```
//!
//! Three things about this table are deliberate:
//!
//! - **`publish` is legal from `draft` as well as `approved`.** Skipping
//!   review is a *policy* question, not a second code path: the same
//!   transition is simply permitted for an editor or admin persona
//!   (spec `workflow.md`). One machine, many permission profiles — the
//!   alternative is two publish paths that drift apart, and only one of
//!   them gets the gates.
//! - **`publish` is legal from `published`.** Re-publishing moves the
//!   pointer to a newer revision; refusing it would force an editor to
//!   unpublish the live page first, which is a worse outcome than the
//!   thing it prevents.
//! - **Reasons are required where the action destroys or hides work**
//!   (reject, unpublish, archive, restore). An action that needs a
//!   reason and is given none is refused rather than defaulted, because
//!   the audit row is the point.
//!
//! Saving a revision is **not** a transition: a published variant stays
//! published while newer drafts accumulate behind it, and the live page
//! does not change until the next publish (CMS-D3).

use serde::{Deserialize, Serialize};

/// What an operator is asking to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Hand a draft to a reviewer.
    Submit,
    /// Accept a reviewed variant.
    Approve,
    /// Send it back, with a reason.
    Reject,
    /// Make a specific revision live.
    Publish,
    /// Take the live revision down, with a reason.
    Unpublish,
    /// Put it away, with a reason.
    Archive,
    /// Bring it back from archived, with a reason.
    Restore,
}

impl Action {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Submit => "submit",
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Publish => "publish",
            Self::Unpublish => "unpublish",
            Self::Archive => "archive",
            Self::Restore => "restore",
        }
    }

    /// Parse a wire token.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        [
            Self::Submit,
            Self::Approve,
            Self::Reject,
            Self::Publish,
            Self::Unpublish,
            Self::Archive,
            Self::Restore,
        ]
        .into_iter()
        .find(|action| action.as_str() == token)
    }

    /// Whether this action must carry a reason.
    #[must_use]
    pub const fn requires_reason(self) -> bool {
        matches!(
            self,
            Self::Reject | Self::Unpublish | Self::Archive | Self::Restore
        )
    }
}

/// Every action, for error messages that list the alternatives.
pub const ACTIONS: &[&str] = &[
    "submit",
    "approve",
    "reject",
    "publish",
    "unpublish",
    "archive",
    "restore",
];

/// The transition table itself: the state an action leads to, or
/// `None` when the pair is not a legal transition.
///
/// Separate from [`next`] because [`next`]'s error message lists the
/// legal actions, and computing that list means asking this question
/// for every action. Routing that through [`next`] made the two
/// functions call each other until the stack ran out — caught by the
/// "an illegal transition explains itself" test below, which is why it
/// exists. The table is the primitive; the message is built on top.
// Several arms legitimately land on the same state — reject,
// unpublish, and restore all return work to `draft`. Merging them
// would make the table stop reading like the diagram above, which is
// the one property that makes this function reviewable.
#[allow(clippy::match_same_arms)]
#[must_use]
pub const fn try_next(from: &str, action: Action) -> Option<&'static str> {
    match (from.as_bytes(), action) {
        (b"draft", Action::Submit) => Some("in_review"),
        (b"in_review", Action::Approve) => Some("approved"),
        (b"in_review", Action::Reject) => Some("draft"),
        // Direct-publish from draft is the same transition, gated by
        // policy rather than by a second code path.
        (b"draft" | b"approved" | b"published", Action::Publish) => Some("published"),
        (b"published", Action::Unpublish) => Some("draft"),
        (b"draft" | b"in_review" | b"approved" | b"published", Action::Archive) => Some("archived"),
        (b"archived", Action::Restore) => Some("draft"),
        _ => None,
    }
}

/// The state an action leads to, given the state it starts from.
///
/// # Errors
///
/// A message naming the **current** state and the legal actions from
/// it — the family convention, and the difference between a `422` an
/// operator can act on and one they have to guess at.
pub fn next(from: &str, action: Action) -> Result<&'static str, String> {
    try_next(from, action).ok_or_else(|| {
        format!(
            "cannot {} a variant that is {from:?}; legal actions from {from:?} are {:?}",
            action.as_str(),
            legal_actions(from)
        )
    })
}

/// The actions legal from `state`.
#[must_use]
pub fn legal_actions(state: &str) -> Vec<&'static str> {
    ACTIONS
        .iter()
        .copied()
        .filter(|token| {
            Action::parse(token).is_some_and(|action| try_next(state, action).is_some())
        })
        .collect()
}

/// Whether this transition changes what the public sees: only publish
/// and unpublish do. Used to decide what has to be re-derived and what
/// has to be told to the outside world.
#[must_use]
pub const fn affects_delivery(action: Action) -> bool {
    matches!(action, Action::Publish | Action::Unpublish)
}

/// The translation workflow, which runs **alongside** the editorial
/// lifecycle rather than inside it (CMS-R15).
///
/// ```text
/// (none) ──request──▶ requested ──claim──▶ in_translation ──complete──▶ translated
///    ▲                    │                      │                          │
///    └────────────────────┴──── cancel ──────────┘                          │
///    └──────────────────────── request (again, when stale) ─────────────────┘
/// ```
///
/// Translation status is orthogonal to editorial status: a translated
/// variant then goes through the ordinary review and publish path like
/// anything else. Keeping them separate is what stops "translated"
/// from quietly meaning "approved".
pub mod translation {
    /// A translation-workflow action.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Action {
        /// Ask for a translation of a specific source revision.
        Request,
        /// A translator picks it up.
        Claim,
        /// The translation is written.
        Complete,
        /// Abandon the request.
        Cancel,
    }

    impl Action {
        /// The wire token.
        #[must_use]
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::Request => "request",
                Self::Claim => "claim",
                Self::Complete => "complete",
                Self::Cancel => "cancel",
            }
        }

        /// Parse a wire token.
        #[must_use]
        pub fn parse(token: &str) -> Option<Self> {
            [Self::Request, Self::Claim, Self::Complete, Self::Cancel]
                .into_iter()
                .find(|action| action.as_str() == token)
        }
    }

    /// The status an action leads to, from the current one (`None` when
    /// no translation has ever been asked for).
    ///
    /// # Errors
    ///
    /// A message naming the current status.
    pub fn next(from: Option<&str>, action: Action) -> Result<Option<&'static str>, String> {
        match (from, action) {
            // Re-requesting a translated variant is how a stale one is
            // refreshed — the common case, not an edge case.
            (None | Some("translated"), Action::Request) => Ok(Some("requested")),
            (Some("requested"), Action::Claim) => Ok(Some("in_translation")),
            (Some("in_translation"), Action::Complete) => Ok(Some("translated")),
            (Some("requested" | "in_translation"), Action::Cancel) => Ok(None),
            _ => Err(format!(
                "cannot {} a translation that is {}",
                action.as_str(),
                from.map_or_else(|| "not requested".to_string(), |s| format!("{s:?}"))
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_happy_path_walks_to_published() {
        assert_eq!(next("draft", Action::Submit), Ok("in_review"));
        assert_eq!(next("in_review", Action::Approve), Ok("approved"));
        assert_eq!(next("approved", Action::Publish), Ok("published"));
    }

    #[test]
    fn rejection_returns_to_draft() {
        assert_eq!(next("in_review", Action::Reject), Ok("draft"));
    }

    /// Skipping review is a permission question, not a different code
    /// path — so the machine allows it and the policy decides.
    #[test]
    fn direct_publish_from_draft_is_the_same_transition() {
        assert_eq!(next("draft", Action::Publish), Ok("published"));
    }

    /// Re-publishing moves the pointer to a newer revision. Refusing it
    /// would force an editor to take the live page down first.
    #[test]
    fn republishing_is_legal() {
        assert_eq!(next("published", Action::Publish), Ok("published"));
    }

    #[test]
    fn unpublish_returns_to_draft_and_archive_is_reachable_from_everywhere() {
        assert_eq!(next("published", Action::Unpublish), Ok("draft"));
        for state in ["draft", "in_review", "approved", "published"] {
            assert_eq!(next(state, Action::Archive), Ok("archived"), "{state}");
        }
        assert_eq!(next("archived", Action::Restore), Ok("draft"));
    }

    /// Archived is terminal except a reasoned restore.
    #[test]
    fn archived_accepts_only_restore() {
        assert_eq!(legal_actions("archived"), vec!["restore"]);
        for action in [
            Action::Submit,
            Action::Approve,
            Action::Publish,
            Action::Unpublish,
        ] {
            assert!(next("archived", action).is_err(), "{action:?}");
        }
    }

    /// The refusal names the current state and what *is* possible.
    #[test]
    fn an_illegal_transition_explains_itself() {
        let error = next("draft", Action::Approve).unwrap_err();
        assert!(error.contains("\"draft\""));
        assert!(error.contains("legal actions"));
        assert!(error.contains("submit"));
        assert!(error.contains("publish"));
    }

    #[test]
    fn destructive_actions_require_a_reason() {
        for action in [
            Action::Reject,
            Action::Unpublish,
            Action::Archive,
            Action::Restore,
        ] {
            assert!(action.requires_reason(), "{action:?} should need a reason");
        }
        for action in [Action::Submit, Action::Approve, Action::Publish] {
            assert!(!action.requires_reason(), "{action:?} should not");
        }
    }

    #[test]
    fn only_publish_and_unpublish_change_what_the_public_sees() {
        assert!(affects_delivery(Action::Publish));
        assert!(affects_delivery(Action::Unpublish));
        for action in [
            Action::Submit,
            Action::Approve,
            Action::Reject,
            Action::Archive,
            Action::Restore,
        ] {
            assert!(!affects_delivery(action), "{action:?}");
        }
    }

    #[test]
    fn wire_tokens_round_trip_and_unknown_ones_are_none() {
        for token in ACTIONS {
            assert_eq!(Action::parse(token).map(Action::as_str), Some(*token));
        }
        assert!(Action::parse("delete").is_none());
        assert!(Action::parse("Publish").is_none());
        assert!(Action::parse("").is_none());
    }

    /// An unknown state has no legal actions rather than panicking —
    /// a stored status this build does not know about must not become a
    /// crash.
    #[test]
    fn an_unknown_state_is_inert() {
        assert!(legal_actions("banana").is_empty());
        assert!(next("banana", Action::Publish).is_err());
    }

    /// Every legal transition lands on a status the vocabulary knows.
    #[test]
    fn every_target_state_is_a_declared_status() {
        for state in crate::rules::tokens::VARIANT_STATUSES {
            for token in ACTIONS {
                if let Some(action) = Action::parse(token)
                    && let Ok(to) = next(state, action)
                {
                    assert!(
                        crate::rules::tokens::VARIANT_STATUSES.contains(&to),
                        "{state} + {token} -> unknown status {to}"
                    );
                }
            }
        }
    }

    // ---- translation ----------------------------------------------

    use translation::Action as T;

    #[test]
    fn a_translation_walks_request_claim_complete() {
        assert_eq!(translation::next(None, T::Request), Ok(Some("requested")));
        assert_eq!(
            translation::next(Some("requested"), T::Claim),
            Ok(Some("in_translation"))
        );
        assert_eq!(
            translation::next(Some("in_translation"), T::Complete),
            Ok(Some("translated"))
        );
    }

    /// Refreshing a stale translation is a re-request — the common
    /// case, not an edge case.
    #[test]
    fn a_translated_variant_can_be_requested_again() {
        assert_eq!(
            translation::next(Some("translated"), T::Request),
            Ok(Some("requested"))
        );
    }

    #[test]
    fn a_request_can_be_cancelled_back_to_nothing() {
        assert_eq!(translation::next(Some("requested"), T::Cancel), Ok(None));
        assert_eq!(
            translation::next(Some("in_translation"), T::Cancel),
            Ok(None)
        );
        assert!(translation::next(Some("translated"), T::Cancel).is_err());
    }

    #[test]
    fn out_of_order_translation_actions_name_the_current_status() {
        let error = translation::next(None, T::Complete).unwrap_err();
        assert!(error.contains("not requested"));
        let error = translation::next(Some("requested"), T::Complete).unwrap_err();
        assert!(error.contains("\"requested\""));
    }

    #[test]
    fn translation_tokens_round_trip() {
        for token in ["request", "claim", "complete", "cancel"] {
            assert_eq!(T::parse(token).map(T::as_str), Some(token));
        }
        assert!(T::parse("translate").is_none());
    }
}
