//! Pure record-merge logic for plans.
//!
//! Merging folds a confirmed-duplicate plan into a surviving "main"
//! record: list fields are unioned, scalar identity fields keep main's
//! value (falling back to the duplicate's when main's is empty), and the
//! duplicate's former name is preserved as an `alternate_names` entry.
//! The DB orchestration (update main, soft-delete the duplicate, write
//! the audit + merge-history rows, publish the event) lives in the
//! controller; this module is pure and unit-testable. The survivor keeps
//! its own optional `kind` label — merging never changes `name` or
//! `kind`.

use project_portfolio_management_matcher::{Goal, Plan};
use serde_json::Value;

/// The result of merging a duplicate into a surviving (main) plan.
pub struct MergeOutcome {
    /// The survivor's new payload — main plus everything transferred.
    pub merged: Plan,
    /// Snapshot of the duplicate at merge time, kept in the merge-history
    /// record for auditability.
    pub transferred: Value,
}

/// Merge `duplicate` into `main`, returning the survivor's new payload.
///
/// - **Scalars** (`code`, `owner_org_id`, `owner_org_name`, `lead_ref`,
///   `parent_ref`, `status`, `start_date`, `target_date`,
///   `in_language`): keep main's value, else take the duplicate's.
/// - **Lists** (`alternate_names`, `goals`, `keywords`, `tags`,
///   `same_as`, `identifiers`, `relationships`): unioned, dropping exact
///   duplicates.
/// - The duplicate's primary `name` is added to the survivor's
///   `alternate_names` (when it differs). `name` and `kind` are never
///   changed.
#[must_use]
pub fn merge_plans(main: &Plan, duplicate: &Plan) -> MergeOutcome {
    let mut merged = main.clone();

    // The duplicate's former name becomes an alternate name on main.
    let mut alt_seed = main.alternate_names.clone();
    if duplicate.name != main.name && !alt_seed.iter().any(|n| n == &duplicate.name) {
        alt_seed.push(duplicate.name.clone());
    }
    merged.alternate_names = union_strings(&alt_seed, &duplicate.alternate_names);

    // Scalars: keep main's, else adopt the duplicate's.
    merged.code = main.code.clone().or_else(|| duplicate.code.clone());
    merged.owner_org_id = main
        .owner_org_id
        .clone()
        .or_else(|| duplicate.owner_org_id.clone());
    merged.owner_org_name = main
        .owner_org_name
        .clone()
        .or_else(|| duplicate.owner_org_name.clone());
    merged.lead_ref = main.lead_ref.clone().or_else(|| duplicate.lead_ref.clone());
    merged.parent_ref = main
        .parent_ref
        .clone()
        .or_else(|| duplicate.parent_ref.clone());
    merged.status = main.status.clone().or_else(|| duplicate.status.clone());
    merged.start_date = main
        .start_date
        .clone()
        .or_else(|| duplicate.start_date.clone());
    merged.target_date = main
        .target_date
        .clone()
        .or_else(|| duplicate.target_date.clone());
    merged.in_language = main
        .in_language
        .clone()
        .or_else(|| duplicate.in_language.clone());

    // List unions.
    merged.keywords = union_strings(&main.keywords, &duplicate.keywords);
    merged.tags = union_strings(&main.tags, &duplicate.tags);
    merged.same_as = union_strings(&main.same_as, &duplicate.same_as);
    merged.goals = union_by(&main.goals, &duplicate.goals, goal_eq);
    merged.identifiers = union_by(&main.identifiers, &duplicate.identifiers, |a, b| {
        a.scheme == b.scheme && a.value == b.value
    });
    merged.relationships = union_by(&main.relationships, &duplicate.relationships, |a, b| {
        a.relation == b.relation && a.plan_id == b.plan_id
    });

    let transferred = serde_json::to_value(duplicate).unwrap_or(Value::Null);
    MergeOutcome {
        merged,
        transferred,
    }
}

