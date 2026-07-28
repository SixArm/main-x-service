//! `SeaORM` entity modules.
//!
//! One module per table from `migrations/`. JSONB columns are typed as
//! `serde_json::Value` and rehydrated to typed collections by the
//! repository.

// ───────────────────────── providers ─────────────────────────

/// Entity for the `providers` table (course-authoring organisations).
pub mod providers {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        /// Website URL.
        pub url: Option<String>,
        /// Provider kind, stored as a string.
        pub kind: Option<String>,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// Soft-delete timestamp, if deleted.
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// Foreign-key relations from `providers`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// One provider authors many courses.
        #[sea_orm(has_many = "super::courses::Entity")]
        Courses,
    }

    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Courses.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── courses ─────────────────────────

/// Entity for the `courses` table — the schema.org/Course template row.
pub mod courses {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        /// SHA-256 (FIPS 180-4) over the assembled record's pre-image.
        ///
        /// `None` on a row written before the column existed — reported
        /// as unhashed, never as a mismatch, and never back-filled.
        pub content_hash: Option<String>,
        /// SHA3-256 (FIPS 202) over the same pre-image.
        pub content_hash_sha3: Option<String>,
        /// HMAC-SHA256 over the same pre-image — the only one of the
        /// three an adversary holding just this database cannot forge.
        pub content_mac: Option<String>,
        /// Long description.
        pub description: Option<String>,
        /// Disambiguating one-liner.
        pub disambiguating_description: Option<String>,
        /// Canonical course URL.
        pub url: Option<String>,
        /// schema.org/additionalType.
        pub additional_type: Option<String>,
        /// Target audience.
        pub audience: Option<String>,
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
        /// Educational level enum (bare string).
        pub educational_level: Option<String>,
        /// schema.org/educationalUse.
        pub educational_use: Option<String>,
        /// Learning-resource-type enum (bare string).
        pub learning_resource_type: Option<String>,
        /// Interactivity-type enum (bare string).
        pub interactivity_type: Option<String>,
        /// Provider catalog code.
        pub course_code: Option<String>,
        /// Credit count (stored as `i32`).
        pub number_of_credits: Option<i32>,
        /// Historical enrollment count (stored as `i64`).
        pub total_historical_enrollment: Option<i64>,
        /// Lifecycle status (bare string).
        pub status: String,
        /// Active flag.
        pub active: bool,
        /// FK to the owning provider.
        pub provider_id: Option<Uuid>,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// Soft-delete timestamp, if deleted.
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
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
        fn to() -> RelationDef {
            Relation::CourseIdentifiers.def()
        }
    }
    impl Related<super::course_instances::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::CourseInstances.def()
        }
    }
    impl Related<super::syllabus_sections::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::SyllabusSections.def()
        }
    }
    impl Related<super::providers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Provider.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_identifiers ─────────────────────

