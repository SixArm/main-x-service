//! FHIR `PlanDefinition` search parameters + the in-memory match predicate.
//!
//! The supported subset ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §6): `_id`, `_count`, `identifier` (token), `name` (over title +
//! alternate names), `status`. `_lastUpdated` is accepted and ignored (no
//! `_history`). Unknown parameters are ignored, not rejected (v1).
//! Filtering is in-memory over the active rows the handler loads, matching
//! the native `check-duplicates` scan model.

use care_pathway_matcher::CarePathway;
use serde::Deserialize;

use super::scheme_to_system;

/// Default page size when `_count` is absent (mirrors the native list cap
/// intent — bounded responses).
pub const DEFAULT_COUNT: usize = 50;

/// Parsed FHIR `PlanDefinition` search query. Every field is optional; a
/// request with none matches all active rows (up to [`DEFAULT_COUNT`]).
#[derive(Debug, Default, Deserialize)]
pub struct FhirPlanSearchParams {
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
    /// `name` — case-insensitive substring over title + alternate names.
    #[serde(default)]
    pub name: Option<String>,
    /// `status` — exact match on the derived `active` / `retired` status.
    #[serde(default)]
    pub status: Option<String>,
}

/// Case-insensitive substring test.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

impl FhirPlanSearchParams {
    /// The effective result cap (`_count`, else [`DEFAULT_COUNT`]).
    #[must_use]
    pub fn limit(&self) -> usize {
        self.count.unwrap_or(DEFAULT_COUNT)
    }

    /// Whether the stored `pathway` (with public id `pid` and derived
    /// `status`) matches every supplied parameter (conjunction; absent
    /// parameters don't constrain).
    #[must_use]
    pub fn matches(&self, pathway: &CarePathway, pid: &str, status: &str) -> bool {
        if let Some(ref id) = self.id
            && pid != id
        {
            return false;
        }
        if let Some(ref want) = self.status
            && !want.eq_ignore_ascii_case(status)
        {
            return false;
        }
        if let Some(ref name) = self.name
            && !name_matches(pathway, name)
        {
            return false;
        }
        if let Some(ref ident) = self.identifier
            && !identifier_matches(pathway, ident)
        {
            return false;
        }
        true
    }
}

/// Case-insensitive substring of `name` over the pathway's name and
/// alternate names.
fn name_matches(pathway: &CarePathway, name: &str) -> bool {
    contains_ci(&pathway.name, name) || pathway.alternate_names.iter().any(|a| contains_ci(a, name))
}

/// Match an `identifier` token: `system|value` matches both parts; a bare
/// `value` matches any external identifier with that value.
fn identifier_matches(pathway: &CarePathway, token: &str) -> bool {
    let (system, value) = token
        .split_once('|')
        .map_or((None, token), |(s, v)| (Some(s), v));
    pathway.identifiers.iter().any(|id| {
        let value_hit = id.value == value;
        match system {
            Some(sys) => value_hit && scheme_to_system(&id.scheme) == sys,
            None => value_hit,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use care_pathway_matcher::{IdentifierScheme, PathwayIdentifier};

    fn stroke() -> CarePathway {
        CarePathway {
            alternate_names: vec!["ASCP".to_string()],
            identifiers: vec![PathwayIdentifier {
                scheme: IdentifierScheme::GuidelineId,
                value: "NG128".to_string(),
            }],
            ..CarePathway::new("Acute Stroke Care Pathway")
        }
    }

    #[test]
    fn empty_params_match_everything() {
        let p = FhirPlanSearchParams::default();
        assert!(p.matches(&stroke(), "pid-1", "active"));
        assert_eq!(p.limit(), DEFAULT_COUNT);
    }

    #[test]
    fn name_matches_alias_case_insensitively() {
        let p = FhirPlanSearchParams {
            name: Some("ascp".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&stroke(), "pid-1", "active"));
    }

    #[test]
    fn id_must_match_exactly() {
        let p = FhirPlanSearchParams {
            id: Some("pid-1".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&stroke(), "pid-1", "active"));
        assert!(!p.matches(&stroke(), "pid-2", "active"));
    }

    #[test]
    fn status_filters_active_vs_retired() {
        let p = FhirPlanSearchParams {
            status: Some("active".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&stroke(), "pid-1", "active"));
        assert!(!p.matches(&stroke(), "pid-1", "retired"));
    }

    #[test]
    fn identifier_token_matches_system_and_value() {
        let p = FhirPlanSearchParams {
            identifier: Some("urn:mxi:carepathway:guideline|NG128".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&stroke(), "pid-1", "active"));
        // Wrong system ⇒ no match.
        let p2 = FhirPlanSearchParams {
            identifier: Some("https://doi.org|NG128".to_string()),
            ..Default::default()
        };
        assert!(!p2.matches(&stroke(), "pid-1", "active"));
        // Bare value ⇒ match.
        let p3 = FhirPlanSearchParams {
            identifier: Some("NG128".to_string()),
            ..Default::default()
        };
        assert!(p3.matches(&stroke(), "pid-1", "active"));
    }
}