/// Two goals are "the same" for the union when their (trimmed,
/// case-insensitive) titles match.
fn goal_eq(a: &Goal, b: &Goal) -> bool {
    a.title.trim().eq_ignore_ascii_case(b.title.trim())
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
    use project_portfolio_management_matcher::{
        IdentifierScheme, PlanIdentifier, PlanRelationship, RelationKind,
    };

    fn project(name: &str) -> Plan {
        Plan::new(name)
    }

    fn ident(scheme: IdentifierScheme, value: &str) -> PlanIdentifier {
        PlanIdentifier {
            scheme,
            value: value.to_string(),
        }
    }

    #[test]
    fn duplicate_name_becomes_an_alternate_name() {
        let main = project("Apollo platform migration");
        let dup = project("Apollo migration");
        let out = merge_plans(&main, &dup);
        assert_eq!(out.merged.name, "Apollo platform migration");
        assert!(
            out.merged
                .alternate_names
                .contains(&"Apollo migration".to_string())
        );
    }

    #[test]
    fn same_name_is_not_added_as_alternate() {
        let main = project("Apollo");
        let dup = project("Apollo");
        assert!(merge_plans(&main, &dup).merged.alternate_names.is_empty());
    }

    #[test]
    fn scalars_fill_from_duplicate_only_when_main_is_empty() {
        let mut main = project("P");
        main.code = Some("MAIN-01".into());
        let mut dup = project("Q");
        dup.code = Some("DUP-01".into());
        dup.owner_org_id = Some("organization:9a2f".into());
        let out = merge_plans(&main, &dup);
        assert_eq!(out.merged.code.as_deref(), Some("MAIN-01")); // kept
        assert_eq!(
            out.merged.owner_org_id.as_deref(),
            Some("organization:9a2f")
        ); // adopted
    }

    #[test]
    fn list_fields_union_without_duplicates() {
        let mut main = project("P");
        main.keywords = vec!["infra".into(), "latency".into()];
        main.identifiers = vec![ident(IdentifierScheme::JiraProjectKey, "APOLLO")];
        main.relationships = vec![PlanRelationship {
            relation: RelationKind::DependsOn,
            plan_id: "p2".into(),
        }];
        let mut dup = project("Q");
        dup.keywords = vec!["latency".into(), "perf".into()]; // "latency" overlaps
        dup.identifiers = vec![
            ident(IdentifierScheme::JiraProjectKey, "APOLLO"), // same → dropped
            ident(IdentifierScheme::LinearId, "LIN-9"),        // new
        ];
        dup.relationships = vec![PlanRelationship {
            relation: RelationKind::DependsOn,
            plan_id: "p2".into(), // same → dropped
        }];
        let out = merge_plans(&main, &dup);
        assert_eq!(out.merged.keywords, vec!["infra", "latency", "perf"]);
        assert_eq!(out.merged.identifiers.len(), 2);
        assert_eq!(out.merged.relationships.len(), 1);
    }

    #[test]
    fn goals_union_by_title_case_insensitive() {
        let mut main = project("P");
        main.goals = vec![Goal {
            title: "Cut latency".into(),
            ..Default::default()
        }];
        let mut dup = project("Q");
        dup.goals = vec![
            Goal {
                title: "cut latency".into(), // dup of main (case-insensitive)
                ..Default::default()
            },
            Goal {
                title: "Improve uptime".into(), // new
                ..Default::default()
            },
        ];
        let out = merge_plans(&main, &dup);
        assert_eq!(out.merged.goals.len(), 2);
    }

    #[test]
    fn transferred_snapshot_is_the_duplicate() {
        let main = project("P");
        let mut dup = project("Q");
        dup.keywords = vec!["x".into()];
        let out = merge_plans(&main, &dup);
        assert_eq!(out.transferred["name"], "Q");
        assert_eq!(out.transferred["keywords"][0], "x");
    }
}
