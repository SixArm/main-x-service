//! Data-quality validation enforcing FR-21..FR-28 from `spec.md §10`.
//!
//! - FR-21: `name` is required, non-empty after trim.
//! - FR-22: `course_code`, when present, MUST be 1-100 chars.
//! - FR-23: `number_of_credits`, when present, MUST be non-negative
//!   (`u32` is already non-negative; we additionally cap at a sane
//!   upper bound to catch obvious nonsense).
//! - FR-24: `in_language` entries MUST be valid BCP-47 codes
//!   (length check only; full RFC-5646 validation is deferred).
//! - FR-25: `url`, `image[*]`, `same_as[*]`, identifier `url`s MUST
//!   start with `http://` or `https://`.
//! - FR-26: `CourseInstance.schedule.end_date` MUST be ≥
//!   `schedule.start_date` when both set.
//! - FR-27: `CourseInstance.enrollment_closes` MUST be ≥
//!   `enrollment_opens` when both set.
//! - FR-28: `CourseInstance.maximum_attendee_capacity` MUST be ≥
//!   `enrolled_count` when both set.
//!
//! Returned errors are field-scoped so the REST layer can surface
//! them as `{field, message}` pairs under a `422` response.

use serde::Serialize;
use utoipa::ToSchema;

use crate::models::{Course, CourseInstance};

/// One validation failure. Pair maps cleanly to the REST `422` body.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, ToSchema)]
pub struct ValidationError {
    /// Dotted/indexed path to the offending field (e.g.
    /// `instances[0].enrollment_closes`).
    pub field: String,
    /// Human-readable explanation of the failure.
    pub message: String,
}

