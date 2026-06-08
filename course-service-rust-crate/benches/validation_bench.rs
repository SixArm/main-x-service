//! Validation micro-benchmarks. Establishes a baseline for one
//! `validate_course` pass on a fully-populated record (every
//! FR-21..FR-28 branch exercised at least once).

use jiff::Timestamp;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use uuid::Uuid;

use course_service::models::{
    Course, CourseIdentifier, CourseInstance, CourseInstanceStatus, IdentifierType, Schedule,
};
use course_service::validation::validate_course;

/// Build a fully-populated course (one nested instance, identifiers,
/// URLs, languages, credits) so a single validate pass exercises every
/// FR-21..FR-28 branch at least once.
fn populated_course() -> Course {
    let mut c = Course::new("Introduction to Computer Science");
    c.course_code = Some("CS101".into());
    c.url = Some("https://example.edu/cs101".into());
    c.image = vec!["https://example.edu/cs101.png".into()];
    c.same_as = vec!["https://wikidata.org/wiki/Q12345".into()];
    c.in_language = vec!["en".into(), "en-GB".into()];
    c.available_language = vec!["en".into(), "fr".into()];
    c.identifiers = vec![CourseIdentifier {
        property_id: IdentifierType::Doi,
        value: "10.1234/intro-cs".into(),
        name: None,
        url: Some("https://doi.org/10.1234/intro-cs".into()),
    }];
    c.number_of_credits = Some(3);
    let now = Timestamp::now();
    c.instances.push(CourseInstance {
        id: Uuid::new_v4(),
        course_id: c.id,
        name: Some("Spring 2026".into()),
        course_mode: None,
        status: CourseInstanceStatus::Scheduled,
        schedule: Some(Schedule {
            start_date: Some(now),
            end_date: Some(now + jiff::SignedDuration::from_hours(24 * (120))),
            time_zone: Some("UTC".into()),
            recurrence: None,
            sessions: vec![],
        }),
        in_language: vec!["en".into()],
        location: Some("Online".into()),
        location_id: None,
        instructor_ids: vec![Uuid::new_v4()],
        instructor_names: vec!["Prof. Smith".into()],
        maximum_attendee_capacity: Some(60),
        enrolled_count: Some(42),
        enrollment_opens: Some(now),
        enrollment_closes: Some(now + jiff::SignedDuration::from_hours(24 * (30))),
        created_at: now,
        updated_at: now,
    });
    c
}

/// Benchmark a single `validate_course` pass over the populated record.
fn bench_validate_populated(c: &mut Criterion) {
    let course = populated_course();
    c.bench_function("validation/validate_course/populated", |b| {
        b.iter(|| {
            black_box(validate_course(black_box(&course)));
        });
    });
}

criterion_group!(validation, bench_validate_populated);
criterion_main!(validation);
