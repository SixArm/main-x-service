//! Database access — connection pool + repository.
//!
//! `SeaOrmCourseRepository` round-trips a domain [`Course`] against the
//! relational schema: scalar fields and JSONB collections live on the
//! `courses` row; `identifiers` and `links` live in their own child
//! tables. Sub-resources (`instances`, `syllabus_sections`) have their
//! own dedicated API surface and are intentionally not loaded here —
//! they ship with T-8.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Result;
use crate::config::DatabaseConfig;
use crate::models::{
    Course, CourseIdentifier, CourseLink, CourseStatus, EducationalLevel, IdentifierType,
    InteractivityType, LearningResourceType, LinkType,
};

pub mod models;

use models::{course_identifiers, course_links, courses};

/// Open a connection pool from a `DatabaseConfig`.
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);
    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}

/// Repository for Course CRUD.
///
/// `dyn`-compatibility provided by `async_trait` (object-safety
/// blocks plain `async fn` in trait methods until we move to RPITIT).
#[async_trait::async_trait]
pub trait CourseRepository: Send + Sync {
    async fn create(&self, course: &Course) -> Result<Course>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Course>>;
    async fn update(&self, course: &Course) -> Result<Course>;
    async fn soft_delete(&self, id: &Uuid) -> Result<()>;
    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Course>>;
}

pub struct SeaOrmCourseRepository {
    db: DatabaseConnection,
}