impl ValidationError {
    /// Construct a field-scoped error from any string-like inputs.
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// FR-22 upper bound on `course_code` length.
const COURSE_CODE_MAX: usize = 100;
/// FR-24 minimum plausible BCP-47 code length.
const BCP47_MIN: usize = 2;
/// FR-24 maximum plausible BCP-47 code length.
const BCP47_MAX: usize = 35;
/// FR-23 sanity cap on `number_of_credits`.
const CREDITS_MAX: u32 = 10_000;

/// Validate a [`Course`] against FR-21..FR-28, recursing into nested
/// instances. Returns an empty `Vec` when the record is valid;
/// otherwise one [`ValidationError`] per failing field.
#[must_use]
pub fn validate_course(c: &Course) -> Vec<ValidationError> {
    let mut errs = Vec::new();

    // FR-21
    if c.name.trim().is_empty() {
        errs.push(ValidationError::new(
            "name",
            "name is required and must be non-empty",
        ));
    }

    // FR-22
    if let Some(code) = c.course_code.as_deref() {
        let len = code.chars().count();
        if len == 0 || len > COURSE_CODE_MAX {
            errs.push(ValidationError::new(
                "course_code",
                format!("course_code must be 1..={COURSE_CODE_MAX} characters"),
            ));
        }
    }

    // FR-23
    if let Some(n) = c.number_of_credits
        && n > CREDITS_MAX
    {
        errs.push(ValidationError::new(
            "number_of_credits",
            format!("number_of_credits must be ≤ {CREDITS_MAX}"),
        ));
    }

    // FR-24
    for (i, code) in c.in_language.iter().enumerate() {
        if !is_plausible_bcp47(code) {
            errs.push(ValidationError::new(
                format!("in_language[{i}]"),
                format!("'{code}' is not a plausible BCP-47 language code"),
            ));
        }
    }
    for (i, code) in c.available_language.iter().enumerate() {
        if !is_plausible_bcp47(code) {
            errs.push(ValidationError::new(
                format!("available_language[{i}]"),
                format!("'{code}' is not a plausible BCP-47 language code"),
            ));
        }
    }

    // FR-25
    if let Some(url) = c.url.as_deref()
        && !is_http_url(url)
    {
        errs.push(ValidationError::new(
            "url",
            "url must start with http:// or https://",
        ));
    }
    for (i, u) in c.image.iter().enumerate() {
        if !is_http_url(u) {
            errs.push(ValidationError::new(
                format!("image[{i}]"),
                "image url must start with http:// or https://",
            ));
        }
    }
    for (i, u) in c.same_as.iter().enumerate() {
        if !is_http_url(u) {
            errs.push(ValidationError::new(
                format!("same_as[{i}]"),
                "same_as url must start with http:// or https://",
            ));
        }
    }
    for (i, ident) in c.identifiers.iter().enumerate() {
        if let Some(u) = ident.url.as_deref()
            && !is_http_url(u)
        {
            errs.push(ValidationError::new(
                format!("identifiers[{i}].url"),
                "identifier url must start with http:// or https://",
            ));
        }
    }

    // FR-26..28 — instances nested on the parent
    for (i, inst) in c.instances.iter().enumerate() {
        for mut e in validate_instance(inst) {
            e.field = format!("instances[{i}].{}", e.field);
            errs.push(e);
        }
    }

    errs
}

/// Validate a [`CourseInstance`] against FR-24 and FR-26..FR-28
/// (language codes, schedule ordering, enrollment-window ordering,
/// capacity vs. enrolled count). Returns an empty `Vec` when valid.
#[must_use]
pub fn validate_instance(inst: &CourseInstance) -> Vec<ValidationError> {
    let mut errs = Vec::new();

    // FR-24 (instances too)
    for (i, code) in inst.in_language.iter().enumerate() {
        if !is_plausible_bcp47(code) {
            errs.push(ValidationError::new(
                format!("in_language[{i}]"),
                format!("'{code}' is not a plausible BCP-47 language code"),
            ));
        }
    }

    // FR-26
    if let Some(sched) = inst.schedule.as_ref()
        && let (Some(start), Some(end)) = (sched.start_date, sched.end_date)
        && end < start
    {
        errs.push(ValidationError::new(
            "schedule.end_date",
            "end_date must be on or after start_date",
        ));
    }

    // FR-27
    if let (Some(opens), Some(closes)) = (inst.enrollment_opens, inst.enrollment_closes)
        && closes < opens
    {
        errs.push(ValidationError::new(
            "enrollment_closes",
            "enrollment_closes must be on or after enrollment_opens",
        ));
    }

    // FR-28
    if let (Some(max), Some(enrolled)) = (inst.maximum_attendee_capacity, inst.enrolled_count)
        && max < enrolled
    {
        errs.push(ValidationError::new(
            "maximum_attendee_capacity",
            "maximum_attendee_capacity must be ≥ enrolled_count",
        ));
    }

    errs
}

/// Whether `s` is an `http://` or `https://` URL (case-insensitive,
/// after trimming). The FR-25 web-URL gate.
fn is_http_url(s: &str) -> bool {
    let lower = s.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Coarse BCP-47 plausibility check: length 2–35 chars; only ASCII
/// letters, digits, and hyphens; never starts or ends with a hyphen.
/// Full RFC-5646 validation is deferred — see FR-24 in `spec.md`.
fn is_plausible_bcp47(s: &str) -> bool {
    let len = s.len();
    if !(BCP47_MIN..=BCP47_MAX).contains(&len) {
        return false;
    }
    let bytes = s.as_bytes();
    if bytes[0] == b'-' || bytes[len - 1] == b'-' {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::models::{CourseIdentifier, CourseInstanceStatus, IdentifierType, Schedule};

    /// Test fixture: a course that passes every FR-21..FR-28 rule.
    fn valid_course() -> Course {
        let mut c = Course::new("Intro to CS");
        c.course_code = Some("CS101".into());
        c.url = Some("https://example.edu/cs101".into());
        c.in_language = vec!["en".into(), "en-GB".into()];
        c
    }

    /// A fully-valid course produces no errors.
    #[test]
    fn valid_course_has_no_errors() {
        assert!(validate_course(&valid_course()).is_empty());
    }

    /// FR-21: a whitespace-only name is rejected.
    #[test]
    fn blank_name_is_rejected() {
        let mut c = valid_course();
        c.name = "   ".into();
        let errs = validate_course(&c);
        assert!(errs.iter().any(|e| e.field == "name"));
    }

    /// FR-22: a `course_code` longer than the cap is rejected.
    #[test]
    fn over_length_course_code_is_rejected() {
        let mut c = valid_course();
        c.course_code = Some("X".repeat(101));
        let errs = validate_course(&c);
        assert!(errs.iter().any(|e| e.field == "course_code"));
    }

    /// FR-22: an empty-but-present `course_code` is rejected.
    #[test]
    fn empty_course_code_is_rejected() {
        let mut c = valid_course();
        c.course_code = Some(String::new());
        let errs = validate_course(&c);
        assert!(errs.iter().any(|e| e.field == "course_code"));
    }

    /// FR-25: a non-http(s) `url` scheme is rejected.
    #[test]
    fn non_http_url_is_rejected() {
        let mut c = valid_course();
        c.url = Some("ftp://example.edu".into());
        let errs = validate_course(&c);
        assert!(errs.iter().any(|e| e.field == "url"));
    }

    /// FR-25: an identifier's `url` must also be http(s).
    #[test]
    fn identifier_url_must_be_http() {
        let mut c = valid_course();
        c.identifiers.push(CourseIdentifier {
            property_id: IdentifierType::Doi,
            value: "10.1/x".into(),
            name: None,
            url: Some("javascript:alert(1)".into()),
        });
        let errs = validate_course(&c);
        assert!(errs.iter().any(|e| e.field == "identifiers[0].url"));
    }

    /// FR-24: single-char and leading-hyphen language codes are rejected.
    #[test]
    fn implausible_language_code_is_rejected() {
        let mut c = valid_course();
        c.in_language = vec!["E".into(), "english".into(), "-en".into()];
        let errs = validate_course(&c);
        assert_eq!(
            errs.iter()
                .filter(|e| e.field.starts_with("in_language"))
                .count(),
            2,
            "expected 2 in_language errors (single-char and leading-hyphen), got {errs:?}"
        );
    }

    /// FR-26: an instance schedule ending before it starts is rejected.
    #[test]
    fn schedule_end_before_start_is_rejected() {
        let mut inst = CourseInstance {
            id: uuid::Uuid::new_v4(),
            course_id: uuid::Uuid::new_v4(),
            name: None,
            course_mode: None,
            status: CourseInstanceStatus::default(),
            schedule: None,
            in_language: vec![],
            location: None,
            location_id: None,
            instructor_ids: vec![],
            instructor_names: vec![],
            maximum_attendee_capacity: None,
            enrolled_count: None,
            enrollment_opens: None,
            enrollment_closes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let start = Utc::now();
        let end = start - chrono::Duration::hours(24 * (7));
        inst.schedule = Some(Schedule {
            start_date: Some(start),
            end_date: Some(end),
            time_zone: None,
            recurrence: None,
            sessions: vec![],
        });
        let errs = validate_instance(&inst);
        assert!(errs.iter().any(|e| e.field == "schedule.end_date"));
    }

    /// FR-27: enrollment closing before it opens is rejected.
    #[test]
    fn enrollment_window_must_be_ordered() {
        let inst = CourseInstance {
            id: uuid::Uuid::new_v4(),
            course_id: uuid::Uuid::new_v4(),
            name: None,
            course_mode: None,
            status: CourseInstanceStatus::default(),
            schedule: None,
            in_language: vec![],
            location: None,
            location_id: None,
            instructor_ids: vec![],
            instructor_names: vec![],
            maximum_attendee_capacity: None,
            enrolled_count: None,
            enrollment_opens: Some(Utc::now()),
            enrollment_closes: Some(Utc::now() - chrono::Duration::hours(24)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let errs = validate_instance(&inst);
        assert!(errs.iter().any(|e| e.field == "enrollment_closes"));
    }

    /// FR-28: `enrolled_count` exceeding capacity is rejected.
    #[test]
    fn enrolled_cannot_exceed_capacity() {
        let inst = CourseInstance {
            id: uuid::Uuid::new_v4(),
            course_id: uuid::Uuid::new_v4(),
            name: None,
            course_mode: None,
            status: CourseInstanceStatus::default(),
            schedule: None,
            in_language: vec![],
            location: None,
            location_id: None,
            instructor_ids: vec![],
            instructor_names: vec![],
            maximum_attendee_capacity: Some(30),
            enrolled_count: Some(31),
            enrollment_opens: None,
            enrollment_closes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let errs = validate_instance(&inst);
        assert!(errs.iter().any(|e| e.field == "maximum_attendee_capacity"));
    }

    /// Nested instance errors carry an `instances[i].` field prefix.
    #[test]
    fn nested_instance_errors_are_path_prefixed() {
        let mut c = valid_course();
        c.instances.push(CourseInstance {
            id: uuid::Uuid::new_v4(),
            course_id: c.id,
            name: None,
            course_mode: None,
            status: CourseInstanceStatus::default(),
            schedule: None,
            in_language: vec![],
            location: None,
            location_id: None,
            instructor_ids: vec![],
            instructor_names: vec![],
            maximum_attendee_capacity: Some(10),
            enrolled_count: Some(20),
            enrollment_opens: None,
            enrollment_closes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        let errs = validate_course(&c);
        assert!(
            errs.iter()
                .any(|e| e.field == "instances[0].maximum_attendee_capacity")
        );
    }
}
