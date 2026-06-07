//! SeaORM entity modules.
//!
//! One module per table from `migrations/`. JSONB columns are typed as
//! `serde_json::Value` and rehydrated to typed collections by the
//! repository.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ───────────────────────── providers ─────────────────────────

/// Entity for the `providers` table (course-authoring organisations).
pub mod providers {
    use super::*;

    /// A row in the `providers` table; maps to
    /// [`Provider`](crate::models::organization::Provider).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "providers")]
    pub struct Model {
        /// Provider UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Canonical name.
        pub name: String,
        /// Alternate names (JSONB array).
        pub alternate_names: Json,
        /// Website URL.
        pub url: Option<String>,
        /// External authority URLs (JSONB array).
        pub same_as: Json,
        /// Provider kind, stored as a string.
        pub kind: Option<String>,
        /// Row creation timestamp.
        pub created_at: DateTimeUtc,
        /// Row last-update timestamp.
        pub updated_at: DateTimeUtc,
        /// Soft-delete timestamp, if deleted.
        pub deleted_at: Option<DateTimeUtc>,
    }

    /// Foreign-key relations from `providers`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// One provider authors many courses.
        #[sea_orm(has_many = "super::courses::Entity")]
        Courses,
    }

    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef { Relation::Courses.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── courses ─────────────────────────

/// Entity for the `courses` table — the schema.org/Course template row.
pub mod courses {
    use super::*;

    /// A row in the `courses` table; maps to
    /// [`Course`](crate::models::course::Course). Collection fields are
    /// JSONB; enum fields are stored as JSONB or bare strings.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "courses")]
    pub struct Model {
        /// Course UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Course name.
        pub name: String,
        /// Alternate names (JSONB array).
        pub alternate_names: Json,
        /// Long description.
        pub description: Option<String>,
        /// Disambiguating one-liner.
        pub disambiguating_description: Option<String>,
        /// Canonical course URL.
        pub url: Option<String>,
        /// Image URLs (JSONB array).
        pub image: Json,
        /// External authority URLs (JSONB array).
        pub same_as: Json,
        /// Keyword tags (JSONB array).
        pub keywords: Json,
        /// schema.org/additionalType.
        pub additional_type: Option<String>,
        /// schema.org/about subjects (JSONB array).
        pub about: Json,
        /// Target audience.
        pub audience: Option<String>,
        /// Languages of instruction (JSONB array).
        pub in_language: Json,
        /// License URL/text.
        pub license: Option<String>,
        /// Typical learner age range.
        pub typical_age_range: Option<String>,
        /// ISO 8601 duration.
        pub time_required: Option<String>,
        /// Version label.
        pub version: Option<String>,
        /// Whether the course is free to access.
        pub is_accessible_for_free: Option<bool>,
        /// Competencies taught (JSONB array).
        pub teaches: Json,
        /// Competencies assessed (JSONB array).
        pub assesses: Json,
        /// Required prior competencies (JSONB array).
        pub competency_required: Json,
        /// Educational level enum (JSONB).
        pub educational_level: Option<Json>,
        /// schema.org/educationalUse.
        pub educational_use: Option<String>,
        /// Learning-resource-type enum (JSONB).
        pub learning_resource_type: Option<Json>,
        /// Interactivity-type enum (bare string).
        pub interactivity_type: Option<String>,
        /// Provider catalog code.
        pub course_code: Option<String>,
        /// Credit count (stored as `i32`).
        pub number_of_credits: Option<i32>,
        /// Prerequisite descriptions (JSONB array).
        pub course_prerequisites: Json,
        /// Available languages (JSONB array).
        pub available_language: Json,
        /// Financial-aid eligibility notes (JSONB array).
        pub financial_aid_eligible: Json,
        /// Educational credential awarded (JSONB).
        pub educational_credential_awarded: Option<Json>,
        /// Occupational credential awarded (JSONB).
        pub occupational_credential_awarded: Option<Json>,
        /// Historical enrollment count (stored as `i64`).
        pub total_historical_enrollment: Option<i64>,
        /// Lifecycle status (bare string).
        pub status: String,
        /// Active flag.
        pub active: bool,
        /// FK to the owning provider.
        pub provider_id: Option<Uuid>,
        /// Row creation timestamp.
        pub created_at: DateTimeUtc,
        /// Row last-update timestamp.
        pub updated_at: DateTimeUtc,
        /// Soft-delete timestamp, if deleted.
        pub deleted_at: Option<DateTimeUtc>,
    }

    /// Foreign-key relations from `courses`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Child identifier rows.
        #[sea_orm(has_many = "super::course_identifiers::Entity")]
        CourseIdentifiers,
        /// Child instance rows.
        #[sea_orm(has_many = "super::course_instances::Entity")]
        CourseInstances,
        /// Child syllabus-section rows.
        #[sea_orm(has_many = "super::syllabus_sections::Entity")]
        SyllabusSections,
        /// Owning provider.
        #[sea_orm(
            belongs_to = "super::providers::Entity",
            from = "Column::ProviderId",
            to = "super::providers::Column::Id"
        )]
        Provider,
    }

    impl Related<super::course_identifiers::Entity> for Entity {
        fn to() -> RelationDef { Relation::CourseIdentifiers.def() }
    }
    impl Related<super::course_instances::Entity> for Entity {
        fn to() -> RelationDef { Relation::CourseInstances.def() }
    }
    impl Related<super::syllabus_sections::Entity> for Entity {
        fn to() -> RelationDef { Relation::SyllabusSections.def() }
    }
    impl Related<super::providers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Provider.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_identifiers ─────────────────────