impl SeaOrmCourseRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl CourseRepository for SeaOrmCourseRepository {
    async fn create(&self, course: &Course) -> Result<Course> {
        let txn = self.db.begin().await.map_err(map_db)?;
        let active = to_course_active(course, false)?;
        active.insert(&txn).await.map_err(map_db)?;
        insert_identifiers(&txn, course.id, &course.identifiers).await?;
        insert_links(&txn, course.id, &course.links).await?;
        txn.commit().await.map_err(map_db)?;
        self.get_by_id(&course.id)
            .await?
            .ok_or_else(|| crate::Error::Database("course not found after insert".into()))
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Course>> {
        let row = courses::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(map_db)?;
        let Some(row) = row else { return Ok(None) };
        if row.deleted_at.is_some() {
            return Ok(None);
        }
        let identifiers = load_identifiers(&self.db, *id).await?;
        let links = load_links(&self.db, *id).await?;
        Ok(Some(hydrate_course(row, identifiers, links)?))
    }

    async fn update(&self, course: &Course) -> Result<Course> {
        let exists = courses::Entity::find_by_id(course.id)
            .one(&self.db)
            .await
            .map_err(map_db)?;
        if exists.is_none() {
            return Err(crate::Error::NotFound);
        }
        let txn = self.db.begin().await.map_err(map_db)?;
        let active = to_course_active(course, true)?;
        active.update(&txn).await.map_err(map_db)?;
        course_identifiers::Entity::delete_many()
            .filter(course_identifiers::Column::CourseId.eq(course.id))
            .exec(&txn)
            .await
            .map_err(map_db)?;
        course_links::Entity::delete_many()
            .filter(course_links::Column::CourseId.eq(course.id))
            .exec(&txn)
            .await
            .map_err(map_db)?;
        insert_identifiers(&txn, course.id, &course.identifiers).await?;
        insert_links(&txn, course.id, &course.links).await?;
        txn.commit().await.map_err(map_db)?;
        self.get_by_id(&course.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete(&self, id: &Uuid) -> Result<()> {
        let row = courses::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(map_db)?
            .ok_or(crate::Error::NotFound)?;
        let mut active: courses::ActiveModel = row.into();
        active.active = Set(false);
        active.deleted_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        active.update(&self.db).await.map_err(map_db)?;
        Ok(())
    }

    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Course>> {
        let rows = courses::Entity::find()
            .filter(courses::Column::DeletedAt.is_null())
            .order_by_desc(courses::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await
            .map_err(map_db)?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let identifiers = load_identifiers(&self.db, id).await?;
            let links = load_links(&self.db, id).await?;
            out.push(hydrate_course(row, identifiers, links)?);
        }
        Ok(out)
    }
}

// ────────────────── Domain ↔ DB conversion ──────────────────

fn to_course_active(course: &Course, is_update: bool) -> Result<courses::ActiveModel> {
    let now = Utc::now();
    Ok(courses::ActiveModel {
        id: Set(course.id),
        name: Set(course.name.clone()),
        alternate_names: Set(to_json(&course.alternate_names)?),
        description: Set(course.description.clone()),
        disambiguating_description: Set(course.disambiguating_description.clone()),
        url: Set(course.url.clone()),
        image: Set(to_json(&course.image)?),
        same_as: Set(to_json(&course.same_as)?),
        keywords: Set(to_json(&course.keywords)?),
        additional_type: Set(course.additional_type.clone()),
        about: Set(to_json(&course.about)?),
        audience: Set(course.audience.clone()),
        in_language: Set(to_json(&course.in_language)?),
        license: Set(course.license.clone()),
        typical_age_range: Set(course.typical_age_range.clone()),
        time_required: Set(course.time_required.clone()),
        version: Set(course.version.clone()),
        is_accessible_for_free: Set(course.is_accessible_for_free),
        teaches: Set(to_json(&course.teaches)?),
        assesses: Set(to_json(&course.assesses)?),
        competency_required: Set(to_json(&course.competency_required)?),
        educational_level: Set(course
            .educational_level
            .as_ref()
            .map(to_json)
            .transpose()?),
        educational_use: Set(course.educational_use.clone()),
        learning_resource_type: Set(course
            .learning_resource_type
            .as_ref()
            .map(to_json)
            .transpose()?),
        interactivity_type: Set(course
            .interactivity_type
            .as_ref()
            .map(enum_to_string)
            .transpose()?),
        course_code: Set(course.course_code.clone()),
        number_of_credits: Set(course.number_of_credits.map(|v| v as i32)),
        course_prerequisites: Set(to_json(&course.course_prerequisites)?),
        available_language: Set(to_json(&course.available_language)?),
        financial_aid_eligible: Set(to_json(&course.financial_aid_eligible)?),
        educational_credential_awarded: Set(course
            .educational_credential_awarded
            .as_ref()
            .map(to_json)
            .transpose()?),
        occupational_credential_awarded: Set(course
            .occupational_credential_awarded
            .as_ref()
            .map(to_json)
            .transpose()?),
        total_historical_enrollment: Set(course.total_historical_enrollment.map(|v| v as i64)),
        status: Set(enum_to_string(&course.status)?),
        active: Set(course.active),
        provider_id: Set(course.provider_id),
        created_at: Set(course.created_at),
        updated_at: Set(if is_update { now } else { course.updated_at }),
        deleted_at: Set(course.deleted_at),
    })
}

fn hydrate_course(
    row: courses::Model,
    identifiers: Vec<CourseIdentifier>,
    links: Vec<CourseLink>,
) -> Result<Course> {
    Ok(Course {
        id: row.id,
        name: row.name,
        alternate_names: from_json(row.alternate_names)?,
        description: row.description,
        disambiguating_description: row.disambiguating_description,
        url: row.url,
        image: from_json(row.image)?,
        same_as: from_json(row.same_as)?,
        keywords: from_json(row.keywords)?,
        identifiers,
        additional_type: row.additional_type,
        active: row.active,
        about: from_json(row.about)?,
        audience: row.audience,
        in_language: from_json(row.in_language)?,
        license: row.license,
        typical_age_range: row.typical_age_range,
        time_required: row.time_required,
        version: row.version,
        is_accessible_for_free: row.is_accessible_for_free,
        teaches: from_json(row.teaches)?,
        assesses: from_json(row.assesses)?,
        competency_required: from_json(row.competency_required)?,
        educational_level: row.educational_level.map(from_json::<EducationalLevel>).transpose()?,
        educational_use: row.educational_use,
        learning_resource_type: row
            .learning_resource_type
            .map(from_json::<LearningResourceType>)
            .transpose()?,
        interactivity_type: row
            .interactivity_type
            .as_deref()
            .map(enum_from_string::<InteractivityType>)
            .transpose()?,
        course_code: row.course_code,
        number_of_credits: row.number_of_credits.map(|v| v as u32),
        course_prerequisites: from_json(row.course_prerequisites)?,
        available_language: from_json(row.available_language)?,
        financial_aid_eligible: from_json(row.financial_aid_eligible)?,
        educational_credential_awarded: row
            .educational_credential_awarded
            .map(from_json)
            .transpose()?,
        occupational_credential_awarded: row
            .occupational_credential_awarded
            .map(from_json)
            .transpose()?,
        total_historical_enrollment: row.total_historical_enrollment.map(|v| v as u64),
        syllabus_sections: vec![],
        instances: vec![],
        status: enum_from_string::<CourseStatus>(&row.status)?,
        links,
        provider_id: row.provider_id,
        deleted_at: row.deleted_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

// ────────────────── Child-table round-trip ──────────────────

async fn insert_identifiers<C>(
    conn: &C,
    course_id: Uuid,
    identifiers: &[CourseIdentifier],
) -> Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    for ident in identifiers {
        let row = course_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            course_id: Set(course_id),
            property_id: Set(to_json(&ident.property_id)?),
            value: Set(ident.value.clone()),
            name: Set(ident.name.clone()),
            url: Set(ident.url.clone()),
            created_at: Set(Utc::now()),
        };
        row.insert(conn).await.map_err(map_db)?;
    }
    Ok(())
}

async fn insert_links<C>(conn: &C, course_id: Uuid, links: &[CourseLink]) -> Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    for link in links {
        let row = course_links::ActiveModel {
            id: Set(Uuid::new_v4()),
            course_id: Set(course_id),
            other_course_id: Set(link.other_course_id),
            link_type: Set(enum_to_string(&link.link_type)?),
            created_at: Set(Utc::now()),
        };
        row.insert(conn).await.map_err(map_db)?;
    }
    Ok(())
}

async fn load_identifiers(
    db: &DatabaseConnection,
    course_id: Uuid,
) -> Result<Vec<CourseIdentifier>> {
    let rows = course_identifiers::Entity::find()
        .filter(course_identifiers::Column::CourseId.eq(course_id))
        .all(db)
        .await
        .map_err(map_db)?;
    rows.into_iter()
        .map(|r| {
            Ok(CourseIdentifier {
                property_id: from_json::<IdentifierType>(r.property_id)?,
                value: r.value,
                name: r.name,
                url: r.url,
            })
        })
        .collect()
}

async fn load_links(db: &DatabaseConnection, course_id: Uuid) -> Result<Vec<CourseLink>> {
    let rows = course_links::Entity::find()
        .filter(course_links::Column::CourseId.eq(course_id))
        .all(db)
        .await
        .map_err(map_db)?;
    rows.into_iter()
        .map(|r| {
            Ok(CourseLink {
                other_course_id: r.other_course_id,
                link_type: enum_from_string::<LinkType>(&r.link_type)?,
            })
        })
        .collect()
}

// ────────────────── Helpers ──────────────────

fn map_db(e: sea_orm::DbErr) -> crate::Error {
    crate::Error::Database(e.to_string())
}

fn to_json<T: Serialize>(v: &T) -> Result<serde_json::Value> {
    serde_json::to_value(v).map_err(|e| crate::Error::Database(e.to_string()))
}

fn from_json<T: for<'de> Deserialize<'de>>(j: serde_json::Value) -> Result<T> {
    serde_json::from_value(j).map_err(|e| crate::Error::Database(e.to_string()))
}

fn enum_to_string<T: Serialize>(v: &T) -> Result<String> {
    let json = serde_json::to_value(v).map_err(|e| crate::Error::Database(e.to_string()))?;
    json.as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| crate::Error::Database("enum did not serialise to a string".into()))
}

fn enum_from_string<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| crate::Error::Database(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CourseStatus;

    #[test]
    fn course_status_round_trips_through_string() {
        for status in [
            CourseStatus::Draft,
            CourseStatus::Published,
            CourseStatus::Archived,
            CourseStatus::Retired,
        ] {
            let s = enum_to_string(&status).unwrap();
            let back: CourseStatus = enum_from_string(&s).unwrap();
            assert_eq!(status, back, "{s} did not round-trip");
        }
    }

    #[test]
    fn link_type_round_trips_through_string() {
        for lt in [
            LinkType::Replaces,
            LinkType::ReplacedBy,
            LinkType::Seealso,
            LinkType::Prerequisite,
            LinkType::Successor,
        ] {
            let s = enum_to_string(&lt).unwrap();
            let back: LinkType = enum_from_string(&s).unwrap();
            assert_eq!(lt, back, "{s} did not round-trip");
        }
    }

    #[test]
    fn course_active_model_carries_all_scalar_fields() {
        let mut course = Course::new("Intro to CS");
        course.course_code = Some("CS101".into());
        course.number_of_credits = Some(3);
        course.educational_level = Some(EducationalLevel::Undergraduate);
        course.keywords = vec!["programming".into(), "algorithms".into()];
        let active = to_course_active(&course, false).unwrap();
        assert!(matches!(active.name, Set(ref n) if n == "Intro to CS"));
        assert!(matches!(active.course_code, Set(Some(ref c)) if c == "CS101"));
        assert!(matches!(active.number_of_credits, Set(Some(3))));
        assert!(matches!(active.status, Set(ref s) if s == "published"));
    }
}
