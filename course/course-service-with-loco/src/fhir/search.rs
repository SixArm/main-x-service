//! FHIR `Basic` (course) search parameters + the in-memory match predicate.
//!
//! The supported subset ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §6): `_id`, `_count`, `identifier` (token), `code` (token), `name`.
//! `_lastUpdated` is accepted and ignored (no `_history`). Unknown parameters
//! are ignored, not rejected (v1). Filtering is in-memory over the active
//! rows the handler loads, mirroring the native list/scan model.

use serde::Deserialize;

use super::{SYS_COURSE_CODE, scheme_to_system};
use crate::fhir::resources::{RESOURCE_CODE, RESOURCE_CODE_SYSTEM};
use crate::models::Course;

/// Default page size when `_count` is absent (bounded responses).
pub const DEFAULT_COUNT: usize = 50;

/// Parsed FHIR `Basic` (course) search query. Every field is optional; a
/// request with none matches all active rows (up to [`DEFAULT_COUNT`]).
#[derive(Debug, Default, Deserialize)]
pub struct FhirCourseSearchParams {
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
    /// `code` — a token `system|value` (or a bare `value`); matches the
    /// non-standard `{urn:mxi:resource | course}` resource coding.
    #[serde(default)]
    pub code: Option<String>,
    /// `name` — case-insensitive substring over name + alternate names.
    #[serde(default)]
    pub name: Option<String>,
}

/// Case-insensitive substring test.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Split a token into `(system, value)`; a bare token has no system.
fn split_token(token: &str) -> (Option<&str>, &str) {
    token
        .split_once('|')
        .map_or((None, token), |(s, v)| (Some(s), v))
}

impl FhirCourseSearchParams {
    /// The effective result cap (`_count`, else [`DEFAULT_COUNT`]).
    #[must_use]
    pub fn limit(&self) -> usize {
        self.count.unwrap_or(DEFAULT_COUNT)
    }

    /// Whether the stored `course` (with public id `pid`) matches every
    /// supplied parameter (conjunction; absent parameters don't constrain).
    #[must_use]
    pub fn matches(&self, course: &Course, pid: &str) -> bool {
        if let Some(ref id) = self.id
            && pid != id
        {
            return false;
        }
        if let Some(ref name) = self.name
            && !name_matches(course, name)
        {
            return false;
        }
        if let Some(ref ident) = self.identifier
            && !identifier_matches(course, ident)
        {
            return false;
        }
        if let Some(ref code) = self.code
            && !code_matches(code)
        {
            return false;
        }
        true
    }
}

/// Case-insensitive substring of `name` over the course's name + aliases.
fn name_matches(course: &Course, name: &str) -> bool {
    contains_ci(&course.name, name)
        || course
            .alternate_names
            .iter()
            .any(|a| contains_ci(a, name))
}

/// Match an `identifier` token against `course_code` or any
/// [`CourseIdentifier`](crate::models::CourseIdentifier): `system|value`
/// matches both parts; a bare `value` matches any identifier with that value.
fn identifier_matches(course: &Course, token: &str) -> bool {
    let (system, value) = split_token(token);
    let code_hit = course.course_code.as_deref() == Some(value)
        && system.is_none_or(|s| s == SYS_COURSE_CODE);
    let ident_hit = course.identifiers.iter().any(|id| {
        id.value == value && system.is_none_or(|s| scheme_to_system(&id.property_id) == s)
    });
    code_hit || ident_hit
}

/// Match a `code` token against the fixed `{urn:mxi:resource | course}`
/// resource coding every record carries.
fn code_matches(token: &str) -> bool {
    let (system, value) = split_token(token);
    value == RESOURCE_CODE && system.is_none_or(|s| s == RESOURCE_CODE_SYSTEM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CourseIdentifier, IdentifierType};

    fn cs101() -> Course {
        let mut course = Course::new("Introduction to Computer Science");
        course.alternate_names = vec!["Intro to CS".to_string()];
        course.course_code = Some("CS101".to_string());
        course.identifiers = vec![CourseIdentifier {
            property_id: IdentifierType::Doi,
            value: "10.1234/abc".to_string(),
            name: None,
            url: None,
        }];
        course
    }

    #[test]
    fn empty_params_match_everything() {
        let p = FhirCourseSearchParams::default();
        assert!(p.matches(&cs101(), "pid-1"));
        assert_eq!(p.limit(), DEFAULT_COUNT);
    }

    #[test]
    fn name_matches_alias_case_insensitively() {
        let p = FhirCourseSearchParams {
            name: Some("intro to cs".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&cs101(), "pid-1"));
    }

    #[test]
    fn id_must_match_exactly() {
        let p = FhirCourseSearchParams {
            id: Some("pid-1".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&cs101(), "pid-1"));
        assert!(!p.matches(&cs101(), "pid-2"));
    }

    #[test]
    fn identifier_token_matches_course_code_and_scheme() {
        // Scalar course_code, system-qualified.
        let p = FhirCourseSearchParams {
            identifier: Some(format!("{SYS_COURSE_CODE}|CS101")),
            ..Default::default()
        };
        assert!(p.matches(&cs101(), "pid-1"));
        // DOI identifier, system-qualified.
        let p2 = FhirCourseSearchParams {
            identifier: Some("https://doi.org|10.1234/abc".to_string()),
            ..Default::default()
        };
        assert!(p2.matches(&cs101(), "pid-1"));
        // Wrong system ⇒ no match.
        let p3 = FhirCourseSearchParams {
            identifier: Some("https://ror.org|10.1234/abc".to_string()),
            ..Default::default()
        };
        assert!(!p3.matches(&cs101(), "pid-1"));
        // Bare value ⇒ match.
        let p4 = FhirCourseSearchParams {
            identifier: Some("CS101".to_string()),
            ..Default::default()
        };
        assert!(p4.matches(&cs101(), "pid-1"));
    }

    #[test]
    fn code_matches_course_coding() {
        let hit = FhirCourseSearchParams {
            code: Some("course".to_string()),
            ..Default::default()
        };
        assert!(hit.matches(&cs101(), "pid-1"));
        let qualified = FhirCourseSearchParams {
            code: Some(format!("{RESOURCE_CODE_SYSTEM}|course")),
            ..Default::default()
        };
        assert!(qualified.matches(&cs101(), "pid-1"));
        let miss = FhirCourseSearchParams {
            code: Some("patient".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&cs101(), "pid-1"));
    }
}
