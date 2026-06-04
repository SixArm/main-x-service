//! SeaORM entity modules.
//!
//! One module per table from `migrations/`. JSONB columns are typed as
//! `serde_json::Value` and rehydrated to typed collections by the
//! repository.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ───────────────────────── providers ─────────────────────────

pub mod providers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "providers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub name: String,
        pub alternate_names: Json,
        pub url: Option<String>,
        pub same_as: Json,
        pub kind: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub deleted_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::courses::Entity")]
        Courses,
    }

    impl Related<super::courses::Entity> for Entity {
        fn to() -> RelationDef { Relation::Courses.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────────── courses ─────────────────────────

pub mod courses {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "courses")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub name: String,
        pub alternate_names: Json,
        pub description: Option<String>,
        pub disambiguating_description: Option<String>,
        pub url: Option<String>,
        pub image: Json,
        pub same_as: Json,
        pub keywords: Json,
        pub additional_type: Option<String>,
        pub about: Json,
        pub audience: Option<String>,
        pub in_language: Json,
        pub license: Option<String>,
        pub typical_age_range: Option<String>,
        pub time_required: Option<String>,
        pub version: Option<String>,
        pub is_accessible_for_free: Option<bool>,
        pub teaches: Json,
        pub assesses: Json,
        pub competency_required: Json,
        pub educational_level: Option<Json>,
        pub educational_use: Option<String>,
        pub learning_resource_type: Option<Json>,
        pub interactivity_type: Option<String>,
        pub course_code: Option<String>,
        pub number_of_credits: Option<i32>,
        pub course_prerequisites: Json,
        pub available_language: Json,
        pub financial_aid_eligible: Json,
        pub educational_credential_awarded: Option<Json>,
        pub occupational_credential_awarded: Option<Json>,
        pub total_historical_enrollment: Option<i64>,
        pub status: String,
        pub active: bool,
        pub provider_id: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub deleted_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::course_identifiers::Entity")]
        CourseIdentifiers,
        #[sea_orm(has_many = "super::course_instances::Entity")]
        CourseInstances,
        #[sea_orm(has_many = "super::syllabus_sections::Entity")]
        SyllabusSections,
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

pub mod course_identifiers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_identifiers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub course_id: Uuid,
        pub property_id: Json,
        pub value: String,
        pub name: Option<String>,
        pub url: Option<String>,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod course_links {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_links")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub course_id: Uuid,
        pub other_course_id: Uuid,
        pub link_type: String,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod course_instances {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_instances")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub course_id: Uuid,
        pub name: Option<String>,
        pub course_mode: Option<String>,
        pub status: String,
        pub in_language: Json,
        pub location: Option<String>,
        pub location_id: Option<Uuid>,
        pub instructor_ids: Json,
        pub instructor_names: Json,
        pub maximum_attendee_capacity: Option<i32>,
        pub enrolled_count: Option<i32>,
        pub enrollment_opens: Option<DateTimeUtc>,
        pub enrollment_closes: Option<DateTimeUtc>,
        pub schedule: Option<Json>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub deleted_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod syllabus_sections {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "syllabus_sections")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub course_id: Uuid,
        pub parent_id: Option<Uuid>,
        pub name: String,
        pub description: Option<String>,
        pub position: Option<i32>,
        pub teaches: Json,
        pub time_required: Option<String>,
        pub resources: Json,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod audit_log {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub entity_type: String,
        pub entity_id: Uuid,
        pub action: String,
        pub user_id: Option<String>,
        pub user_ip_address: Option<String>,
        pub user_agent: Option<String>,
        pub old_values: Option<Json>,
        pub new_values: Option<Json>,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────── course_match_scores ───────────────────

pub mod course_match_scores {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_match_scores")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub course_id: Uuid,
        pub candidate_id: Uuid,
        pub match_score: f64,
        pub match_quality: String,
        pub detection_method: String,
        pub score_breakdown: Option<Json>,
        pub status: String,
        pub reviewed_by: Option<String>,
        pub created_at: DateTimeUtc,
        pub reviewed_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ─────────────────── course_merge_records ───────────────────

pub mod course_merge_records {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "course_merge_records")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub main_course_id: Uuid,
        pub duplicate_course_id: Uuid,
        pub status: String,
        pub merged_by: Option<String>,
        pub merge_reason: Option<String>,
        pub match_score: Option<f64>,
        pub transferred_data: Option<Json>,
        pub merged_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
