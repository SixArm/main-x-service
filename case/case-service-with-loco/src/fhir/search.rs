//! FHIR `Task` search parameters + the in-memory match predicate.
//!
//! The supported subset ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §6, reflected in the `CapabilityStatement`): `_id`, `_lastUpdated`,
//! `_count`, `identifier` (token), `status`, `priority`. `_lastUpdated` is
//! accepted and ignored (no `_history`). Unknown parameters are ignored,
//! not rejected (v1). Filtering is in-memory over the active rows the
//! handler loads, matching the native `check-duplicates` scan model.

use case_matcher::Case;
use serde::Deserialize;

use super::{CASE_NUMBER_SYSTEM, case_status_to_fhir, priority_to_fhir, scheme_to_system};

/// Default page size when `_count` is absent (bounded responses).
pub const DEFAULT_COUNT: usize = 50;

/// Parsed FHIR `Task` search query. Every field is optional; a request
/// with none matches all active rows (up to [`DEFAULT_COUNT`]).
#[derive(Debug, Default, Deserialize)]
pub struct FhirTaskSearchParams {
    /// `_id` — exact match on the resource id (`pid`).
    #[serde(rename = "_id", default)]
    pub id: Option<String>,
    /// `_lastUpdated` — accepted for compatibility, currently ignored.
    #[serde(rename = "_lastUpdated", default)]
    pub last_updated: Option<String>,
    /// `_count` — page size (result cap).
    #[serde(rename = "_count", default)]
    pub count: Option<usize>,
    /// `identifier` — a token `system|value` (or a bare `value`).
    #[serde(default)]
    pub identifier: Option<String>,
    /// `status` — the FHIR `Task.status` code (e.g. `in-progress`).
    #[serde(default)]
    pub status: Option<String>,
    /// `priority` — the FHIR `RequestPriority` code (e.g. `urgent`).
    #[serde(default)]
    pub priority: Option<String>,
}

impl FhirTaskSearchParams {
    /// The effective result cap (`_count`, else [`DEFAULT_COUNT`]).
    #[must_use]
    pub fn limit(&self) -> usize {
        self.count.unwrap_or(DEFAULT_COUNT)
    }

    /// Whether the stored `case` (with public id `pid`) matches every
    /// supplied parameter (conjunction; absent parameters don't constrain).
    #[must_use]
    pub fn matches(&self, case: &Case, pid: &str) -> bool {
        if let Some(ref id) = self.id
            && pid != id
        {
            return false;
        }
        if let Some(ref status) = self.status
            && case.status.as_ref().map(case_status_to_fhir) != Some(status.as_str())
        {
            return false;
        }
        if let Some(ref priority) = self.priority
            && case.priority.as_ref().map(priority_to_fhir) != Some(priority.as_str())
        {
            return false;
        }
        if let Some(ref ident) = self.identifier
            && !identifier_matches(case, ident)
        {
            return false;
        }
        true
    }
}

/// Match an `identifier` token against every identifier the resource would
/// emit — the general `identifiers` (each `scheme|value`) and the
/// agency-scoped `case_number` (`CASE_NUMBER_SYSTEM|value`). A
/// `system|value` token matches both parts; a bare `value` matches any
/// identifier with that value.
fn identifier_matches(case: &Case, token: &str) -> bool {
    let (system, value) = token
        .split_once('|')
        .map_or((None, token), |(s, v)| (Some(s), v));

    let hit = |sys: &str, val: &str| -> bool { val == value && system.is_none_or(|s| s == sys) };

    if let Some(ref number) = case.case_number
        && hit(CASE_NUMBER_SYSTEM, number)
    {
        return true;
    }
    case.identifiers
        .iter()
        .any(|id| hit(&scheme_to_system(&id.scheme), &id.value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use case_matcher::{CaseIdentifier, CaseStatus, IdentifierScheme, Priority};

    fn housing() -> Case {
        let mut case = Case::new("Housing benefit appeal");
        case.case_number = Some("HB-2026-01".to_string());
        case.status = Some(CaseStatus::InProgress);
        case.priority = Some(Priority::High);
        case.identifiers = vec![CaseIdentifier {
            scheme: IdentifierScheme::Docket,
            value: "CV-2024-001234".to_string(),
        }];
        case
    }

    #[test]
    fn empty_params_match_everything() {
        let p = FhirTaskSearchParams::default();
        assert!(p.matches(&housing(), "pid-1"));
        assert_eq!(p.limit(), DEFAULT_COUNT);
    }

    #[test]
    fn id_must_match_exactly() {
        let p = FhirTaskSearchParams {
            id: Some("pid-1".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&housing(), "pid-1"));
        assert!(!p.matches(&housing(), "pid-2"));
    }

    #[test]
    fn status_and_priority_filter() {
        let p = FhirTaskSearchParams {
            status: Some("in-progress".to_string()),
            priority: Some("urgent".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&housing(), "pid-1"));
        let miss = FhirTaskSearchParams {
            status: Some("completed".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&housing(), "pid-1"));
    }

    #[test]
    fn identifier_token_matches_system_and_value() {
        // General identifier, system|value.
        let p = FhirTaskSearchParams {
            identifier: Some("urn:mxi:case:docket|CV-2024-001234".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&housing(), "pid-1"));
        // Wrong system ⇒ no match.
        let p2 = FhirTaskSearchParams {
            identifier: Some("urn:mxi:case:uri|CV-2024-001234".to_string()),
            ..Default::default()
        };
        assert!(!p2.matches(&housing(), "pid-1"));
        // Bare value ⇒ match.
        let p3 = FhirTaskSearchParams {
            identifier: Some("CV-2024-001234".to_string()),
            ..Default::default()
        };
        assert!(p3.matches(&housing(), "pid-1"));
        // The agency-scoped case number is searchable too.
        let p4 = FhirTaskSearchParams {
            identifier: Some("urn:mxi:case:case-number|HB-2026-01".to_string()),
            ..Default::default()
        };
        assert!(p4.matches(&housing(), "pid-1"));
    }
}
