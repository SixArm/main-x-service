//! Matching micro-benchmarks. Establishes a per-pair scoring
//! baseline for the canonical `course-matcher` algorithm driven
//! through the service's `CourseMatcher` facade + adapter.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use uuid::Uuid;

use course_service::config::MatchingConfig;
use course_service::matching::CourseMatcher;
use course_service::models::{
    Course, CourseIdentifier, EducationalLevel, IdentifierType, LearningResourceType,
};

fn cfg() -> MatchingConfig {
    MatchingConfig { threshold_score: 0.85 }
}

fn populated_course(provider: Uuid, name: &str, code: &str) -> Course {
    let mut c = Course::new(name);
    c.course_code = Some(code.into());
    c.provider_id = Some(provider);
    c.alternate_names = vec!["Intro CS".into(), "CS Foundations".into()];
    c.keywords = vec![
        "programming".into(),
        "algorithms".into(),
        "data structures".into(),
    ];
    c.teaches = vec!["recursion".into(), "asymptotics".into()];
    c.educational_level = Some(EducationalLevel::Undergraduate);
    c.learning_resource_type = Some(LearningResourceType::Lecture);
    c.same_as = vec!["https://wikidata.org/wiki/Q12345".into()];
    c.identifiers = vec![CourseIdentifier {
        property_id: IdentifierType::Doi,
        value: "10.1234/intro-cs".into(),
        name: None,
        url: None,
    }];
    c
}

fn bench_match_pair(c: &mut Criterion) {
    let matcher = CourseMatcher::new(cfg());
    let provider = Uuid::new_v4();
    let a = populated_course(provider, "Introduction to Computer Science", "CS101");
    let b = populated_course(provider, "Intro to Computer Science", "CS 101");

    c.bench_function("match_courses/populated_pair", |bencher| {
        bencher.iter(|| {
            black_box(matcher.match_courses(black_box(&a), black_box(&b)));
        });
    });
}

fn bench_match_deterministic(c: &mut Criterion) {
    let matcher = CourseMatcher::new(cfg());
    let mut a = Course::new("Linear Algebra");
    let mut b = Course::new("Wholly Different Course");
    let ident = CourseIdentifier {
        property_id: IdentifierType::Doi,
        value: "10.9999/short-circuit".into(),
        name: None,
        url: None,
    };
    a.identifiers = vec![ident.clone()];
    b.identifiers = vec![ident];

    c.bench_function("match_courses/deterministic_short_circuit", |bencher| {
        bencher.iter(|| {
            black_box(matcher.match_courses(black_box(&a), black_box(&b)));
        });
    });
}

fn bench_rank_100(c: &mut Criterion) {
    let matcher = CourseMatcher::new(cfg());
    let provider = Uuid::new_v4();
    let probe = populated_course(provider, "Introduction to Computer Science", "CS101");
    let candidates: Vec<Course> = (0..100)
        .map(|i| populated_course(provider, &format!("Course Number {i}"), &format!("X{i:03}")))
        .collect();

    c.bench_function("find_matches/rank_100_candidates", |bencher| {
        bencher.iter(|| {
            black_box(matcher.find_matches(black_box(&probe), black_box(&candidates)));
        });
    });
}

criterion_group!(matching, bench_match_pair, bench_match_deterministic, bench_rank_100);
criterion_main!(matching);
