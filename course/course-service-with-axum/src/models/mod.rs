//! Domain models for the Course Service.
//!
//! The model surface is aligned with
//! [schema.org/Course](https://schema.org/Course) and its parent
//! `CreativeWork` + `Thing`. The full property mapping lives in
//! [`agents/models.md`](../../agents/models.md).

pub mod course;
pub mod course_instance;
pub mod credential;
pub mod identifier;
pub mod merge;
pub mod organization;
pub mod review_queue;
pub mod syllabus;

pub use course::{
    Course, CourseLink, CourseStatus, EducationalLevel, InteractivityType, LearningResourceType,
    LinkType,
};
pub use course_instance::{CourseInstance, CourseInstanceStatus, CourseMode, Schedule};
pub use credential::{CredentialCategory, EducationalCredential};
pub use identifier::{CourseIdentifier, IdentifierType};
pub use merge::{MergeRecord, MergeRequest, MergeResponse, MergeStatus};
pub use organization::{Provider, ProviderKind};
pub use review_queue::{
    BatchDeduplicationRequest, BatchDeduplicationResponse, DecideOutcome, NewReviewItem,
    ReviewDecisionRequest, ReviewQueueItem, ReviewStatus, canonical_pair,
};
pub use syllabus::Syllabus;
