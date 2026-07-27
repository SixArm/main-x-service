//! Database access — connection pool + repository.
//!
//! `SeaOrmCourseRepository` round-trips a domain [`Course`] against the
//! relational schema: scalar fields and JSONB collections live on the
//! `courses` row; `identifiers` and `links` live in their own child
//! tables. Sub-resources (`instances`, `syllabus_sections`) have their
//! own dedicated API surface and are intentionally not loaded here —
//! they ship with T-8.

mod convert;
use convert::{offset_to_ts, ts_to_offset};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::config::DatabaseConfig;
use crate::db::outbox::OutboxInsert;
use crate::models::{
    Course, CourseIdentifier, CourseInstance, CourseInstanceStatus, CourseLink, CourseMode,
    CourseStatus, EducationalLevel, IdentifierType, InteractivityType, LearningResourceType,
    LinkType, MergeRecord, Schedule,
};
use crate::streaming::EventTransport;
use crate::streaming::envelope::EventKind;

pub mod audit;
pub mod models;
pub mod outbox;

use models::{
    course_credentials, course_identifiers, course_instance_instructors, course_instance_languages,
    course_instance_sessions, course_instances, course_links, course_merge_records,
    course_text_values, courses,
};

/// Open a connection pool from a `DatabaseConfig`.
///
/// # Errors
///
/// Returns [`enum@crate::Error`] if the pool cannot connect to the
/// configured database URL.
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
    /// Insert a new course (plus its identifier/link child rows).
    async fn create(&self, course: &Course) -> Result<Course>;
    /// Fetch a course by id, returning `None` if absent or soft-deleted.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Course>>;
    /// Update an existing course, replacing its child rows.
    async fn update(&self, course: &Course) -> Result<Course>;
    /// Soft-delete a course (sets `deleted_at`, clears `active`).
    async fn soft_delete(&self, id: &Uuid) -> Result<()>;
    /// List non-deleted courses, newest first, with limit/offset paging.
    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Course>>;

    // CourseInstance sub-resource (T-8, FR-10..FR-13).
    /// List a course's non-deleted instances (FR-10 ordering applied).
    async fn list_instances(&self, course_id: &Uuid) -> Result<Vec<CourseInstance>>;
    /// Fetch one instance scoped to its parent course.
    async fn get_instance(
        &self,
        course_id: &Uuid,
        instance_id: &Uuid,
    ) -> Result<Option<CourseInstance>>;
    /// Insert a new instance under its parent course.
    async fn create_instance(&self, instance: &CourseInstance) -> Result<CourseInstance>;
    /// Update an existing instance.
    async fn update_instance(&self, instance: &CourseInstance) -> Result<CourseInstance>;
    /// Soft-delete one instance scoped to its parent course.
    async fn soft_delete_instance(&self, course_id: &Uuid, instance_id: &Uuid) -> Result<()>;

    /// Merge bookkeeping (T-7b / FR-8). Inserts a row into
    /// `course_merge_records` describing the fold of `duplicate` into
    /// `main`. The handler is responsible for the actual data
    /// transfer + soft-delete of the duplicate.
    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord>;

    /// Apply the merged `survivor`'s parent + child row updates and
    /// soft-delete the `duplicate_id` record **in one transaction**, so
    /// the whole merge commits (or rolls back) atomically. Under the
    /// outbox transport this enqueues a `Merged` outbox row for the
    /// survivor (carrying the duplicate's pid via `merged_from`) and a
    /// `Deleted` outbox row for the duplicate on the same transaction.
    /// Returns the reloaded survivor.
    async fn merge(&self, survivor: &Course, duplicate_id: &Uuid) -> Result<Course>;
}

/// SeaORM-backed [`CourseRepository`] implementation over a `PostgreSQL`
/// connection pool.
pub struct SeaOrmCourseRepository {
    /// The shared `SeaORM` connection pool.
    db: DatabaseConnection,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps only the post-commit
    /// in-memory publish (done by the handlers); [`EventTransport::Outbox`]
    /// additionally writes one `course_outbox` row **inside** each write's
    /// transaction.
    transport: EventTransport,
}

