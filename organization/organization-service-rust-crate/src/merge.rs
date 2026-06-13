//! Pure record-merge logic for organizations.
//!
//! Merging folds a confirmed-duplicate organization into a surviving
//! "main" record: list fields are unioned, scalar identity fields keep
//! main's value (falling back to the duplicate's when main's is empty),
//! and the duplicate's former common name is preserved as an
//! `alternate_names` entry. The DB orchestration (update main,
//! soft-delete the duplicate, write the audit + merge-history rows,
//! publish the event) lives in the controller; this module is pure and
//! unit-testable.

use organization_matcher::Organization;
use serde_json::Value;

/// The result of merging a duplicate into a surviving (main) organization.
pub struct MergeOutcome {
    /// The survivor's new payload — main plus everything transferred.
    pub merged: Organization,
    /// Snapshot of the duplicate at merge time, kept in the merge-history
    /// record for auditability.
    pub transferred: Value,
}

/// Merge `duplicate` into `main`, returning the survivor's new payload.
///
/// - **Scalars** (`legal_name`, `url`, `address`, `jurisdiction`,
///   `founding_date`, `telephone`, `email`): keep main's value, else take
///   the duplicate's.
/// - **Lists** (`alternate_names`, `same_as`, `keywords`, `identifiers`):
///   unioned, dropping exact duplicates.
/// - The duplicate's primary `name` is added to the survivor's
///   `alternate_names` as a "former" name (when it differs from main's).
/// - `name` itself is never changed — the survivor keeps its name.
#[must_use]
pub fn merge_orgs(main: &Organization, duplicate: &Organization) -> MergeOutcome {
    let mut merged = main.clone();

    // The duplicate's former common name becomes an alternate name on main.
    let mut alt_seed = main.alternate_names.clone();
    if duplicate.name != main.name && !alt_seed.iter().any(|n| n == &duplicate.name) {
        alt_seed.push(duplicate.name.clone());
    }
    merged.alternate_names = union_strings(&alt_seed, &duplicate.alternate_names);

    // Scalars: keep main's, else adopt the duplicate's.
    merged.legal_name = main
        .legal_name
        .clone()
        .or_else(|| duplicate.legal_name.clone());
    merged.url = main.url.clone().or_else(|| duplicate.url.clone());
    merged.address = main.address.clone().or_else(|| duplicate.address.clone());
    merged.jurisdiction = main
        .jurisdiction
        .clone()
        .or_else(|| duplicate.jurisdiction.clone());
    merged.founding_date = main
        .founding_date
        .clone()
        .or_else(|| duplicate.founding_date.clone());
    merged.telephone = main
        .telephone
        .clone()
        .or_else(|| duplicate.telephone.clone());
    merged.email = main.email.clone().or_else(|| duplicate.email.clone());

    // List unions.
    merged.same_as = union_strings(&main.same_as, &duplicate.same_as);
    merged.keywords = union_strings(&main.keywords, &duplicate.keywords);
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
    use organization_matcher::{IdentifierScheme, OrgIdentifier};

    fn id(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
        OrgIdentifier {
            scheme,
            value: value.to_string(),
        }
    }

    #[test]
    fn duplicate_name_becomes_an_alternate_name() {
        let main = Organization {
            name: "Acme Inc".into(),
            ..Default::default()
        };
        let dup = Organization {
            name: "Acme Incorporated".into(),
            ..Default::default()
        };
        let out = merge_orgs(&main, &dup);
        assert_eq!(out.merged.name, "Acme Inc");
        assert!(out
            .merged
            .alternate_names
            .contains(&"Acme Incorporated".to_string()));
    }

    #[test]
    fn same_name_is_not_added_as_alternate() {
        let main = Organization {
            name: "Acme".into(),
            ..Default::default()
        };
        let dup = Organization {
            name: "Acme".into(),
            ..Default::default()
        };
        assert!(merge_orgs(&main, &dup).merged.alternate_names.is_empty());
    }

    #[test]
    fn scalars_fill_from_duplicate_only_when_main_is_empty() {
        let main = Organization {
            name: "P".into(),
            url: Some("https://main.example".into()),
            ..Default::default()
        };
        let dup = Organization {
            name: "Q".into(),
            url: Some("https://dup.example".into()),
            jurisdiction: Some("GB".into()),
            email: Some("info@dup.example".into()),
            ..Default::default()
        };
        let out = merge_orgs(&main, &dup);
        assert_eq!(out.merged.url.as_deref(), Some("https://main.example")); // kept
        assert_eq!(out.merged.jurisdiction.as_deref(), Some("GB")); // adopted
        assert_eq!(out.merged.email.as_deref(), Some("info@dup.example")); // adopted
    }

    #[test]
    fn list_fields_union_without_duplicates() {
        let main = Organization {
            name: "P".into(),
            keywords: vec!["fintech".into(), "bank".into()],
            identifiers: vec![id(IdentifierScheme::Lei, "X")],
            ..Default::default()
        };
        let dup = Organization {
            name: "Q".into(),
            keywords: vec!["bank".into(), "lending".into()], // "bank" overlaps
            identifiers: vec![
                id(IdentifierScheme::Lei, "X"),    // same → dropped
                id(IdentifierScheme::Duns, "999"), // new
            ],
            ..Default::default()
        };
        let out = merge_orgs(&main, &dup);
        assert_eq!(out.merged.keywords, vec!["fintech", "bank", "lending"]);
        assert_eq!(out.merged.identifiers.len(), 2);
    }

    #[test]
    fn transferred_snapshot_is_the_duplicate() {
        let main = Organization {
            name: "P".into(),
            ..Default::default()
        };
        let dup = Organization {
            name: "Q".into(),
            keywords: vec!["x".into()],
            ..Default::default()
        };
        let out = merge_orgs(&main, &dup);
        assert_eq!(out.transferred["name"], "Q");
        assert_eq!(out.transferred["keywords"][0], "x");
    }
}
