//! Case's declared bulk-import **stable key**
//! (`agents/share/bulk-import-export.md` §10.1).
//!
//! The stable key is what drives idempotent upsert: on import, a row
//! whose stable key matches an existing record updates it in place
//! rather than creating a duplicate, so re-running the same file is a
//! no-op.
//!
//! Case's precedence (most-to-least specific):
//!
//! 1. The **agency-scoped case number** — the `(agency_id, case_number)`
//!    pair, both present and non-blank after trimming. A case number is
//!    only unique *within* an agency (mirroring the case-matcher's own
//!    deterministic short-circuit — see the case-matcher spec's R-1 rule
//!    and `agents/share/overview.md`'s "agency-scoped case number"
//!    description), so the pair together is the key, never `case_number`
//!    alone.
//! 2. The record **`pid`** ([`BulkCaseRow::pid`]) as the fallback — this
//!    is what makes re-importing an *export* (which always carries pids)
//!    idempotent even when a case has no case number recorded.
//!
//! Unlike the person reference implementation, there is no third
//! "always-present" fallback: `case_matcher::Case` has no `tax_id`-style
//! convenience scalar, and — critically — [`BulkCaseRow::pid`] is a
//! genuine `Option<Uuid>` with no fabricated default (see [`row`](super::row)'s
//! module docs), so [`resolve_stable_key`] can simply return `None` for a
//! row with neither. That `None` **is** the keyless signal; there is no
//! separate `row_has_explicit_id` byte-sniff to reconcile it with.

use super::row::BulkCaseRow;
use uuid::Uuid;

/// A resolved stable key for one import row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StableKey {
    /// Match on the agency-scoped case number.
    AgencyCaseNumber {
        /// The handling agency's identifier.
        agency_id: String,
        /// The agency's local case number.
        case_number: String,
    },
    /// Match on the record's own pid.
    Pid(Uuid),
}

/// Resolve the stable key for `row`, per the precedence in the module
/// docs. Returns `None` for a **keyless** row (see [`is_keyless`]).
#[must_use]
pub fn resolve_stable_key(row: &BulkCaseRow) -> Option<StableKey> {
    let agency_id = trimmed_non_empty(row.case.agency_id.as_deref());
    let case_number = trimmed_non_empty(row.case.case_number.as_deref());
    if let (Some(agency_id), Some(case_number)) = (agency_id, case_number) {
        return Some(StableKey::AgencyCaseNumber {
            agency_id: agency_id.to_string(),
            case_number: case_number.to_string(),
        });
    }
    row.pid.map(StableKey::Pid)
}

/// Whether `row` is **keyless**
/// (`agents/share/bulk-import-export.md` §6): no agency-scoped case
/// number and no pid of its own.
///
/// A keyless row cannot be idempotently upserted — there is no existing
/// record for [`resolve_stable_key`] to point at — so the import
/// pipeline routes it through duplicate detection instead of a blind
/// create.
#[must_use]
pub fn is_keyless(row: &BulkCaseRow) -> bool {
    resolve_stable_key(row).is_none()
}

/// Trim and treat a blank string as absent.
fn trimmed_non_empty(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{StableKey, is_keyless, resolve_stable_key};
    use crate::bulk::row::BulkCaseRow;
    use case_matcher::Case;
    use uuid::Uuid;

    fn case_with(agency_id: Option<&str>, case_number: Option<&str>) -> Case {
        Case {
            agency_id: agency_id.map(str::to_string),
            case_number: case_number.map(str::to_string),
            ..Case::new("A case")
        }
    }

    #[test]
    fn prefers_agency_case_number_over_pid() {
        let row = BulkCaseRow {
            pid: Some(Uuid::new_v4()),
            case: case_with(Some("dhs"), Some("CN-001")),
        };
        assert_eq!(
            resolve_stable_key(&row),
            Some(StableKey::AgencyCaseNumber {
                agency_id: "dhs".to_string(),
                case_number: "CN-001".to_string(),
            })
        );
    }

    #[test]
    fn falls_back_to_pid_when_agency_or_case_number_absent() {
        let pid = Uuid::new_v4();
        for (agency_id, case_number) in [(None, Some("CN-001")), (Some("dhs"), None), (None, None)]
        {
            let row = BulkCaseRow {
                pid: Some(pid),
                case: case_with(agency_id, case_number),
            };
            assert_eq!(resolve_stable_key(&row), Some(StableKey::Pid(pid)));
        }
    }

    #[test]
    fn blank_agency_or_case_number_is_treated_as_absent() {
        let pid = Uuid::new_v4();
        let row = BulkCaseRow {
            pid: Some(pid),
            case: case_with(Some("   "), Some("CN-001")),
        };
        assert_eq!(
            resolve_stable_key(&row),
            Some(StableKey::Pid(pid)),
            "a blank agency_id must not combine with case_number"
        );
    }

    #[test]
    fn no_key_and_no_pid_is_keyless() {
        let row = BulkCaseRow {
            pid: None,
            case: case_with(None, None),
        };
        assert_eq!(resolve_stable_key(&row), None);
        assert!(is_keyless(&row));
    }

    #[test]
    fn a_bare_pid_is_not_keyless() {
        let row = BulkCaseRow {
            pid: Some(Uuid::new_v4()),
            case: case_with(None, None),
        };
        assert!(!is_keyless(&row));
    }

    #[test]
    fn agency_case_number_pair_is_not_keyless() {
        let row = BulkCaseRow {
            pid: None,
            case: case_with(Some("dhs"), Some("CN-001")),
        };
        assert!(!is_keyless(&row));
    }
}