/// Entity for the `course_identifiers` table (external identifiers).
pub mod course_identifiers {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        /// Identifier scheme tag (or `Custom`).
        pub property_id: String,
        /// Free-text label for the `Custom` scheme.
        pub custom_label: Option<String>,
        /// Scheme-specific value.
        pub value: String,
        /// Optional label.
        pub name: Option<String>,
        /// Optional authority URL.
        pub url: Option<String>,
        /// Ordinal position within the list.
        pub position: i32,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Course.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── course_links ─────────────────────────

/// Entity for the `course_links` table (typed course-to-course links).
pub mod course_links {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        pub created_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Course.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────────── course_instances ───────────────────────

/// Entity for the `course_instances` table (specific offerings).
pub mod course_instances {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        /// Free-text location.
        pub location: Option<String>,
        /// External place-service location reference.
        pub location_id: Option<Uuid>,
        /// Maximum capacity (stored as `i32`).
        pub maximum_attendee_capacity: Option<i32>,
        /// Current enrollment (stored as `i32`).
        pub enrolled_count: Option<i32>,
        /// Enrollment window open time.
        pub enrollment_opens: Option<TimeDateTimeWithTimeZone>,
        /// Enrollment window close time.
        pub enrollment_closes: Option<TimeDateTimeWithTimeZone>,
        /// Schedule start (flattened from the Schedule struct).
        pub schedule_start_date: Option<TimeDateTimeWithTimeZone>,
        /// Schedule end (flattened).
        pub schedule_end_date: Option<TimeDateTimeWithTimeZone>,
        /// Schedule time zone (flattened).
        pub schedule_time_zone: Option<String>,
        /// Schedule recurrence rule (flattened).
        pub schedule_recurrence: Option<String>,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// Soft-delete timestamp, if deleted.
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
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
        fn to() -> RelationDef {
            Relation::Course.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────────── syllabus_sections ───────────────────────

/// Entity for the `syllabus_sections` table (course-outline tree).
pub mod syllabus_sections {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        /// ISO 8601 duration.
        pub time_required: Option<String>,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Course.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── audit_log ─────────────────────────

/// Entity for the `audit_log` table (HIPAA-style change trail).
pub mod audit_log {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// SHA-256 (FIPS 180-4) over this audit row's pre-image.
        ///
        /// Unkeyed, so anyone holding the database can recompute it — what it
        /// catches is careless or unaware modification. Written
        /// unconditionally, unlike the MAC, which needs a key: with no key
        /// configured these two digests are the row's only integrity.
        pub hash: Option<String>,
        /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
        /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
        /// against one design family does not transfer.
        pub hash_sha3: Option<String>,
        /// HMAC-SHA256 over this audit row's pre-image.
        ///
        /// Detects a row whose content was altered. It does **not**
        /// detect a row deleted wholesale — that needs the hash chain
        /// this service does not yet have (see `crate::compliance`).
        pub mac: Option<String>,
    }

    /// No outbound relations from `audit_log`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────── course_match_scores ───────────────────

/// Entity for the `course_match_scores` table (review-queue rows).
pub mod course_match_scores {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Review timestamp, if reviewed.
        pub reviewed_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// No outbound relations from `course_match_scores`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────── course_merge_records ───────────────────

/// Entity for the `course_merge_records` table (merge audit rows).
pub mod course_merge_records {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

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
        pub merged_at: TimeDateTimeWithTimeZone,
    }

    /// No outbound relations from `course_merge_records`.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_text_values ─────────────────────

/// Tagged table for the Course aggregate's parallel string-list properties.
pub mod course_text_values {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// One `(field, value)` row for a course string list.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_text_values")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning course.
        pub course_id: Uuid,
        /// Which list this value belongs to.
        pub field: String,
        /// The value.
        pub value: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent course.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `courses`.
        #[sea_orm(
            belongs_to = "super::courses::Entity",
            from = "Column::CourseId",
            to = "super::courses::Column::Id"
        )]
        Course,
    }
    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Course.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_credentials ─────────────────────

/// Educational / occupational credential awarded by a course.
pub mod course_credentials {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// One credential row (`role` = educational|occupational).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_credentials")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning course.
        pub course_id: Uuid,
        /// `educational` or `occupational`.
        pub role: String,
        /// Credential name.
        pub name: String,
        /// Credential category enum (bare string), if any.
        pub category: Option<String>,
        /// Educational level text, if any.
        pub educational_level: Option<String>,
        /// Recognizing body, if any.
        pub recognized_by: Option<String>,
        /// Credential URL, if any.
        pub url: Option<String>,
    }

    /// `belongs_to` the parent course.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `courses`.
        #[sea_orm(
            belongs_to = "super::courses::Entity",
            from = "Column::CourseId",
            to = "super::courses::Column::Id"
        )]
        Course,
    }
    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Course.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_instance_languages ─────────────────────

/// Languages of instruction for a course instance.
pub mod course_instance_languages {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// One language row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_instance_languages")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning instance.
        pub instance_id: Uuid,
        /// ISO 639-1 language code.
        pub language: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent instance.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `course_instances`.
        #[sea_orm(
            belongs_to = "super::course_instances::Entity",
            from = "Column::InstanceId",
            to = "super::course_instances::Column::Id"
        )]
        Instance,
    }
    impl Related<super::course_instances::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Instance.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_instance_instructors ─────────────────────

