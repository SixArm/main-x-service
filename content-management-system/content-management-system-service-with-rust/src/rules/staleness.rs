//! Translation staleness (CMS-R15, CMS-D13) — pure, DB-free.
//!
//! A translated variant records **the exact source revision it was
//! translated from** (`translation_of_revision_pid`). When the source
//! publishes newer revisions, the translation is out of date — and the
//! whole point of storing that pointer is that the system can say so
//! instead of leaving readers to notice.
//!
//! ```text
//! stale  ⇔  source.published_revision.number > translated_from.number
//! ```
//!
//! Three properties are deliberate:
//!
//! - **Derived, never stored.** There is no `is_stale` column to fall
//!   out of date with the thing it describes (CMS-D13).
//! - **Says how far behind, and which revisions.** "Stale" alone tells
//!   a translator nothing; the list is what lets them read the diff
//!   rather than re-translate from scratch.
//! - **Not an automatic unpublish.** Stale-but-published usually beats
//!   absent, and that judgement belongs to an editor — so it is an
//!   opt-in per content type (`unpublish_on_stale`), off by default.

use serde::{Deserialize, Serialize};

/// How far behind its source a translation is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Staleness {
    /// Whether the source has moved past what was translated.
    pub stale: bool,
    /// How many published source revisions landed since.
    pub revisions_behind: usize,
    /// The source revision this translation was made from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_from_number: Option<i32>,
    /// The source revision currently published.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_published_number: Option<i32>,
    /// The revision numbers that landed since, so a translator can read
    /// the diff rather than start again.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub newer_revision_numbers: Vec<i32>,
    /// Why staleness could not be determined, when it could not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown: Option<&'static str>,
}

impl Staleness {
    /// A verdict of "cannot tell", with the reason.
    fn unknown(reason: &'static str) -> Self {
        Self {
            stale: false,
            revisions_behind: 0,
            translated_from_number: None,
            source_published_number: None,
            newer_revision_numbers: Vec::new(),
            unknown: Some(reason),
        }
    }
}

/// Compute staleness from revision numbers.
///
/// - `translated_from` — the number of the source revision this
///   translation was made from, if it records one.
/// - `source_published` — the number of the source's currently
///   published revision, if it has one.
/// - `source_numbers` — every revision number on the source variant,
///   used to list what landed in between.
///
/// A translation with no recorded source revision is **unknown**, not
/// "fresh": claiming freshness for content whose provenance was never
/// recorded is the more dangerous of the two guesses.
#[must_use]
pub fn staleness(
    translated_from: Option<i32>,
    source_published: Option<i32>,
    source_numbers: &[i32],
) -> Staleness {
    let Some(from) = translated_from else {
        return Staleness::unknown("this variant does not record a source revision");
    };
    let Some(published) = source_published else {
        return Staleness::unknown("the source variant has nothing published");
    };
    let mut newer: Vec<i32> = source_numbers
        .iter()
        .copied()
        .filter(|number| *number > from && *number <= published)
        .collect();
    newer.sort_unstable();
    Staleness {
        stale: published > from,
        revisions_behind: newer.len(),
        translated_from_number: Some(from),
        source_published_number: Some(published),
        newer_revision_numbers: newer,
        unknown: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_translation_of_the_live_revision_is_fresh() {
        let result = staleness(Some(3), Some(3), &[1, 2, 3]);
        assert!(!result.stale);
        assert_eq!(result.revisions_behind, 0);
        assert!(result.newer_revision_numbers.is_empty());
        assert!(result.unknown.is_none());
    }

    /// The number that matters to a translator: how far behind, and
    /// which revisions to read.
    #[test]
    fn staleness_reports_how_far_behind_and_which_revisions() {
        let result = staleness(Some(2), Some(5), &[1, 2, 3, 4, 5]);
        assert!(result.stale);
        assert_eq!(result.revisions_behind, 3);
        assert_eq!(result.newer_revision_numbers, vec![3, 4, 5]);
        assert_eq!(result.translated_from_number, Some(2));
        assert_eq!(result.source_published_number, Some(5));
    }

    /// Unpublished source revisions do not count: a translation is
    /// behind the *published* source, not behind someone's draft.
    #[test]
    fn drafts_beyond_the_published_source_are_not_counted() {
        let result = staleness(Some(2), Some(4), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(result.revisions_behind, 2);
        assert_eq!(result.newer_revision_numbers, vec![3, 4]);
    }

    /// Translated from something newer than what is published (the
    /// source was rolled back): behind by nothing, and not stale.
    #[test]
    fn a_translation_ahead_of_the_published_source_is_not_stale() {
        let result = staleness(Some(5), Some(3), &[1, 2, 3, 4, 5]);
        assert!(!result.stale);
        assert_eq!(result.revisions_behind, 0);
    }

    /// "Cannot tell" is reported as such. Claiming freshness for
    /// content whose provenance was never recorded is the more
    /// dangerous guess.
    #[test]
    fn missing_provenance_is_unknown_not_fresh() {
        let no_source = staleness(None, Some(3), &[1, 2, 3]);
        assert!(!no_source.stale);
        assert_eq!(
            no_source.unknown,
            Some("this variant does not record a source revision")
        );

        let nothing_published = staleness(Some(1), None, &[1, 2]);
        assert_eq!(
            nothing_published.unknown,
            Some("the source variant has nothing published")
        );
    }

    /// A gappy or unsorted revision list is handled without panicking,
    /// and the answer comes back ordered.
    #[test]
    fn odd_revision_lists_are_handled() {
        let result = staleness(Some(1), Some(9), &[9, 1, 4, 7]);
        assert!(result.stale);
        assert_eq!(result.newer_revision_numbers, vec![4, 7, 9]);
        assert_eq!(result.revisions_behind, 3);
        assert!(
            staleness(Some(1), Some(2), &[]).stale,
            "an empty list still compares numbers"
        );
    }
}