impl SeaOrmCourseRepository {
    /// Wrap an existing connection pool in a repository, with the default
    /// [`EventTransport::Memory`] transport (behaviour unchanged from
    /// before the outbox landed).
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            transport: EventTransport::Memory,
        }
    }

    /// Builder: select the event transport (see [`EventTransport`]).
    /// `AppState` wires this from `COURSE_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `course_outbox`
    /// row for `kind` applied to `course` **on `conn`** — pass the open
    /// `&DatabaseTransaction` so the row commits with the entity write
    /// (the outbox atomicity guarantee). A no-op under
    /// [`EventTransport::Memory`].
    ///
    /// The `ConnectionTrait` generic lives here on a concrete method (not
    /// on the object-safe [`CourseRepository`] trait), which is how a
    /// `dyn`-trait repository threads the outbox insert into its own
    /// transaction.
    async fn enqueue_outbox<C: ConnectionTrait>(
        &self,
        conn: &C,
        course: &Course,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(course, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl CourseRepository for SeaOrmCourseRepository {
    async fn create(&self, course: &Course) -> Result<Course> {
        // One transaction so the parent row and every child collection
        // (identifiers, links, text-values, credentials) commit atomically.
        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
        let active = to_course_active(course, false)?;
        active.insert(&txn).await.map_err(|e| map_db(&e))?;
        insert_identifiers(&txn, course.id, &course.identifiers).await?;
        insert_links(&txn, course.id, &course.links).await?;
        insert_course_collections(&txn, course).await?;
        // Durable event bus (Phase 2): under the outbox transport, write
        // the `course_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the post-commit in-memory publish in the handler.
        self.enqueue_outbox(&txn, course, EventKind::Created)
            .await?;
        txn.commit().await.map_err(|e| map_db(&e))?;
        // Re-read through the normal hydrate path so the returned value
        // reflects exactly what was persisted (defaults, ordering, etc).
        self.get_by_id(&course.id)
            .await?
            .ok_or_else(|| crate::Error::Database("course not found after insert".into()))
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Course>> {
        let row = courses::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let Some(row) = row else { return Ok(None) };
        // Soft-deleted rows read as absent — callers never see them.
        if row.deleted_at.is_some() {
            return Ok(None);
        }
        let identifiers = load_identifiers(&self.db, *id).await?;
        let links = load_links(&self.db, *id).await?;
        let (text_values, credentials) = load_course_collections(&self.db, *id).await?;
        Ok(Some(hydrate_course(
            row,
            identifiers,
            links,
            &text_values,
            &credentials,
        )?))
    }

    async fn update(&self, course: &Course) -> Result<Course> {
        let exists = courses::Entity::find_by_id(course.id)
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        if exists.is_none() {
            return Err(crate::Error::NotFound);
        }
        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
        apply_course_update(&txn, course).await?;
        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&txn, course, EventKind::Updated)
            .await?;
        txn.commit().await.map_err(|e| map_db(&e))?;
        self.get_by_id(&course.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete(&self, id: &Uuid) -> Result<()> {
        let row = courses::Entity::find_by_id(*id)
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?
            .ok_or(crate::Error::NotFound)?;
        // Soft delete: flip `active` off and stamp `deleted_at` rather
        // than removing the row, preserving it for the audit trail.
        let mut active: courses::ActiveModel = row.into();
        active.active = Set(false);
        active.deleted_at = Set(Some(OffsetDateTime::now_utc()));
        active.updated_at = Set(OffsetDateTime::now_utc());
        // Under the outbox transport, open a transaction so the tombstone
        // write and the `deleted` outbox row commit atomically; the memory
        // transport keeps the plain, tx-free update. The domain course is
        // reloaded (still visible pre-delete) to build the deleted envelope.
        if self.transport.is_outbox() {
            let old = self.get_by_id(id).await?;
            let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
            active.update(&txn).await.map_err(|e| map_db(&e))?;
            if let Some(old) = old.as_ref() {
                self.enqueue_outbox(&txn, old, EventKind::Deleted).await?;
            }
            txn.commit().await.map_err(|e| map_db(&e))?;
        } else {
            active.update(&self.db).await.map_err(|e| map_db(&e))?;
        }
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
            .map_err(|e| map_db(&e))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let identifiers = load_identifiers(&self.db, id).await?;
            let links = load_links(&self.db, id).await?;
            let (text_values, credentials) = load_course_collections(&self.db, id).await?;
            out.push(hydrate_course(
                row,
                identifiers,
                links,
                &text_values,
                &credentials,
            )?);
        }
        Ok(out)
    }

    // ──────────── CourseInstance sub-resource ────────────

    async fn list_instances(&self, course_id: &Uuid) -> Result<Vec<CourseInstance>> {
        let rows = course_instances::Entity::find()
            .filter(course_instances::Column::CourseId.eq(*course_id))
            .filter(course_instances::Column::DeletedAt.is_null())
            .all(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(hydrate_instance(&self.db, row).await?);
        }
        // FR-10 — `schedule.start_date DESC NULLS LAST`. Sort in-memory
        // after hydration.
        out.sort_by_key(|b| std::cmp::Reverse(schedule_start(b)));
        Ok(out)
    }

    async fn get_instance(
        &self,
        course_id: &Uuid,
        instance_id: &Uuid,
    ) -> Result<Option<CourseInstance>> {
        let row = course_instances::Entity::find_by_id(*instance_id)
            .filter(course_instances::Column::CourseId.eq(*course_id))
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let Some(row) = row else { return Ok(None) };
        if row.deleted_at.is_some() {
            return Ok(None);
        }
        Ok(Some(hydrate_instance(&self.db, row).await?))
    }

    async fn create_instance(&self, instance: &CourseInstance) -> Result<CourseInstance> {
        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
        to_instance_active(instance, false)?
            .insert(&txn)
            .await
            .map_err(|e| map_db(&e))?;
        insert_instance_collections(&txn, instance).await?;
        txn.commit().await.map_err(|e| map_db(&e))?;
        self.get_instance(&instance.course_id, &instance.id)
            .await?
            .ok_or_else(|| crate::Error::Database("instance not found after insert".into()))
    }

    async fn update_instance(&self, instance: &CourseInstance) -> Result<CourseInstance> {
        let exists = course_instances::Entity::find_by_id(instance.id)
            .filter(course_instances::Column::CourseId.eq(instance.course_id))
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?;
        let Some(row) = exists else {
            return Err(crate::Error::NotFound);
        };
        if row.deleted_at.is_some() {
            return Err(crate::Error::NotFound);
        }
        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;
        to_instance_active(instance, true)?
            .update(&txn)
            .await
            .map_err(|e| map_db(&e))?;
        delete_instance_collections(&txn, instance.id).await?;
        insert_instance_collections(&txn, instance).await?;
        txn.commit().await.map_err(|e| map_db(&e))?;
        self.get_instance(&instance.course_id, &instance.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete_instance(&self, course_id: &Uuid, instance_id: &Uuid) -> Result<()> {
        let row = course_instances::Entity::find_by_id(*instance_id)
            .filter(course_instances::Column::CourseId.eq(*course_id))
            .one(&self.db)
            .await
            .map_err(|e| map_db(&e))?
            .ok_or(crate::Error::NotFound)?;
        if row.deleted_at.is_some() {
            return Err(crate::Error::NotFound);
        }
        let mut active: course_instances::ActiveModel = row.into();
        active.deleted_at = Set(Some(OffsetDateTime::now_utc()));
        active.updated_at = Set(OffsetDateTime::now_utc());
        active.update(&self.db).await.map_err(|e| map_db(&e))?;
        Ok(())
    }

    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord> {
        let active = course_merge_records::ActiveModel {
            id: Set(rec.id),
            main_course_id: Set(rec.main_course_id),
            duplicate_course_id: Set(rec.duplicate_course_id),
            status: Set(enum_to_string(&rec.status)?),
            merged_by: Set(rec.merged_by.clone()),
            merge_reason: Set(rec.merge_reason.clone()),
            match_score: Set(rec.match_score),
            transferred_data: Set(rec.transferred_data.clone()),
            merged_at: Set(ts_to_offset(rec.merged_at)),
        };
        active.insert(&self.db).await.map_err(|e| map_db(&e))?;
        Ok(rec.clone())
    }

    async fn merge(&self, survivor: &Course, duplicate_id: &Uuid) -> Result<Course> {
        // Reload the duplicate's pre-image so its `Deleted` outbox envelope
        // carries the record's name (needed for the operator projection).
        let old_duplicate = self.get_by_id(duplicate_id).await?;

        // The whole merge — survivor row updates + duplicate soft-delete +
        // both outbox rows — commits (or rolls back) under one transaction.
        let txn = self.db.begin().await.map_err(|e| map_db(&e))?;

        // Apply the survivor's parent + child rows (shared with `update`).
        apply_course_update(&txn, survivor).await?;

        // Soft-delete the duplicate (shared shape with `soft_delete`).
        apply_course_soft_delete(&txn, *duplicate_id).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor (carrying the duplicate's pid via `merged_from`, so a
        // merge-repointing consumer can move edges off the duplicate) and a
        // `Deleted` outbox row for the duplicate — both inside this
        // transaction. A no-op under the memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&txn)
                .await?;
            if let Some(dup) = old_duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&txn)
                    .await?;
            }
        }

        txn.commit().await.map_err(|e| map_db(&e))?;

        self.get_by_id(&survivor.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }
}

/// Apply the merged `course`'s parent-row update + wholesale child-row
/// replacement on `conn`. Shared by [`CourseRepository::update`] and
/// [`CourseRepository::merge`]: updates the `courses` row (stamping
/// `updated_at`), then replaces the identifier / link / text-value /
/// credential child rows from the incoming course (PUT semantics). Runs
/// on the caller's connection/transaction so it shares that commit
/// boundary.
async fn apply_course_update<C: ConnectionTrait>(conn: &C, course: &Course) -> Result<()> {
    let active = to_course_active(course, true)?;
    active.update(conn).await.map_err(|e| map_db(&e))?;
    // Child collections are replace-not-merge: delete the existing rows
    // then re-insert from the incoming course, so the persisted state
    // mirrors the request body exactly.
    course_identifiers::Entity::delete_many()
        .filter(course_identifiers::Column::CourseId.eq(course.id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    course_links::Entity::delete_many()
        .filter(course_links::Column::CourseId.eq(course.id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    delete_course_collections(conn, course.id).await?;
    insert_identifiers(conn, course.id, &course.identifiers).await?;
    insert_links(conn, course.id, &course.links).await?;
    insert_course_collections(conn, course).await?;
    Ok(())
}

/// Soft-delete the course `id` on `conn` by stamping `deleted_at` /
/// `updated_at` and flipping `active` off, leaving every other column in
/// place. Factored so [`CourseRepository::merge`] can reuse it on its own
/// transaction (a partial UPDATE via `..Default::default()`).
async fn apply_course_soft_delete<C: ConnectionTrait>(conn: &C, id: Uuid) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    let active = courses::ActiveModel {
        id: Set(id),
        active: Set(false),
        deleted_at: Set(Some(now)),
        updated_at: Set(now),
        // Clear the digests rather than leave ones that no longer
        // describe the row. This is a partial update — the record is not
        // loaded here, and reassembling it would add several queries to a
        // path `merge` also uses — so recomputing is not available.
        //
        // The consequence is stated rather than hidden: a soft-deleted
        // course reports as `unhashed`, not as verified and not as
        // tampered. That is the honest reading, and it avoids the worse
        // outcome of every soft delete surfacing as a false finding.
        content_hash: Set(None),
        content_hash_sha3: Set(None),
        content_mac: Set(None),
        ..Default::default()
    };
    active.update(conn).await.map_err(|e| map_db(&e))?;
    Ok(())
}

/// Pull an instance's schedule start date for FR-10 in-memory sorting.
fn schedule_start(i: &CourseInstance) -> Option<chrono::DateTime<chrono::Utc>> {
    i.schedule.as_ref().and_then(|s| s.start_date)
}

// ────────────────── Domain ↔ DB conversion ──────────────────

/// Build a `courses` `SeaORM` `ActiveModel` from a domain [`Course`],
/// serialising collection fields to JSONB. On update, `updated_at` is
/// stamped to now; on insert the model's own timestamp is kept.
fn to_course_active(course: &Course, is_update: bool) -> Result<courses::ActiveModel> {
    let now = OffsetDateTime::now_utc();
    // All three digests from one call over the **assembled** record, so
    // the child collections are covered too. Computed here rather than
    // stamped afterwards, so one statement writes the row and its
    // integrity values together — and every full write builds its active
    // model through this function.
    let digests = crate::compliance::record_integrity::digests(
        &crate::compliance::record_integrity::RecordInput {
            id: course.id,
            course,
            active: course.active,
        },
    );
    Ok(courses::ActiveModel {
        id: Set(course.id),
        content_hash: Set(Some(digests.sha256)),
        content_hash_sha3: Set(Some(digests.sha3)),
        content_mac: Set(digests.mac),
        name: Set(course.name.clone()),
        description: Set(course.description.clone()),
        disambiguating_description: Set(course.disambiguating_description.clone()),
        url: Set(course.url.clone()),
        additional_type: Set(course.additional_type.clone()),
        audience: Set(course.audience.clone()),
        license: Set(course.license.clone()),
        typical_age_range: Set(course.typical_age_range.clone()),
        time_required: Set(course.time_required.clone()),
        version: Set(course.version.clone()),
        is_accessible_for_free: Set(course.is_accessible_for_free),
        educational_level: Set(course
            .educational_level
            .as_ref()
            .map(enum_to_string)
            .transpose()?),
        educational_use: Set(course.educational_use.clone()),
        learning_resource_type: Set(course
            .learning_resource_type
            .as_ref()
            .map(enum_to_string)
            .transpose()?),
        interactivity_type: Set(course
            .interactivity_type
            .as_ref()
            .map(enum_to_string)
            .transpose()?),
        course_code: Set(course.course_code.clone()),
        number_of_credits: Set(course
            .number_of_credits
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX))),
        total_historical_enrollment: Set(course
            .total_historical_enrollment
            .map(|v| i64::try_from(v).unwrap_or(i64::MAX))),
        status: Set(enum_to_string(&course.status)?),
        active: Set(course.active),
        provider_id: Set(course.provider_id),
        created_at: Set(ts_to_offset(course.created_at)),
        updated_at: Set(if is_update {
            now
        } else {
            ts_to_offset(course.updated_at)
        }),
        deleted_at: Set(course.deleted_at.map(ts_to_offset)),
    })
}

/// The course string-list fields, as `(field tag, &Vec<String>)` pairs, so
/// `course_text_values` rows can be written/read uniformly.
fn course_text_fields(course: &Course) -> [(&'static str, &Vec<String>); 12] {
    [
        ("alternate_name", &course.alternate_names),
        ("image", &course.image),
        ("same_as", &course.same_as),
        ("keyword", &course.keywords),
        ("about", &course.about),
        ("in_language", &course.in_language),
        ("teaches", &course.teaches),
        ("assesses", &course.assesses),
        ("competency_required", &course.competency_required),
        ("course_prerequisite", &course.course_prerequisites),
        ("available_language", &course.available_language),
        ("financial_aid_eligible", &course.financial_aid_eligible),
    ]
}

/// Insert the `course_text_values` + `course_credentials` rows for `course`.
async fn insert_course_collections<C: sea_orm::ConnectionTrait>(
    conn: &C,
    course: &Course,
) -> Result<()> {
    for (field, values) in course_text_fields(course) {
        for (i, value) in values.iter().enumerate() {
            course_text_values::ActiveModel {
                id: Set(Uuid::new_v4()),
                course_id: Set(course.id),
                field: Set(field.to_string()),
                value: Set(value.clone()),
                position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
            }
            .insert(conn)
            .await
            .map_err(|e| map_db(&e))?;
        }
    }
    if let Some(cred) = &course.educational_credential_awarded {
        insert_credential(conn, course.id, "educational", cred).await?;
    }
    if let Some(cred) = &course.occupational_credential_awarded {
        insert_credential(conn, course.id, "occupational", cred).await?;
    }
    Ok(())
}

/// Insert one credential row in the given `role`.
async fn insert_credential<C: sea_orm::ConnectionTrait>(
    conn: &C,
    course_id: Uuid,
    role: &str,
    cred: &crate::models::EducationalCredential,
) -> Result<()> {
    course_credentials::ActiveModel {
        id: Set(Uuid::new_v4()),
        course_id: Set(course_id),
        role: Set(role.to_string()),
        name: Set(cred.name.clone()),
        category: Set(cred.category.as_ref().map(enum_to_string).transpose()?),
        educational_level: Set(cred.educational_level.clone()),
        recognized_by: Set(cred.recognized_by.clone()),
        url: Set(cred.url.clone()),
    }
    .insert(conn)
    .await
    .map_err(|e| map_db(&e))?;
    Ok(())
}

/// Delete the `course_text_values` + `course_credentials` rows for a course.
async fn delete_course_collections<C: sea_orm::ConnectionTrait>(
    conn: &C,
    course_id: Uuid,
) -> Result<()> {
    course_text_values::Entity::delete_many()
        .filter(course_text_values::Column::CourseId.eq(course_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    course_credentials::Entity::delete_many()
        .filter(course_credentials::Column::CourseId.eq(course_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    Ok(())
}

/// Load the `course_text_values` + `course_credentials` rows for a course.
async fn load_course_collections(
    db: &DatabaseConnection,
    course_id: Uuid,
) -> Result<(
    Vec<course_text_values::Model>,
    Vec<course_credentials::Model>,
)> {
    let text_values = course_text_values::Entity::find()
        .filter(course_text_values::Column::CourseId.eq(course_id))
        .order_by_asc(course_text_values::Column::Position)
        .all(db)
        .await
        .map_err(|e| map_db(&e))?;
    let credentials = course_credentials::Entity::find()
        .filter(course_credentials::Column::CourseId.eq(course_id))
        .all(db)
        .await
        .map_err(|e| map_db(&e))?;
    Ok((text_values, credentials))
}

/// Collect the values of one tagged field from loaded text-value rows.
fn texts(rows: &[course_text_values::Model], field: &str) -> Vec<String> {
    rows.iter()
        .filter(|r| r.field == field)
        .map(|r| r.value.clone())
        .collect()
}

/// Reconstruct an [`EducationalCredential`] from a row, or `None`.
fn credential_of(
    rows: &[course_credentials::Model],
    role: &str,
) -> Result<Option<crate::models::EducationalCredential>> {
    rows.iter()
        .find(|r| r.role == role)
        .map(|r| {
            Ok(crate::models::EducationalCredential {
                name: r.name.clone(),
                category: r.category.as_deref().map(enum_from_string).transpose()?,
                educational_level: r.educational_level.clone(),
                recognized_by: r.recognized_by.clone(),
                url: r.url.clone(),
            })
        })
        .transpose()
}

/// Reconstruct a domain [`Course`] from its `courses` row plus the
/// child identifier/link collections, deserialising JSONB columns.
/// Sub-resources (`syllabus_sections`, `instances`) are left empty —
/// they load via their own endpoints.
fn hydrate_course(
    row: courses::Model,
    identifiers: Vec<CourseIdentifier>,
    links: Vec<CourseLink>,
    text_values: &[course_text_values::Model],
    credentials: &[course_credentials::Model],
) -> Result<Course> {
    Ok(Course {
        id: row.id,
        name: row.name,
        alternate_names: texts(text_values, "alternate_name"),
        description: row.description,
        disambiguating_description: row.disambiguating_description,
        url: row.url,
        image: texts(text_values, "image"),
        same_as: texts(text_values, "same_as"),
        keywords: texts(text_values, "keyword"),
        identifiers,
        additional_type: row.additional_type,
        active: row.active,
        about: texts(text_values, "about"),
        audience: row.audience,
        in_language: texts(text_values, "in_language"),
        license: row.license,
        typical_age_range: row.typical_age_range,
        time_required: row.time_required,
        version: row.version,
        is_accessible_for_free: row.is_accessible_for_free,
        teaches: texts(text_values, "teaches"),
        assesses: texts(text_values, "assesses"),
        competency_required: texts(text_values, "competency_required"),
        educational_level: row
            .educational_level
            .as_deref()
            .map(enum_from_string::<EducationalLevel>)
            .transpose()?,
        educational_use: row.educational_use,
        learning_resource_type: row
            .learning_resource_type
            .as_deref()
            .map(enum_from_string::<LearningResourceType>)
            .transpose()?,
        interactivity_type: row
            .interactivity_type
            .as_deref()
            .map(enum_from_string::<InteractivityType>)
            .transpose()?,
        course_code: row.course_code,
        number_of_credits: row.number_of_credits.map(|v| u32::try_from(v).unwrap_or(0)),
        course_prerequisites: texts(text_values, "course_prerequisite"),
        available_language: texts(text_values, "available_language"),
        financial_aid_eligible: texts(text_values, "financial_aid_eligible"),
        educational_credential_awarded: credential_of(credentials, "educational")?,
        occupational_credential_awarded: credential_of(credentials, "occupational")?,
        total_historical_enrollment: row
            .total_historical_enrollment
            .map(|v| u64::try_from(v).unwrap_or(0)),
        syllabus_sections: vec![],
        instances: vec![],
        status: enum_from_string::<CourseStatus>(&row.status)?,
        links,
        provider_id: row.provider_id,
        deleted_at: row.deleted_at.map(offset_to_ts),
        created_at: offset_to_ts(row.created_at),
        updated_at: offset_to_ts(row.updated_at),
    })
}

// ────────────────── Instance round-trip ──────────────────

/// Build a `course_instances` `ActiveModel` from a domain
/// [`CourseInstance`], serialising collection/schedule fields to JSONB.
fn to_instance_active(
    i: &CourseInstance,
    is_update: bool,
) -> Result<course_instances::ActiveModel> {
    let now = OffsetDateTime::now_utc();
    let sched = i.schedule.as_ref();
    Ok(course_instances::ActiveModel {
        id: Set(i.id),
        course_id: Set(i.course_id),
        name: Set(i.name.clone()),
        course_mode: Set(i.course_mode.as_ref().map(enum_to_string).transpose()?),
        status: Set(enum_to_string(&i.status)?),
        location: Set(i.location.clone()),
        location_id: Set(i.location_id),
        maximum_attendee_capacity: Set(i
            .maximum_attendee_capacity
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX))),
        enrolled_count: Set(i
            .enrolled_count
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX))),
        enrollment_opens: Set(i.enrollment_opens.map(ts_to_offset)),
        enrollment_closes: Set(i.enrollment_closes.map(ts_to_offset)),
        schedule_start_date: Set(sched.and_then(|s| s.start_date).map(ts_to_offset)),
        schedule_end_date: Set(sched.and_then(|s| s.end_date).map(ts_to_offset)),
        schedule_time_zone: Set(sched.and_then(|s| s.time_zone.clone())),
        schedule_recurrence: Set(sched.and_then(|s| s.recurrence.clone())),
        created_at: Set(ts_to_offset(i.created_at)),
        updated_at: Set(if is_update {
            now
        } else {
            ts_to_offset(i.updated_at)
        }),
        deleted_at: Set(None),
    })
}

/// Insert an instance's normalized child collections (languages,
/// instructors, sessions) on connection `conn`.
async fn insert_instance_collections<C: sea_orm::ConnectionTrait>(
    conn: &C,
    i: &CourseInstance,
) -> Result<()> {
    for (idx, lang) in i.in_language.iter().enumerate() {
        course_instance_languages::ActiveModel {
            id: Set(Uuid::new_v4()),
            instance_id: Set(i.id),
            language: Set(lang.clone()),
            position: Set(i32::try_from(idx).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await
        .map_err(|e| map_db(&e))?;
    }
    // Instructor ids and names are parallel lists in the domain model; emit
    // one row per position, carrying whichever of (id, name) is present.
    let n = i.instructor_ids.len().max(i.instructor_names.len());
    for idx in 0..n {
        course_instance_instructors::ActiveModel {
            id: Set(Uuid::new_v4()),
            instance_id: Set(i.id),
            instructor_id: Set(i.instructor_ids.get(idx).copied()),
            instructor_name: Set(i.instructor_names.get(idx).cloned()),
            position: Set(i32::try_from(idx).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await
        .map_err(|e| map_db(&e))?;
    }
    if let Some(s) = &i.schedule {
        for (idx, session) in s.sessions.iter().enumerate() {
            course_instance_sessions::ActiveModel {
                id: Set(Uuid::new_v4()),
                instance_id: Set(i.id),
                start_at: Set(ts_to_offset(session.start)),
                end_at: Set(session.end.map(ts_to_offset)),
                label: Set(session.label.clone()),
                position: Set(i32::try_from(idx).unwrap_or(i32::MAX)),
            }
            .insert(conn)
            .await
            .map_err(|e| map_db(&e))?;
        }
    }
    Ok(())
}

/// Delete an instance's normalized child collections on connection `conn`.
async fn delete_instance_collections<C: sea_orm::ConnectionTrait>(
    conn: &C,
    instance_id: Uuid,
) -> Result<()> {
    course_instance_languages::Entity::delete_many()
        .filter(course_instance_languages::Column::InstanceId.eq(instance_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    course_instance_instructors::Entity::delete_many()
        .filter(course_instance_instructors::Column::InstanceId.eq(instance_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    course_instance_sessions::Entity::delete_many()
        .filter(course_instance_sessions::Column::InstanceId.eq(instance_id))
        .exec(conn)
        .await
        .map_err(|e| map_db(&e))?;
    Ok(())
}

/// Reconstruct a domain [`CourseInstance`] from its row, deserialising
/// JSONB columns and widening the stored `i32` counts back to `u32`.
async fn hydrate_instance(
    db: &DatabaseConnection,
    row: course_instances::Model,
) -> Result<CourseInstance> {
    let id = row.id;

    let lang_rows = course_instance_languages::Entity::find()
        .filter(course_instance_languages::Column::InstanceId.eq(id))
        .order_by_asc(course_instance_languages::Column::Position)
        .all(db)
        .await
        .map_err(|e| map_db(&e))?;
    let in_language = lang_rows.into_iter().map(|r| r.language).collect();

    let inst_rows = course_instance_instructors::Entity::find()
        .filter(course_instance_instructors::Column::InstanceId.eq(id))
        .order_by_asc(course_instance_instructors::Column::Position)
        .all(db)
        .await
        .map_err(|e| map_db(&e))?;
    let instructor_ids = inst_rows.iter().filter_map(|r| r.instructor_id).collect();
    let instructor_names = inst_rows
        .into_iter()
        .filter_map(|r| r.instructor_name)
        .collect();

    let session_rows = course_instance_sessions::Entity::find()
        .filter(course_instance_sessions::Column::InstanceId.eq(id))
        .order_by_asc(course_instance_sessions::Column::Position)
        .all(db)
        .await
        .map_err(|e| map_db(&e))?;
    let sessions: Vec<crate::models::course_instance::Session> = session_rows
        .into_iter()
        .map(|s| crate::models::course_instance::Session {
            start: offset_to_ts(s.start_at),
            end: s.end_at.map(offset_to_ts),
            label: s.label,
        })
        .collect();

    let has_schedule = row.schedule_start_date.is_some()
        || row.schedule_end_date.is_some()
        || row.schedule_time_zone.is_some()
        || row.schedule_recurrence.is_some()
        || !sessions.is_empty();
    let schedule = has_schedule.then(|| Schedule {
        start_date: row.schedule_start_date.map(offset_to_ts),
        end_date: row.schedule_end_date.map(offset_to_ts),
        time_zone: row.schedule_time_zone.clone(),
        recurrence: row.schedule_recurrence.clone(),
        sessions,
    });

    Ok(CourseInstance {
        id: row.id,
        course_id: row.course_id,
        name: row.name,
        course_mode: row
            .course_mode
            .as_deref()
            .map(enum_from_string::<CourseMode>)
            .transpose()?,
        status: enum_from_string::<CourseInstanceStatus>(&row.status)?,
        in_language,
        location: row.location,
        location_id: row.location_id,
        instructor_ids,
        instructor_names,
        maximum_attendee_capacity: row
            .maximum_attendee_capacity
            .map(|v| u32::try_from(v).unwrap_or(0)),
        enrolled_count: row.enrolled_count.map(|v| u32::try_from(v).unwrap_or(0)),
        enrollment_opens: row.enrollment_opens.map(offset_to_ts),
        enrollment_closes: row.enrollment_closes.map(offset_to_ts),
        schedule,
        created_at: offset_to_ts(row.created_at),
        updated_at: offset_to_ts(row.updated_at),
    })
}

// ────────────────── Child-table round-trip ──────────────────

/// Insert each [`CourseIdentifier`] as a `course_identifiers` child row
/// under `course_id`. Generic over the connection so it runs inside a
/// transaction.
async fn insert_identifiers<C>(
    conn: &C,
    course_id: Uuid,
    identifiers: &[CourseIdentifier],
) -> Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    for (i, ident) in identifiers.iter().enumerate() {
        let (property_id, custom_label) = ident_type_parts(&ident.property_id)?;
        let row = course_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            course_id: Set(course_id),
            property_id: Set(property_id),
            custom_label: Set(custom_label),
            value: Set(ident.value.clone()),
            name: Set(ident.name.clone()),
            url: Set(ident.url.clone()),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
            created_at: Set(OffsetDateTime::now_utc()),
        };
        row.insert(conn).await.map_err(|e| map_db(&e))?;
    }
    Ok(())
}

/// Split a course [`IdentifierType`] into its `(tag, custom_label)` columns.
/// Non-`Custom` variants use the serde string tag; `Custom(s)` stores the
/// literal tag `"Custom"` with the label in `custom_label`.
fn ident_type_parts(t: &IdentifierType) -> Result<(String, Option<String>)> {
    match t {
        IdentifierType::Custom(label) => Ok(("Custom".to_string(), Some(label.clone()))),
        other => Ok((enum_to_string(other)?, None)),
    }
}

/// Inverse of [`ident_type_parts`].
fn parse_ident_type(tag: &str, custom_label: Option<String>) -> Result<IdentifierType> {
    if tag == "Custom" {
        Ok(IdentifierType::Custom(custom_label.unwrap_or_default()))
    } else {
        enum_from_string::<IdentifierType>(tag)
    }
}

/// Insert each [`CourseLink`] as a `course_links` child row under
/// `course_id`. Generic over the connection so it runs in a transaction.
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
            created_at: Set(OffsetDateTime::now_utc()),
        };
        row.insert(conn).await.map_err(|e| map_db(&e))?;
    }
    Ok(())
}

/// Load and deserialise all `course_identifiers` rows for `course_id`.
async fn load_identifiers(
    db: &DatabaseConnection,
    course_id: Uuid,
) -> Result<Vec<CourseIdentifier>> {
    let rows = course_identifiers::Entity::find()
        .filter(course_identifiers::Column::CourseId.eq(course_id))
        .order_by_asc(course_identifiers::Column::Position)
        .all(db)
        .await
        .map_err(|e| map_db(&e))?;
    rows.into_iter()
        .map(|r| {
            Ok(CourseIdentifier {
                property_id: parse_ident_type(&r.property_id, r.custom_label)?,
                value: r.value,
                name: r.name,
                url: r.url,
            })
        })
        .collect()
}

/// Load and deserialise all `course_links` rows for `course_id`.
async fn load_links(db: &DatabaseConnection, course_id: Uuid) -> Result<Vec<CourseLink>> {
    let rows = course_links::Entity::find()
        .filter(course_links::Column::CourseId.eq(course_id))
        .all(db)
        .await
        .map_err(|e| map_db(&e))?;
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

/// Map a `SeaORM` error into the crate's [`Error::Database`](crate::Error).
fn map_db(e: &sea_orm::DbErr) -> crate::Error {
    crate::Error::Database(e.to_string())
}

/// Serialise a string-valued enum to its bare string form (used for
/// `status`-style columns stored as `TEXT` rather than JSONB).
fn enum_to_string<T: Serialize>(v: &T) -> Result<String> {
    let json = serde_json::to_value(v).map_err(|e| crate::Error::Database(e.to_string()))?;
    json.as_str()
        .map(std::string::ToString::to_string)
        .ok_or_else(|| crate::Error::Database("enum did not serialise to a string".into()))
}

/// Inverse of [`enum_to_string`]: parse a bare string back into a
/// string-valued enum `T`.
fn enum_from_string<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| crate::Error::Database(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CourseStatus;

    /// Every `CourseStatus` survives an enum→string→enum round-trip.
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

    /// Every `LinkType` survives an enum→string→enum round-trip.
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

    /// `to_course_active` copies scalar fields onto the `ActiveModel`.
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

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path.
/// They require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped
/// by a bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod outbox_atomicity_tests {
    use super::{CourseRepository, SeaOrmCourseRepository};
    use crate::db::models::course_outbox;
    use crate::models::Course;
    use crate::streaming::EventTransport;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    async fn connect() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Create writes a `created` outbox row in the same transaction as the
    /// entity insert (under the outbox transport).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = SeaOrmCourseRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let course = repo.create(&Course::new("Intro to CS")).await.unwrap();

        let rows = course_outbox::Entity::find()
            .filter(course_outbox::Column::EntityPid.eq(course.id))
            .filter(course_outbox::Column::Kind.eq("created"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "one created outbox row for the new course");
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = SeaOrmCourseRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo.create(&Course::new("Survivor")).await.unwrap();
        let duplicate = repo.create(&Course::new("Duplicate")).await.unwrap();

        let merged = repo.merge(&survivor, &duplicate.id).await.unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = course_outbox::Entity::find()
            .filter(course_outbox::Column::EntityPid.eq(survivor.id))
            .filter(course_outbox::Column::Kind.eq("merged"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = course_outbox::Entity::find()
            .filter(course_outbox::Column::EntityPid.eq(duplicate.id))
            .filter(course_outbox::Column::Kind.eq("deleted"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(
            deleted_rows.len(),
            1,
            "one deleted outbox row for duplicate"
        );
    }
}