/// Instructors (id and/or name) for a course instance.
pub mod course_instance_instructors {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// One instructor row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_instance_instructors")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning instance.
        pub instance_id: Uuid,
        /// Person-service instructor id, if any.
        pub instructor_id: Option<Uuid>,
        /// Free-text instructor name, if any.
        pub instructor_name: Option<String>,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent instance.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `course_instances`.
        #[sea_orm(
            belongs_to = "super::course_instances::Entity",
            from = "Column::InstanceId",
            to = "super::course_instances::Column::Id"
        )]
        Instance,
    }
    impl Related<super::course_instances::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Instance.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_instance_sessions ─────────────────────

/// Individual scheduled sessions for a course instance.
pub mod course_instance_sessions {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// One session row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_instance_sessions")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning instance.
        pub instance_id: Uuid,
        /// Session start.
        pub start_at: TimeDateTimeWithTimeZone,
        /// Session end, if any.
        pub end_at: Option<TimeDateTimeWithTimeZone>,
        /// Session label, if any.
        pub label: Option<String>,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent instance.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `course_instances`.
        #[sea_orm(
            belongs_to = "super::course_instances::Entity",
            from = "Column::InstanceId",
            to = "super::course_instances::Column::Id"
        )]
        Instance,
    }
    impl Related<super::course_instances::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Instance.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── course_syllabus_text_values ─────────────────────

/// Tagged table for syllabus-section `teaches` / `resource` lists.
pub mod course_syllabus_text_values {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// One `(field, value)` row for a syllabus section.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_syllabus_text_values")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// FK to the owning syllabus section.
        pub section_id: Uuid,
        /// `teaches` or `resource`.
        pub field: String,
        /// The value.
        pub value: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent syllabus section.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `syllabus_sections`.
        #[sea_orm(
            belongs_to = "super::syllabus_sections::Entity",
            from = "Column::SectionId",
            to = "super::syllabus_sections::Column::Id"
        )]
        Section,
    }
    impl Related<super::syllabus_sections::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Section.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── course_outbox ─────────────────────────

/// `SeaORM` entity for the `course_outbox` table — the transactional-outbox
/// hand-off buffer for the durable event bus (Phase 2; see
/// `agents/share/event-bus.md` §3). One row is written inside the same
/// transaction as each Course mutation; a Phase-3 relay worker (roadmap)
/// drains unpublished rows to Fluvio and stamps `published_at`.
pub mod course_outbox {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// One persisted outbox row: a canonical envelope awaiting relay to
    /// the durable bus.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_outbox")]
    pub struct Model {
        /// Auto-increment pk; also the global relay order (`ORDER BY id`).
        #[sea_orm(primary_key)]
        pub id: i64,
        /// Envelope id — the consumer dedup key.
        #[sea_orm(unique)]
        pub event_id: Uuid,
        /// The entity name (`course`).
        pub entity: String,
        /// The record pid — the bus partition key.
        pub entity_pid: Uuid,
        /// The change kind: `created` / `updated` / `deleted` / `merged`.
        pub kind: String,
        /// When the change occurred (stamped at enqueue).
        pub occurred_at: TimeDateTimeWithTimeZone,
        /// The acting user pid, or `None` when unauthenticated.
        pub actor: Option<String>,
        /// The envelope schema version.
        pub schema_version: i32,
        /// The full canonical envelope as JSONB.
        pub payload: Json,
        /// `None` until the relay ships the row to the bus.
        pub published_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// `SeaORM` relations for the outbox entity (none defined).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
