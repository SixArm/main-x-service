//! Pure record-merge logic for care pathways.
//!
//! Merging folds a confirmed-duplicate pathway into a surviving "main"
//! record: list fields are unioned, scalar identity fields keep main's
//! value (falling back to the duplicate's when main's is empty), and the
//! duplicate's former title is preserved as an `alternate_names` entry.
//! The DB orchestration (update main, soft-delete the duplicate, write
//! the audit + merge-history rows, publish the event) lives in the
//! controller; this module is pure and unit-testable.

use care_pathway_matcher::CarePathway;
use serde_json::Value;

/// The result of merging a duplicate into a surviving (main) pathway.
pub struct MergeOutcome {
    /// The survivor's new payload — main plus everything transferred.
    pub merged: CarePathway,
    /// Snapshot of the duplicate at merge time, kept in the merge-history
    /// record for auditability.
    pub transferred: Value,
}

/// Merge `duplicate` into `main`, returning the survivor's new payload.
///
/// - **Scalars** (`pathway_code`, `provider_id`, `provider_name`,
///   `care_setting`): keep main's value, else take the duplicate's.
/// - **Lists** (`alternate_names`, `interventions`, `keywords`,
///   `same_as`, `in_language`, `condition_codes`, `identifiers`):
///   unioned, dropping exact duplicates.
/// - The duplicate's primary `name` is added to the survivor's
///   `alternate_names` as a "former" title (when it differs from main's).
/// - `name` itself is never changed — the survivor keeps its title.
#[must_use]
pub fn merge_pathways(main: &CarePathway, duplicate: &CarePathway) -> MergeOutcome {
    let mut merged = main.clone();

    // The duplicate's former title becomes an alternate name on main.
    let mut alt_seed = main.alternate_names.clone();
    if duplicate.name != main.name && !alt_seed.iter().any(|n| n == &duplicate.name) {
        alt_seed.push(duplicate.name.clone());
    }
    merged.alternate_names = union_strings(&alt_seed, &duplicate.alternate_names);

    // Scalars: keep main's, else adopt the duplicate's.
    merged.pathway_code = main
        .pathway_code
        .clone()
        .or_else(|| duplicate.pathway_code.clone());
    merged.provider_id = main
        .provider_id
        .clone()
        .or_else(|| duplicate.provider_id.clone());
    merged.provider_name = main
        .provider_name
        .clone()
        .or_else(|| duplicate.provider_name.clone());
    merged.care_setting = main
        .care_setting
        .clone()
        .or_else(|| duplicate.care_setting.clone());

    // List unions.
    merged.interventions = union_strings(&main.interventions, &duplicate.interventions);
    merged.keywords = union_strings(&main.keywords, &duplicate.keywords);
    merged.same_as = union_strings(&main.same_as, &duplicate.same_as);
    merged.in_language = union_strings(&main.in_language, &duplicate.in_language);
    merged.condition_codes = union_by(&main.condition_codes, &duplicate.condition_codes, |a, b| {
        a.system == b.system && a.code == b.code
    });
    merged.identifiers = union_by(&main.identifiers, &duplicate.identifiers, |a, b| {
        a.scheme == b.scheme && a.value == b.value
    });

    let transferred = serde_json::to_value(duplicate).unwrap_or(Value::Null);
    MergeOutcome {
        merged,
        transferred,
    }
}

/// Union two string lists, preserving `main`'s order and appending each
/// `extra` not already present (exact, case-sensitive equality).
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

/// Tests for the pure merge fold: alternate-name handling, scalar
/// fallback, list unioning, and the transferred snapshot. DB-free.
#[cfg(test)]
mod tests {
    use super::*;
    use care_pathway_matcher::{CareSetting, CodeSystem, ConditionCode};

    /// Build a [`ConditionCode`] from a system + code (test brevity).
    fn cc(system: CodeSystem, code: &str) -> ConditionCode {
        ConditionCode {
            system,
            code: code.to_string(),
        }
    }

    /// The duplicate's differing title is carried onto the survivor as an
    /// alternate name, while the survivor keeps its own `name`.
    #[test]
    fn duplicate_name_becomes_an_alternate_name() {
        let main = CarePathway::new("Acute Stroke Care Pathway");
        let dup = CarePathway::new("Cerebrovascular Accident Pathway");
        let out = merge_pathways(&main, &dup);
        assert_eq!(out.merged.name, "Acute Stroke Care Pathway");
        assert!(
            out.merged
                .alternate_names
                .contains(&"Cerebrovascular Accident Pathway".to_string())
        );
    }

    /// An identical duplicate title is not duplicated into the survivor's
    /// alternate names.
    #[test]
    fn same_name_is_not_added_as_alternate() {
        let main = CarePathway::new("Stroke");
        let dup = CarePathway::new("Stroke");
        let out = merge_pathways(&main, &dup);
        assert!(out.merged.alternate_names.is_empty());
    }

    /// Scalars keep the survivor's value when present and only adopt the
    /// duplicate's where the survivor's is empty.
    #[test]
    fn scalars_fill_from_duplicate_only_when_main_is_empty() {
        let mut main = CarePathway::new("P");
        main.pathway_code = Some("MAIN-01".into());
        let mut dup = CarePathway::new("Q");
        dup.pathway_code = Some("DUP-01".into());
        dup.provider_id = Some("trust-9".into());
        dup.care_setting = Some(CareSetting::Inpatient);
        let out = merge_pathways(&main, &dup);
        assert_eq!(out.merged.pathway_code.as_deref(), Some("MAIN-01")); // kept
        assert_eq!(out.merged.provider_id.as_deref(), Some("trust-9")); // adopted
        assert_eq!(out.merged.care_setting, Some(CareSetting::Inpatient)); // adopted
    }

    /// List fields union without re-adding overlapping entries (keywords
    /// and condition codes deduped by their identity).
    #[test]
    fn list_fields_union_without_duplicates() {
        let mut main = CarePathway::new("P");
        main.keywords = vec!["stroke".into(), "neuro".into()];
        main.condition_codes = vec![cc(CodeSystem::Icd10, "I63.9")];
        let mut dup = CarePathway::new("Q");
        dup.keywords = vec!["neuro".into(), "acute".into()]; // "neuro" overlaps
        dup.condition_codes = vec![
            cc(CodeSystem::Icd10, "I63.9"),      // same → dropped
            cc(CodeSystem::Snomed, "422504002"), // new
        ];
        let out = merge_pathways(&main, &dup);
        assert_eq!(out.merged.keywords, vec!["stroke", "neuro", "acute"]);
        assert_eq!(out.merged.condition_codes.len(), 2);
    }

    /// The `transferred` snapshot is the full serialized duplicate, kept
    /// in the merge-history record for auditability.
    #[test]
    fn transferred_snapshot_is_the_duplicate() {
        let main = CarePathway::new("P");
        let mut dup = CarePathway::new("Q");
        dup.keywords = vec!["x".into()];
        let out = merge_pathways(&main, &dup);
        assert_eq!(out.transferred["name"], "Q");
        assert_eq!(out.transferred["keywords"][0], "x");
    }
}
