//! Pure record-merge logic for cases.
//!
//! Merging folds a confirmed-duplicate case into a surviving "main"
//! record: list fields are unioned, scalar identity fields keep main's
//! value (falling back to the duplicate's when main's is empty), and the
//! duplicate's former title is preserved as an `alternate_titles` entry.
//! The DB orchestration (update main, soft-delete the duplicate, write
//! the audit + merge-history rows, publish the event) lives in the
//! controller; this module is pure and unit-testable.

use case_matcher::Case;
use serde_json::Value;

/// The result of merging a duplicate into a surviving (main) case.
pub struct MergeOutcome {
    /// The survivor's new payload — main plus everything transferred.
    pub merged: Case,
    /// Snapshot of the duplicate at merge time, kept in the merge-history
    /// record for auditability.
    pub transferred: Value,
}

/// Merge `duplicate` into `main`, returning the survivor's new payload.
///
/// - **Scalars** (`case_number`, `agency_id`, `agency_name`,
///   `case_type`, `status`, `priority`, `opened_date`): keep main's
///   value, else take the duplicate's.
/// - **Lists** (`alternate_titles`, `subjects`, `keywords`, `same_as`,
///   `in_language`, `identifiers`): unioned, dropping exact duplicates.
/// - The duplicate's primary `title` is added to the survivor's
///   `alternate_titles` as a "former" title (when it differs from main's).
/// - `title` itself is never changed — the survivor keeps its title.
#[must_use]
pub fn merge_cases(main: &Case, duplicate: &Case) -> MergeOutcome {
    let mut merged = main.clone();

    // The duplicate's former title becomes an alternate title on main.
    let mut alt_seed = main.alternate_titles.clone();
    if duplicate.title != main.title && !alt_seed.iter().any(|n| n == &duplicate.title) {
        alt_seed.push(duplicate.title.clone());
    }
    merged.alternate_titles = union_strings(&alt_seed, &duplicate.alternate_titles);

    // Scalars: keep main's, else adopt the duplicate's.
    merged.case_number = main
        .case_number
        .clone()
        .or_else(|| duplicate.case_number.clone());
    merged.agency_id = main
        .agency_id
        .clone()
        .or_else(|| duplicate.agency_id.clone());
    merged.agency_name = main
        .agency_name
        .clone()
        .or_else(|| duplicate.agency_name.clone());
    merged.case_type = main
        .case_type
        .clone()
        .or_else(|| duplicate.case_type.clone());
    merged.status = main.status.clone().or_else(|| duplicate.status.clone());
    merged.priority = main.priority.clone().or_else(|| duplicate.priority.clone());
    merged.opened_date = main
        .opened_date
        .clone()
        .or_else(|| duplicate.opened_date.clone());

    // List unions.
    merged.subjects = union_strings(&main.subjects, &duplicate.subjects);
    merged.keywords = union_strings(&main.keywords, &duplicate.keywords);
    merged.same_as = union_strings(&main.same_as, &duplicate.same_as);
    merged.in_language = union_strings(&main.in_language, &duplicate.in_language);
    merged.identifiers = union_by(&main.identifiers, &duplicate.identifiers, |a, b| {
        a.scheme == b.scheme && a.value == b.value
    });

    let transferred = serde_json::to_value(duplicate).unwrap_or(Value::Null);
    MergeOutcome {
        merged,
        transferred,
    }
}

fn union_strings(main: &[String], extra: &[String]) -> Vec<String> {
    union_by(main, extra, |a, b| a == b)
}

/// Clone `main`, then append every `extra` not already present per `eq`.
fn union_by<T: Clone>(main: &[T], extra: &[T], eq: impl Fn(&T, &T) -> bool) -> Vec<T> {
    let mut out = main.to_vec();
    for e in extra {
        if !out.iter().any(|x| eq(x, e)) {
            out.push(e.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use case_matcher::{CaseIdentifier, CaseType, IdentifierScheme};

    fn ident(scheme: IdentifierScheme, value: &str) -> CaseIdentifier {
        CaseIdentifier {
            scheme,
            value: value.to_string(),
        }
    }

    #[test]
    fn duplicate_title_becomes_an_alternate_title() {
        let main = Case::new("Housing benefit appeal");
        let dup = Case::new("HB appeal — J. Smith");
        let out = merge_cases(&main, &dup);
        assert_eq!(out.merged.title, "Housing benefit appeal");
        assert!(out
            .merged
            .alternate_titles
            .contains(&"HB appeal — J. Smith".to_string()));
    }

    #[test]
    fn same_title_is_not_added_as_alternate() {
        let main = Case::new("Housing benefit appeal");
        let dup = Case::new("Housing benefit appeal");
        let out = merge_cases(&main, &dup);
        assert!(out.merged.alternate_titles.is_empty());
    }

    #[test]
    fn scalars_fill_from_duplicate_only_when_main_is_empty() {
        let mut main = Case::new("P");
        main.case_number = Some("MAIN-01".into());
        let mut dup = Case::new("Q");
        dup.case_number = Some("DUP-01".into());
        dup.agency_id = Some("dwp".into());
        dup.case_type = Some(CaseType::Housing);
        let out = merge_cases(&main, &dup);
        assert_eq!(out.merged.case_number.as_deref(), Some("MAIN-01")); // kept
        assert_eq!(out.merged.agency_id.as_deref(), Some("dwp")); // adopted
        assert_eq!(out.merged.case_type, Some(CaseType::Housing)); // adopted
    }

    #[test]
    fn list_fields_union_without_duplicates() {
        let mut main = Case::new("P");
        main.keywords = vec!["housing".into(), "appeal".into()];
        main.identifiers = vec![ident(IdentifierScheme::Docket, "CV-2024-001234")];
        let mut dup = Case::new("Q");
        dup.keywords = vec!["appeal".into(), "benefit".into()]; // "appeal" overlaps
        dup.identifiers = vec![
            ident(IdentifierScheme::Docket, "CV-2024-001234"), // same → dropped
            ident(IdentifierScheme::ExternalCaseId, "EXT-99"), // new
        ];
        let out = merge_cases(&main, &dup);
        assert_eq!(out.merged.keywords, vec!["housing", "appeal", "benefit"]);
        assert_eq!(out.merged.identifiers.len(), 2);
    }

    #[test]
    fn transferred_snapshot_is_the_duplicate() {
        let main = Case::new("P");
        let mut dup = Case::new("Q");
        dup.keywords = vec!["x".into()];
        let out = merge_cases(&main, &dup);
        assert_eq!(out.transferred["title"], "Q");
        assert_eq!(out.transferred["keywords"][0], "x");
    }
}
