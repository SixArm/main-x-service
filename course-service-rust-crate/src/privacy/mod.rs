//! Privacy controls — masking (FR-16) + GDPR Article-15 export
//! (FR-15).
//!
//! schema.org/Course itself carries little PII, but the
//! `CourseInstance` collection points at instructor person-service
//! ids and free-text instructor names. `mask_course` redacts those
//! plus the `provider_id` reference so an operator-facing read can be
//! shared without revealing the embedded identifiers.
//!
//! `export_course` returns a GDPR-portability JSON envelope wrapping
//! the full unmasked record so a data subject (provider, instructor,
//! or learner referenced from the row) can be served the data we
//! hold about them.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::models::Course;

/// Return a masked copy of the course for FR-16. The original is left
/// unchanged.
pub fn mask_course(c: &Course) -> Course {
    let mut out = c.clone();
    out.provider_id = None;
    for inst in &mut out.instances {
        inst.instructor_ids.clear();
        inst.instructor_names = inst
            .instructor_names
            .iter()
            .map(|_| MASK_LABEL.to_string())
            .collect();
    }
    out
}

/// Placeholder text substituted for redacted free-text fields.
const MASK_LABEL: &str = "[REDACTED]";

/// GDPR Article-15 portability export. Wraps the full unmasked
/// record with export metadata so the subject knows when and from
/// where the snapshot was taken.
#[derive(Debug, Clone, Serialize)]
pub struct CourseExport<'a> {
    /// When the snapshot was taken.
    pub exported_at: DateTime<Utc>,
    /// Originating service name.
    pub source: &'static str,
    /// schema.org type URL the record conforms to.
    pub schema: &'static str,
    /// Borrowed reference to the full, unmasked course.
    pub course: &'a Course,
}

/// Serialise an FR-15 export envelope wrapping `c` to JSON. Falls back
/// to `Value::Null` only if serialisation somehow fails (it cannot for
/// this always-serialisable shape).
pub fn export_course(c: &Course) -> Value {
    let envelope = CourseExport {
        exported_at: Utc::now(),
        source: "course-service",
        schema: "https://schema.org/Course",
        course: c,
    };
    serde_json::to_value(envelope).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    use crate::models::{CourseInstance, CourseInstanceStatus};

    /// Test fixture: an instance with instructor refs and names to mask.
    fn instance(course_id: Uuid) -> CourseInstance {
        CourseInstance {
            id: Uuid::new_v4(),
            course_id,
            name: Some("Spring 2026".into()),
            course_mode: None,
            status: CourseInstanceStatus::Scheduled,
            schedule: None,
            in_language: vec!["en".into()],
            location: None,
            location_id: None,
            instructor_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
            instructor_names: vec!["Prof. Smith".into(), "Prof. Jones".into()],
            maximum_attendee_capacity: Some(30),
            enrolled_count: Some(12),
            enrollment_opens: None,
            enrollment_closes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Masking clears provider_id and instructor refs/names.
    #[test]
    fn masking_clears_provider_and_instructor_refs() {
        let mut c = Course::new("Intro to CS");
        c.provider_id = Some(Uuid::new_v4());
        c.instances.push(instance(c.id));
        let masked = mask_course(&c);
        assert!(masked.provider_id.is_none());
        let inst = &masked.instances[0];
        assert!(inst.instructor_ids.is_empty());
        assert_eq!(inst.instructor_names, vec![MASK_LABEL, MASK_LABEL]);
    }

    /// Masking leaves non-sensitive fields (name, code, keywords) intact.
    #[test]
    fn masking_leaves_non_sensitive_fields_intact() {
        let mut c = Course::new("Intro to CS");
        c.course_code = Some("CS101".into());
        c.keywords = vec!["programming".into()];
        let masked = mask_course(&c);
        assert_eq!(masked.name, "Intro to CS");
        assert_eq!(masked.course_code.as_deref(), Some("CS101"));
        assert_eq!(masked.keywords, vec!["programming".to_string()]);
    }

    /// Masking operates on a copy and never mutates the input.
    #[test]
    fn masking_does_not_mutate_input() {
        let mut c = Course::new("Intro to CS");
        let provider = Uuid::new_v4();
        c.provider_id = Some(provider);
        let _masked = mask_course(&c);
        assert_eq!(c.provider_id, Some(provider));
    }

    /// The export envelope carries source/schema metadata plus the record.
    #[test]
    fn export_envelope_carries_metadata_and_record() {
        let c = Course::new("Intro to CS");
        let v = export_course(&c);
        assert_eq!(v["source"], "course-service");
        assert_eq!(v["schema"], "https://schema.org/Course");
        assert_eq!(v["course"]["name"], "Intro to CS");
        assert!(v["exported_at"].is_string());
    }
}