/// Entity for the `course_identifiers` table (external identifiers).
pub mod course_identifiers {
    use super::*;

    /// A row in `course_identifiers`; maps to
    /// [`CourseIdentifier`](crate::models::identifier::CourseIdentifier).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_identifiers")]
    pub struct Model {
        /// Identifier-row UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning course.
        pub course_id: Uuid,
        /// Identifier scheme enum (JSONB).
        pub property_id: Json,
        /// Scheme-specific value.
        pub value: String,
        /// Optional label.
        pub name: Option<String>,
        /// Optional authority URL.
        pub url: Option<String>,
        /// Row creation timestamp.
        pub created_at: DateTimeUtc,
    }

    /// Foreign-key relations from `course_identifiers`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Owning course.
        #[sea_orm(
            belongs_to = "super::courses::Entity",
            from = "Column::CourseId",
            to = "super::courses::Column::Id"
        )]
        Course,
    }

    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef { Relation::Course.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── course_links ─────────────────────────

/// Entity for the `course_links` table (typed course-to-course links).
pub mod course_links {
    use super::*;

    /// A row in `course_links`; maps to
    /// [`CourseLink`](crate::models::course::CourseLink).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_links")]
    pub struct Model {
        /// Link-row UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the source course.
        pub course_id: Uuid,
        /// FK to the linked course.
        pub other_course_id: Uuid,
        /// Link type (bare string).
        pub link_type: String,
        /// Row creation timestamp.
        pub created_at: DateTimeUtc,
    }

    /// Foreign-key relations from `course_links`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Source course.
        #[sea_orm(
            belongs_to = "super::courses::Entity",
            from = "Column::CourseId",
            to = "super::courses::Column::Id"
        )]
        Course,
    }

    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef { Relation::Course.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────────── course_instances ───────────────────────

/// Entity for the `course_instances` table (specific offerings).
pub mod course_instances {
    use super::*;

    /// A row in `course_instances`; maps to
    /// [`CourseInstance`](crate::models::course_instance::CourseInstance).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_instances")]
    pub struct Model {
        /// Instance UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the parent course.
        pub course_id: Uuid,
        /// Instance name.
        pub name: Option<String>,
        /// Delivery mode enum (bare string).
        pub course_mode: Option<String>,
        /// Lifecycle status (bare string).
        pub status: String,
        /// Languages for this offering (JSONB array).
        pub in_language: Json,
        /// Free-text location.
        pub location: Option<String>,
        /// External place-service location reference.
        pub location_id: Option<Uuid>,
        /// Instructor person-service ids (JSONB array).
        pub instructor_ids: Json,
        /// Free-text instructor names (JSONB array).
        pub instructor_names: Json,
        /// Maximum capacity (stored as `i32`).
        pub maximum_attendee_capacity: Option<i32>,
        /// Current enrollment (stored as `i32`).
        pub enrolled_count: Option<i32>,
        /// Enrollment window open time.
        pub enrollment_opens: Option<DateTimeUtc>,
        /// Enrollment window close time.
        pub enrollment_closes: Option<DateTimeUtc>,
        /// Schedule struct (JSONB).
        pub schedule: Option<Json>,
        /// Row creation timestamp.
        pub created_at: DateTimeUtc,
        /// Row last-update timestamp.
        pub updated_at: DateTimeUtc,
        /// Soft-delete timestamp, if deleted.
        pub deleted_at: Option<DateTimeUtc>,
    }

    /// Foreign-key relations from `course_instances`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Parent course.
        #[sea_orm(
            belongs_to = "super::courses::Entity",
            from = "Column::CourseId",
            to = "super::courses::Column::Id"
        )]
        Course,
    }

    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef { Relation::Course.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────────── syllabus_sections ───────────────────────

/// Entity for the `syllabus_sections` table (course-outline tree).
pub mod syllabus_sections {
    use super::*;

    /// A row in `syllabus_sections`; maps to
    /// [`Syllabus`](crate::models::syllabus::Syllabus). `parent_id`
    /// makes the table a self-referential tree.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "syllabus_sections")]
    pub struct Model {
        /// Section UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning course.
        pub course_id: Uuid,
        /// FK to the parent section, if nested.
        pub parent_id: Option<Uuid>,
        /// Section heading.
        pub name: String,
        /// Section description.
        pub description: Option<String>,
        /// Ordering position (stored as `i32`).
        pub position: Option<i32>,
        /// Competencies covered (JSONB array).
        pub teaches: Json,
        /// ISO 8601 duration.
        pub time_required: Option<String>,
        /// Resource URLs (JSONB array).
        pub resources: Json,
        /// Row creation timestamp.
        pub created_at: DateTimeUtc,
    }

    /// Foreign-key relations from `syllabus_sections`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Owning course.
        #[sea_orm(
            belongs_to = "super::courses::Entity",
            from = "Column::CourseId",
            to = "super::courses::Column::Id"
        )]
        Course,
    }

    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef { Relation::Course.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── audit_log ─────────────────────────

/// Entity for the `audit_log` table (HIPAA-style change trail).
pub mod audit_log {
    use super::*;

    /// A row in `audit_log`; projected to
    /// [`AuditEntry`](crate::db::audit::AuditEntry) for the API.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        /// Audit-row UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Affected entity kind.
        pub entity_type: String,
        /// Affected entity id.
        pub entity_id: Uuid,
        /// `CREATE` / `UPDATE` / `DELETE`.
        pub action: String,
        /// Acting user id.
        pub user_id: Option<String>,
        /// Originating IP address.
        pub user_ip_address: Option<String>,
        /// Originating user-agent.
        pub user_agent: Option<String>,
        /// Pre-change snapshot (JSONB).
        pub old_values: Option<Json>,
        /// Post-change snapshot (JSONB).
        pub new_values: Option<Json>,
        /// When the action occurred.
        pub created_at: DateTimeUtc,
    }

    /// No outbound relations from `audit_log`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────── course_match_scores ───────────────────

/// Entity for the `course_match_scores` table (review-queue rows).
pub mod course_match_scores {
    use super::*;

    /// A row in `course_match_scores`; backs
    /// [`ReviewQueueItem`](crate::models::review_queue::ReviewQueueItem).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_match_scores")]
    pub struct Model {
        /// Score-row UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// First course in the candidate pair.
        pub course_id: Uuid,
        /// Second (candidate) course in the pair.
        pub candidate_id: Uuid,
        /// Overall match score.
        pub match_score: f64,
        /// Confidence band label.
        pub match_quality: String,
        /// How the pair was detected.
        pub detection_method: String,
        /// Per-component breakdown (JSONB).
        pub score_breakdown: Option<Json>,
        /// Review status (bare string).
        pub status: String,
        /// Reviewer id, once reviewed.
        pub reviewed_by: Option<String>,
        /// Row creation timestamp.
        pub created_at: DateTimeUtc,
        /// Review timestamp, if reviewed.
        pub reviewed_at: Option<DateTimeUtc>,
    }

    /// No outbound relations from `course_match_scores`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────── course_merge_records ───────────────────

/// Entity for the `course_merge_records` table (merge audit rows).
pub mod course_merge_records {
    use super::*;

    /// A row in `course_merge_records`; maps to
    /// [`MergeRecord`](crate::models::merge::MergeRecord).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_merge_records")]
    pub struct Model {
        /// Merge-record UUID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Surviving course id.
        pub main_course_id: Uuid,
        /// Folded-in (soft-deleted) course id.
        pub duplicate_course_id: Uuid,
        /// Merge lifecycle status (bare string).
        pub status: String,
        /// Actor that performed the merge.
        pub merged_by: Option<String>,
        /// Free-text merge reason.
        pub merge_reason: Option<String>,
        /// Motivating match score.
        pub match_score: Option<f64>,
        /// Snapshot of transferred data (JSONB).
        pub transferred_data: Option<Json>,
        /// When the merge occurred.
        pub merged_at: DateTimeUtc,
    }

    /// No outbound relations from `course_merge_records`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
