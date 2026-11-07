//! Database access — the connection pool, the migrator, and the
//! repository. **Every SQL statement this service runs is in this
//! directory** (spec/tech-stack §5.2), written as a `sqlx::query!` /
//! `query_as!` / `query_scalar!` literal so the compiler checks it against
//! the schema.
//!
//! [`PgCourseRepository`] round-trips a domain [`Course`] against the
//! relational schema: scalar fields live on the `courses` row; the
//! string-list fields live in `course_text_values`, the two credentials in
//! `course_credentials`, and `identifiers` and `links` in their own child
//! tables. Sub-resources (`instances`, `syllabus_sections`) have their own
//! dedicated API surface and are intentionally not loaded with a course.

mod convert;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::Result;
use crate::config::DatabaseConfig;
use crate::db::outbox::OutboxInsert;
use crate::models::{
    Course, CourseIdentifier, CourseInstance, CourseInstanceStatus, CourseLink, CourseMode,
    CourseStatus, DecideOutcome, EducationalLevel, IdentifierType, InteractivityType,
    LearningResourceType, LinkType, MergeRecord, NewReviewItem, ReviewQueueItem, ReviewStatus,
    Schedule, canonical_pair,
};
use crate::streaming::EventTransport;
use crate::streaming::envelope::EventKind;

pub mod audit;
pub mod models;
pub mod outbox;

use models::{
    CourseCredentialRow, CourseInstanceRow, CourseRow, CourseTextValueRow, ReviewQueueRow,
};

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `course-service db migrate | down | reset | status` and the boot
/// auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open a connection pool from a `DatabaseConfig`.
///
/// # Errors
///
/// Returns [`enum@crate::Error`] if the pool cannot connect to the
/// configured database URL.
pub async fn create_connection(config: &DatabaseConfig) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
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

    // Persisted review queue (T-27, `course_match_scores`).
    /// Insert-or-refresh review-queue candidate pairs and return the
    /// stored rows. Pair order is normalized to [`canonical_pair`]
    /// before write, so `(a, b)` and `(b, a)` upsert the same row: a
    /// re-scan refreshes the score columns but never touches a
    /// previously-decided row's `status`.
    async fn upsert_review_items(&self, items: &[NewReviewItem]) -> Result<Vec<ReviewQueueItem>>;
    /// List stored review-queue items, newest first, optionally
    /// filtered by `status`. `limit` is capped at 500.
    async fn list_review_items(
        &self,
        status: Option<ReviewStatus>,
        limit: u64,
    ) -> Result<Vec<ReviewQueueItem>>;
    /// Record an operator decision on one `Pending` review item.
    /// First-writer-wins: only a `Pending` row transitions.
    async fn decide_review_item(
        &self,
        id: Uuid,
        status: ReviewStatus,
        reviewed_by: Option<&str>,
    ) -> Result<DecideOutcome>;
}

/// sqlx-backed [`CourseRepository`] implementation over a `PostgreSQL`
/// connection pool.
pub struct PgCourseRepository {
    /// The shared connection pool.
    db: PgPool,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps only the post-commit
    /// in-memory publish (done by the handlers); [`EventTransport::Outbox`]
    /// additionally writes one `course_outbox` row **inside** each write's
    /// transaction.
    transport: EventTransport,
}

impl PgCourseRepository {
    /// Wrap an existing connection pool in a repository, with the default
    /// [`EventTransport::Memory`] transport (behaviour unchanged from
    /// before the outbox landed).
    #[must_use]
    pub fn new(db: PgPool) -> Self {
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
    /// transaction so the row commits with the entity write (the outbox
    /// atomicity guarantee). A no-op under [`EventTransport::Memory`].
    async fn enqueue_outbox(
        &self,
        conn: &mut PgConnection,
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

    /// Read one `courses` row by id, soft-deleted or not.
    async fn find_row(&self, id: Uuid) -> Result<Option<CourseRow>> {
        let row = sqlx::query_as!(
            CourseRow,
            "SELECT id, name, content_hash, content_hash_sha3, content_mac, description,
                    disambiguating_description, url, additional_type, audience, license,
                    typical_age_range, time_required, version, is_accessible_for_free,
                    educational_level, educational_use, learning_resource_type,
                    interactivity_type, course_code, number_of_credits,
                    total_historical_enrollment, status, active, provider_id,
                    created_at, updated_at, deleted_at
             FROM courses
             WHERE id = $1",
            id,
        )
        .fetch_optional(&self.db)
        .await?;
        Ok(row)
    }

    /// Load a course's child collections and assemble the domain
    /// [`Course`].
    async fn hydrate(&self, row: CourseRow) -> Result<Course> {
        let id = row.id;
        let identifiers = load_identifiers(&self.db, id).await?;
        let links = load_links(&self.db, id).await?;
        let (text_values, credentials) = load_course_collections(&self.db, id).await?;
        hydrate_course(row, identifiers, links, &text_values, &credentials)
    }

    /// Read one `course_instances` row by id, scoped to its parent course,
    /// soft-deleted or not.
    async fn find_instance_row(
        &self,
        course_id: Uuid,
        instance_id: Uuid,
    ) -> Result<Option<CourseInstanceRow>> {
        let row = sqlx::query_as!(
            CourseInstanceRow,
            "SELECT id, course_id, name, course_mode, status, location, location_id,
                    maximum_attendee_capacity, enrolled_count, enrollment_opens,
                    enrollment_closes, schedule_start_date, schedule_end_date,
                    schedule_time_zone, schedule_recurrence, created_at, updated_at, deleted_at
             FROM course_instances
             WHERE id = $1 AND course_id = $2",
            instance_id,
            course_id,
        )
        .fetch_optional(&self.db)
        .await?;
        Ok(row)
    }
}

#[async_trait::async_trait]
impl CourseRepository for PgCourseRepository {
    async fn create(&self, course: &Course) -> Result<Course> {
        // One transaction so the parent row and every child collection
        // (identifiers, links, text-values, credentials) commit atomically.
        let mut txn = self.db.begin().await?;
        insert_course_row(&mut txn, &course_row(course, false)?).await?;
        insert_identifiers(&mut txn, course.id, &course.identifiers).await?;
        insert_links(&mut txn, course.id, &course.links).await?;
        insert_course_collections(&mut txn, course).await?;
        // Durable event bus (Phase 2): under the outbox transport, write
        // the `course_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the post-commit in-memory publish in the handler.
        self.enqueue_outbox(&mut txn, course, EventKind::Created)
            .await?;
        txn.commit().await?;
        // Re-read through the normal hydrate path so the returned value
        // reflects exactly what was persisted (defaults, ordering, etc).
        self.get_by_id(&course.id)
            .await?
            .ok_or_else(|| crate::Error::Database("course not found after insert".into()))
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Course>> {
        let Some(row) = self.find_row(*id).await? else {
            return Ok(None);
        };
        // Soft-deleted rows read as absent — callers never see them.
        if row.deleted_at.is_some() {
            return Ok(None);
        }
        Ok(Some(self.hydrate(row).await?))
    }

    async fn update(&self, course: &Course) -> Result<Course> {
        if self.find_row(course.id).await?.is_none() {
            return Err(crate::Error::NotFound);
        }
        let mut txn = self.db.begin().await?;
        apply_course_update(&mut txn, course).await?;
        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&mut txn, course, EventKind::Updated)
            .await?;
        txn.commit().await?;
        self.get_by_id(&course.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete(&self, id: &Uuid) -> Result<()> {
        if self.find_row(*id).await?.is_none() {
            return Err(crate::Error::NotFound);
        }
        // Soft delete: flip `active` off and stamp `deleted_at` rather
        // than removing the row, preserving it for the audit trail.
        // Under the outbox transport, open a transaction so the tombstone
        // write and the `deleted` outbox row commit atomically; the memory
        // transport keeps the plain, tx-free update. The domain course is
        // reloaded (still visible pre-delete) to build the deleted envelope.
        if self.transport.is_outbox() {
            let old = self.get_by_id(id).await?;
            let mut txn = self.db.begin().await?;
            mark_course_deleted(&mut txn, *id).await?;
            if let Some(old) = old.as_ref() {
                self.enqueue_outbox(&mut txn, old, EventKind::Deleted)
                    .await?;
            }
            txn.commit().await?;
        } else {
            let mut conn = self.db.acquire().await?;
            mark_course_deleted(&mut conn, *id).await?;
        }
        Ok(())
    }

    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Course>> {
        let rows = sqlx::query_as!(
            CourseRow,
            "SELECT id, name, content_hash, content_hash_sha3, content_mac, description,
                    disambiguating_description, url, additional_type, audience, license,
                    typical_age_range, time_required, version, is_accessible_for_free,
                    educational_level, educational_use, learning_resource_type,
                    interactivity_type, course_code, number_of_credits,
                    total_historical_enrollment, status, active, provider_id,
                    created_at, updated_at, deleted_at
             FROM courses
             WHERE deleted_at IS NULL
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2",
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(&self.db)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.hydrate(row).await?);
        }
        Ok(out)
    }

    // ──────────── CourseInstance sub-resource ────────────

    async fn list_instances(&self, course_id: &Uuid) -> Result<Vec<CourseInstance>> {
        let rows = sqlx::query_as!(
            CourseInstanceRow,
            "SELECT id, course_id, name, course_mode, status, location, location_id,
                    maximum_attendee_capacity, enrolled_count, enrollment_opens,
                    enrollment_closes, schedule_start_date, schedule_end_date,
                    schedule_time_zone, schedule_recurrence, created_at, updated_at, deleted_at
             FROM course_instances
             WHERE course_id = $1 AND deleted_at IS NULL",
            course_id,
        )
        .fetch_all(&self.db)
        .await?;
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
        let Some(row) = self.find_instance_row(*course_id, *instance_id).await? else {
            return Ok(None);
        };
        if row.deleted_at.is_some() {
            return Ok(None);
        }
        Ok(Some(hydrate_instance(&self.db, row).await?))
    }

    async fn create_instance(&self, instance: &CourseInstance) -> Result<CourseInstance> {
        let mut txn = self.db.begin().await?;
        insert_instance_row(&mut txn, &instance_row(instance, false)?).await?;
        insert_instance_collections(&mut txn, instance).await?;
        txn.commit().await?;
        self.get_instance(&instance.course_id, &instance.id)
            .await?
            .ok_or_else(|| crate::Error::Database("instance not found after insert".into()))
    }

    async fn update_instance(&self, instance: &CourseInstance) -> Result<CourseInstance> {
        let Some(row) = self
            .find_instance_row(instance.course_id, instance.id)
            .await?
        else {
            return Err(crate::Error::NotFound);
        };
        if row.deleted_at.is_some() {
            return Err(crate::Error::NotFound);
        }
        let mut txn = self.db.begin().await?;
        update_instance_row(&mut txn, &instance_row(instance, true)?).await?;
        delete_instance_collections(&mut txn, instance.id).await?;
        insert_instance_collections(&mut txn, instance).await?;
        txn.commit().await?;
        self.get_instance(&instance.course_id, &instance.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn soft_delete_instance(&self, course_id: &Uuid, instance_id: &Uuid) -> Result<()> {
        let row = self
            .find_instance_row(*course_id, *instance_id)
            .await?
            .ok_or(crate::Error::NotFound)?;
        if row.deleted_at.is_some() {
            return Err(crate::Error::NotFound);
        }
        let done = sqlx::query!(
            "UPDATE course_instances SET deleted_at = $2, updated_at = $3 WHERE id = $1",
            row.id,
            Utc::now(),
            Utc::now(),
        )
        .execute(&self.db)
        .await?;
        // As under the ORM, an update that touches no row is an error.
        if done.rows_affected() == 0 {
            return Err(crate::Error::Database(
                "course instance not deleted: no such row".into(),
            ));
        }
        Ok(())
    }

    async fn record_merge(&self, rec: &MergeRecord) -> Result<MergeRecord> {
        sqlx::query!(
            "INSERT INTO course_merge_records
                 (id, main_course_id, duplicate_course_id, status, merged_by, merge_reason,
                  match_score, transferred_data, merged_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            rec.id,
            rec.main_course_id,
            rec.duplicate_course_id,
            enum_to_string(&rec.status)?,
            rec.merged_by,
            rec.merge_reason,
            rec.match_score,
            rec.transferred_data,
            rec.merged_at,
        )
        .execute(&self.db)
        .await?;
        Ok(rec.clone())
    }

    async fn merge(&self, survivor: &Course, duplicate_id: &Uuid) -> Result<Course> {
        // Reload the duplicate's pre-image so its `Deleted` outbox envelope
        // carries the record's name (needed for the operator projection).
        let old_duplicate = self.get_by_id(duplicate_id).await?;

        // The whole merge — survivor row updates + duplicate soft-delete +
        // both outbox rows — commits (or rolls back) under one transaction.
        let mut txn = self.db.begin().await?;

        // Apply the survivor's parent + child rows (shared with `update`).
        apply_course_update(&mut txn, survivor).await?;

        // Soft-delete the duplicate (shared shape with `soft_delete`).
        apply_course_soft_delete(&mut txn, *duplicate_id).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor (carrying the duplicate's pid via `merged_from`, so a
        // merge-repointing consumer can move edges off the duplicate) and a
        // `Deleted` outbox row for the duplicate — both inside this
        // transaction. A no-op under the memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&mut txn)
                .await?;
            if let Some(dup) = old_duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&mut txn)
                    .await?;
            }
        }

        txn.commit().await?;

        self.get_by_id(&survivor.id)
            .await?
            .ok_or(crate::Error::NotFound)
    }

    async fn upsert_review_items(&self, items: &[NewReviewItem]) -> Result<Vec<ReviewQueueItem>> {
        let pending = enum_to_string(&ReviewStatus::Pending)?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            // Normalize the pair order so (a, b) and (b, a) share one row
            // under the migration's plain `UNIQUE (course_id, candidate_id)`
            // constraint (T-27).
            let (course_id, candidate_id) = canonical_pair(item.course_id_a, item.course_id_b);
            // On conflict, refresh only the score-describing columns — the
            // row's `id`, `status`, `reviewed_by`, and `reviewed_at` are
            // left untouched, so a re-scan never clobbers a decided item.
            let row = sqlx::query_as!(
                ReviewQueueRow,
                "INSERT INTO course_match_scores
                     (id, course_id, candidate_id, match_score, match_quality, detection_method,
                      score_breakdown, status, reviewed_by, created_at, reviewed_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, NULL)
                 ON CONFLICT (course_id, candidate_id) DO UPDATE SET
                     match_score = EXCLUDED.match_score,
                     match_quality = EXCLUDED.match_quality,
                     detection_method = EXCLUDED.detection_method,
                     score_breakdown = EXCLUDED.score_breakdown
                 RETURNING id, course_id, candidate_id, match_score, match_quality,
                           detection_method, score_breakdown, status, reviewed_by,
                           created_at, reviewed_at",
                Uuid::new_v4(),
                course_id,
                candidate_id,
                item.match_score,
                item.match_quality,
                item.detection_method,
                item.score_breakdown,
                pending,
                Utc::now(),
            )
            .fetch_one(&self.db)
            .await?;
            out.push(review_row_to_item(row)?);
        }
        Ok(out)
    }

    async fn list_review_items(
        &self,
        status: Option<ReviewStatus>,
        limit: u64,
    ) -> Result<Vec<ReviewQueueItem>> {
        let status = status.as_ref().map(enum_to_string).transpose()?;
        let rows = sqlx::query_as!(
            ReviewQueueRow,
            "SELECT id, course_id, candidate_id, match_score, match_quality, detection_method,
                    score_breakdown, status, reviewed_by, created_at, reviewed_at
             FROM course_match_scores
             WHERE ($1::text IS NULL OR status = $1)
             ORDER BY created_at DESC, id ASC
             LIMIT $2",
            status,
            to_i64(limit.min(500)),
        )
        .fetch_all(&self.db)
        .await?;
        rows.into_iter().map(review_row_to_item).collect()
    }

    async fn decide_review_item(
        &self,
        id: Uuid,
        status: ReviewStatus,
        reviewed_by: Option<&str>,
    ) -> Result<DecideOutcome> {
        // The transition guard is the `WHERE status = 'Pending'` filter on
        // this single UPDATE statement: it runs atomically in Postgres, so
        // concurrent decisions cannot double-apply — exactly one caller's
        // update matches a row, the rest affect zero rows.
        let done = sqlx::query!(
            "UPDATE course_match_scores
             SET status = $1, reviewed_by = $2, reviewed_at = $3
             WHERE id = $4 AND status = $5",
            enum_to_string(&status)?,
            reviewed_by,
            Utc::now(),
            id,
            enum_to_string(&ReviewStatus::Pending)?,
        )
        .execute(&self.db)
        .await?;

        if done.rows_affected() == 1 {
            let row = find_review_row(&self.db, id)
                .await?
                .ok_or(crate::Error::NotFound)?;
            return Ok(DecideOutcome::Decided(Box::new(review_row_to_item(row)?)));
        }

        // No row was affected: either the id doesn't exist, or it exists
        // but isn't `Pending`. Probe to tell the two apart.
        match find_review_row(&self.db, id).await? {
            Some(row) => Ok(DecideOutcome::AlreadyDecided(enum_from_string(
                &row.status,
            )?)),
            None => Ok(DecideOutcome::NotFound),
        }
    }
}

/// Read one `course_match_scores` row by id.
async fn find_review_row(db: &PgPool, id: Uuid) -> Result<Option<ReviewQueueRow>> {
    let row = sqlx::query_as!(
        ReviewQueueRow,
        "SELECT id, course_id, candidate_id, match_score, match_quality, detection_method,
                score_breakdown, status, reviewed_by, created_at, reviewed_at
         FROM course_match_scores
         WHERE id = $1",
        id,
    )
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Map a stored `course_match_scores` row onto the wire
/// [`ReviewQueueItem`] shape (T-27).
fn review_row_to_item(row: ReviewQueueRow) -> Result<ReviewQueueItem> {
    Ok(ReviewQueueItem {
        id: row.id,
        course_id_a: row.course_id,
        course_id_b: row.candidate_id,
        match_score: row.match_score,
        match_quality: row.match_quality,
        detection_method: row.detection_method,
        score_breakdown: row.score_breakdown,
        status: enum_from_string(&row.status)?,
        reviewed_by: row.reviewed_by,
        created_at: row.created_at,
        reviewed_at: row.reviewed_at,
    })
}

/// The rows `GET /api/records/verify` recomputes, most recently updated
/// first, soft-deleted ones included (the handler's reassembly skips
/// those).
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn rows_for_verification(db: &PgPool, limit: u64) -> Result<Vec<CourseRow>> {
    let rows = sqlx::query_as!(
        CourseRow,
        "SELECT id, name, content_hash, content_hash_sha3, content_mac, description,
                disambiguating_description, url, additional_type, audience, license,
                typical_age_range, time_required, version, is_accessible_for_free,
                educational_level, educational_use, learning_resource_type,
                interactivity_type, course_code, number_of_credits,
                total_historical_enrollment, status, active, provider_id,
                created_at, updated_at, deleted_at
         FROM courses
         ORDER BY updated_at DESC
         LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Apply the merged `course`'s parent-row update + wholesale child-row
/// replacement on `conn`. Shared by [`CourseRepository::update`] and
/// [`CourseRepository::merge`]: rewrites every column of the `courses`
/// row (stamping `updated_at`; `created_at` is written from the course,
/// as it always was), then replaces the identifier / link / text-value /
/// credential child rows from the incoming course (PUT semantics). Runs
/// on the caller's connection/transaction so it shares that commit
/// boundary.
async fn apply_course_update(conn: &mut PgConnection, course: &Course) -> Result<()> {
    update_course_row(conn, &course_row(course, true)?).await?;
    // Child collections are replace-not-merge: delete the existing rows
    // then re-insert from the incoming course, so the persisted state
    // mirrors the request body exactly.
    sqlx::query!(
        "DELETE FROM course_identifiers WHERE course_id = $1",
        course.id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM course_links WHERE course_id = $1", course.id)
        .execute(&mut *conn)
        .await?;
    delete_course_collections(conn, course.id).await?;
    insert_identifiers(conn, course.id, &course.identifiers).await?;
    insert_links(conn, course.id, &course.links).await?;
    insert_course_collections(conn, course).await?;
    Ok(())
}

/// Soft-delete one course on `conn` (the plain soft delete): flip `active`
/// off and stamp `deleted_at` / `updated_at`, leaving every other column —
/// the stored digests included — in place.
async fn mark_course_deleted(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE courses SET active = false, deleted_at = $2, updated_at = $3 WHERE id = $1",
        id,
        Utc::now(),
        Utc::now(),
    )
    .execute(conn)
    .await?;
    // As under the ORM, an update that touches no row is an error.
    if done.rows_affected() == 0 {
        return Err(crate::Error::Database(
            "course not deleted: no such row".into(),
        ));
    }
    Ok(())
}

/// Soft-delete the course `id` on `conn` by stamping `deleted_at` /
/// `updated_at` and flipping `active` off, leaving every other column in
/// place. Factored so [`CourseRepository::merge`] can reuse it on its own
/// transaction (a partial UPDATE).
async fn apply_course_soft_delete(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    let now = Utc::now();
    // Clear the digests rather than leave ones that no longer describe
    // the row. This is a partial update — the record is not loaded here,
    // and reassembling it would add several queries to a path `merge`
    // also uses — so recomputing is not available.
    //
    // The consequence is stated rather than hidden: a soft-deleted
    // course reports as `unhashed`, not as verified and not as tampered.
    // That is the honest reading, and it avoids the worse outcome of
    // every soft delete surfacing as a false finding.
    let done = sqlx::query!(
        "UPDATE courses SET
             active = false, deleted_at = $2, updated_at = $2,
             content_hash = NULL, content_hash_sha3 = NULL, content_mac = NULL
         WHERE id = $1",
        id,
        now,
    )
    .execute(conn)
    .await?;
    // As under the ORM, an update that touches no row is an error.
    if done.rows_affected() == 0 {
        return Err(crate::Error::Database(
            "course not deleted: no such row".into(),
        ));
    }
    Ok(())
}

/// Pull an instance's schedule start date for FR-10 in-memory sorting.
fn schedule_start(i: &CourseInstance) -> Option<chrono::DateTime<chrono::Utc>> {
    i.schedule.as_ref().and_then(|s| s.start_date)
}

// ────────────────── Domain ↔ DB conversion ──────────────────

/// Build the full `courses` row from a domain [`Course`], integrity
/// digests included. On update, `updated_at` is stamped to now; on insert
/// the course's own timestamp is kept.
fn course_row(course: &Course, is_update: bool) -> Result<CourseRow> {
    let now = Utc::now();
    // All three digests from one call over the **assembled** record, so
    // the child collections are covered too. Computed here rather than
    // stamped afterwards, so one statement writes the row and its
    // integrity values together — and every full write builds its row
    // through this function.
    let digests = crate::compliance::record_integrity::digests(
        &crate::compliance::record_integrity::RecordInput {
            id: course.id,
            course,
            active: course.active,
        },
    );
    Ok(CourseRow {
        id: course.id,
        content_hash: Some(digests.sha256),
        content_hash_sha3: Some(digests.sha3),
        content_mac: digests.mac,
        name: course.name.clone(),
        description: course.description.clone(),
        disambiguating_description: course.disambiguating_description.clone(),
        url: course.url.clone(),
        additional_type: course.additional_type.clone(),
        audience: course.audience.clone(),
        license: course.license.clone(),
        typical_age_range: course.typical_age_range.clone(),
        time_required: course.time_required.clone(),
        version: course.version.clone(),
        is_accessible_for_free: course.is_accessible_for_free,
        educational_level: course
            .educational_level
            .as_ref()
            .map(enum_to_string)
            .transpose()?,
        educational_use: course.educational_use.clone(),
        learning_resource_type: course
            .learning_resource_type
            .as_ref()
            .map(enum_to_string)
            .transpose()?,
        interactivity_type: course
            .interactivity_type
            .as_ref()
            .map(enum_to_string)
            .transpose()?,
        course_code: course.course_code.clone(),
        number_of_credits: course
            .number_of_credits
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
        total_historical_enrollment: course
            .total_historical_enrollment
            .map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
        status: enum_to_string(&course.status)?,
        active: course.active,
        provider_id: course.provider_id,
        created_at: course.created_at,
        updated_at: if is_update { now } else { course.updated_at },
        deleted_at: course.deleted_at,
    })
}

/// `INSERT` one whole `courses` row.
async fn insert_course_row(conn: &mut PgConnection, row: &CourseRow) -> Result<()> {
    sqlx::query!(
        "INSERT INTO courses (
             id, name, content_hash, content_hash_sha3, content_mac, description,
             disambiguating_description, url, additional_type, audience, license,
             typical_age_range, time_required, version, is_accessible_for_free,
             educational_level, educational_use, learning_resource_type,
             interactivity_type, course_code, number_of_credits,
             total_historical_enrollment, status, active, provider_id,
             created_at, updated_at, deleted_at
         ) VALUES (
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
             $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28
         )",
        row.id,
        row.name,
        row.content_hash,
        row.content_hash_sha3,
        row.content_mac,
        row.description,
        row.disambiguating_description,
        row.url,
        row.additional_type,
        row.audience,
        row.license,
        row.typical_age_range,
        row.time_required,
        row.version,
        row.is_accessible_for_free,
        row.educational_level,
        row.educational_use,
        row.learning_resource_type,
        row.interactivity_type,
        row.course_code,
        row.number_of_credits,
        row.total_historical_enrollment,
        row.status,
        row.active,
        row.provider_id,
        row.created_at,
        row.updated_at,
        row.deleted_at,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// `UPDATE` every column of one `courses` row except `id`, `created_at`
/// included, exactly as the ORM's full-row update did.
async fn update_course_row(conn: &mut PgConnection, row: &CourseRow) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE courses SET
             name = $2, content_hash = $3, content_hash_sha3 = $4, content_mac = $5,
             description = $6, disambiguating_description = $7, url = $8,
             additional_type = $9, audience = $10, license = $11, typical_age_range = $12,
             time_required = $13, version = $14, is_accessible_for_free = $15,
             educational_level = $16, educational_use = $17, learning_resource_type = $18,
             interactivity_type = $19, course_code = $20, number_of_credits = $21,
             total_historical_enrollment = $22, status = $23, active = $24,
             provider_id = $25, created_at = $26, updated_at = $27, deleted_at = $28
         WHERE id = $1",
        row.id,
        row.name,
        row.content_hash,
        row.content_hash_sha3,
        row.content_mac,
        row.description,
        row.disambiguating_description,
        row.url,
        row.additional_type,
        row.audience,
        row.license,
        row.typical_age_range,
        row.time_required,
        row.version,
        row.is_accessible_for_free,
        row.educational_level,
        row.educational_use,
        row.learning_resource_type,
        row.interactivity_type,
        row.course_code,
        row.number_of_credits,
        row.total_historical_enrollment,
        row.status,
        row.active,
        row.provider_id,
        row.created_at,
        row.updated_at,
        row.deleted_at,
    )
    .execute(conn)
    .await?;
    // A row that vanished is an error, as it was when the ORM reported
    // "no record updated".
    if done.rows_affected() == 0 {
        return Err(crate::Error::Database(
            "course not updated: no such row".into(),
        ));
    }
    Ok(())
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
async fn insert_course_collections(conn: &mut PgConnection, course: &Course) -> Result<()> {
    for (field, values) in course_text_fields(course) {
        for (i, value) in values.iter().enumerate() {
            sqlx::query!(
                "INSERT INTO course_text_values (id, course_id, field, value, position)
                 VALUES ($1, $2, $3, $4, $5)",
                Uuid::new_v4(),
                course.id,
                field,
                value,
                position(i),
            )
            .execute(&mut *conn)
            .await?;
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
async fn insert_credential(
    conn: &mut PgConnection,
    course_id: Uuid,
    role: &str,
    cred: &crate::models::EducationalCredential,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO course_credentials
             (id, course_id, role, name, category, educational_level, recognized_by, url)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        Uuid::new_v4(),
        course_id,
        role,
        cred.name,
        cred.category.as_ref().map(enum_to_string).transpose()?,
        cred.educational_level,
        cred.recognized_by,
        cred.url,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Delete the `course_text_values` + `course_credentials` rows for a course.
async fn delete_course_collections(conn: &mut PgConnection, course_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM course_text_values WHERE course_id = $1",
        course_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM course_credentials WHERE course_id = $1",
        course_id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Load the `course_text_values` + `course_credentials` rows for a course.
async fn load_course_collections(
    db: &PgPool,
    course_id: Uuid,
) -> Result<(Vec<CourseTextValueRow>, Vec<CourseCredentialRow>)> {
    let text_values = sqlx::query_as!(
        CourseTextValueRow,
        "SELECT field, value FROM course_text_values WHERE course_id = $1 ORDER BY position",
        course_id,
    )
    .fetch_all(db)
    .await?;
    let credentials = sqlx::query_as!(
        CourseCredentialRow,
        "SELECT role, name, category, educational_level, recognized_by, url
         FROM course_credentials
         WHERE course_id = $1",
        course_id,
    )
    .fetch_all(db)
    .await?;
    Ok((text_values, credentials))
}

/// Collect the values of one tagged field from loaded text-value rows.
fn texts(rows: &[CourseTextValueRow], field: &str) -> Vec<String> {
    rows.iter()
        .filter(|r| r.field == field)
        .map(|r| r.value.clone())
        .collect()
}

/// Reconstruct an [`EducationalCredential`] from a row, or `None`.
fn credential_of(
    rows: &[CourseCredentialRow],
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
/// child identifier/link/text-value/credential collections.
/// Sub-resources (`syllabus_sections`, `instances`) are left empty —
/// they load via their own endpoints.
fn hydrate_course(
    row: CourseRow,
    identifiers: Vec<CourseIdentifier>,
    links: Vec<CourseLink>,
    text_values: &[CourseTextValueRow],
    credentials: &[CourseCredentialRow],
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
        deleted_at: row.deleted_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

// ────────────────── Instance round-trip ──────────────────

/// Build the full `course_instances` row from a domain
/// [`CourseInstance`], flattening its schedule onto the row. On update,
/// `updated_at` is stamped to now; on insert the instance's own timestamp
/// is kept. `deleted_at` is always written `NULL`.
fn instance_row(i: &CourseInstance, is_update: bool) -> Result<CourseInstanceRow> {
    let now = Utc::now();
    let sched = i.schedule.as_ref();
    Ok(CourseInstanceRow {
        id: i.id,
        course_id: i.course_id,
        name: i.name.clone(),
        course_mode: i.course_mode.as_ref().map(enum_to_string).transpose()?,
        status: enum_to_string(&i.status)?,
        location: i.location.clone(),
        location_id: i.location_id,
        maximum_attendee_capacity: i
            .maximum_attendee_capacity
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
        enrolled_count: i
            .enrolled_count
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
        enrollment_opens: i.enrollment_opens,
        enrollment_closes: i.enrollment_closes,
        schedule_start_date: sched.and_then(|s| s.start_date),
        schedule_end_date: sched.and_then(|s| s.end_date),
        schedule_time_zone: sched.and_then(|s| s.time_zone.clone()),
        schedule_recurrence: sched.and_then(|s| s.recurrence.clone()),
        created_at: i.created_at,
        updated_at: if is_update { now } else { i.updated_at },
        deleted_at: None,
    })
}

/// `INSERT` one whole `course_instances` row.
async fn insert_instance_row(conn: &mut PgConnection, row: &CourseInstanceRow) -> Result<()> {
    sqlx::query!(
        "INSERT INTO course_instances (
             id, course_id, name, course_mode, status, location, location_id,
             maximum_attendee_capacity, enrolled_count, enrollment_opens, enrollment_closes,
             schedule_start_date, schedule_end_date, schedule_time_zone, schedule_recurrence,
             created_at, updated_at, deleted_at
         ) VALUES (
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
         )",
        row.id,
        row.course_id,
        row.name,
        row.course_mode,
        row.status,
        row.location,
        row.location_id,
        row.maximum_attendee_capacity,
        row.enrolled_count,
        row.enrollment_opens,
        row.enrollment_closes,
        row.schedule_start_date,
        row.schedule_end_date,
        row.schedule_time_zone,
        row.schedule_recurrence,
        row.created_at,
        row.updated_at,
        row.deleted_at,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// `UPDATE` every column of one `course_instances` row except `id`,
/// `created_at` and `course_id` included, exactly as the ORM's full-row
/// update did.
async fn update_instance_row(conn: &mut PgConnection, row: &CourseInstanceRow) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE course_instances SET
             course_id = $2, name = $3, course_mode = $4, status = $5, location = $6,
             location_id = $7, maximum_attendee_capacity = $8, enrolled_count = $9,
             enrollment_opens = $10, enrollment_closes = $11, schedule_start_date = $12,
             schedule_end_date = $13, schedule_time_zone = $14, schedule_recurrence = $15,
             created_at = $16, updated_at = $17, deleted_at = $18
         WHERE id = $1",
        row.id,
        row.course_id,
        row.name,
        row.course_mode,
        row.status,
        row.location,
        row.location_id,
        row.maximum_attendee_capacity,
        row.enrolled_count,
        row.enrollment_opens,
        row.enrollment_closes,
        row.schedule_start_date,
        row.schedule_end_date,
        row.schedule_time_zone,
        row.schedule_recurrence,
        row.created_at,
        row.updated_at,
        row.deleted_at,
    )
    .execute(conn)
    .await?;
    // As under the ORM, an update that touches no row is an error.
    if done.rows_affected() == 0 {
        return Err(crate::Error::Database(
            "course instance not updated: no such row".into(),
        ));
    }
    Ok(())
}

/// Insert an instance's normalized child collections (languages,
/// instructors, sessions) on connection `conn`.
async fn insert_instance_collections(conn: &mut PgConnection, i: &CourseInstance) -> Result<()> {
    for (idx, lang) in i.in_language.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO course_instance_languages (id, instance_id, language, position)
             VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            i.id,
            lang,
            position(idx),
        )
        .execute(&mut *conn)
        .await?;
    }
    // Instructor ids and names are parallel lists in the domain model; emit
    // one row per position, carrying whichever of (id, name) is present.
    let n = i.instructor_ids.len().max(i.instructor_names.len());
    for idx in 0..n {
        sqlx::query!(
            "INSERT INTO course_instance_instructors
                 (id, instance_id, instructor_id, instructor_name, position)
             VALUES ($1, $2, $3, $4, $5)",
            Uuid::new_v4(),
            i.id,
            i.instructor_ids.get(idx).copied(),
            i.instructor_names.get(idx),
            position(idx),
        )
        .execute(&mut *conn)
        .await?;
    }
    if let Some(s) = &i.schedule {
        for (idx, session) in s.sessions.iter().enumerate() {
            sqlx::query!(
                "INSERT INTO course_instance_sessions
                     (id, instance_id, start_at, end_at, label, position)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                Uuid::new_v4(),
                i.id,
                session.start,
                session.end,
                session.label,
                position(idx),
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

/// Delete an instance's normalized child collections on connection `conn`.
async fn delete_instance_collections(conn: &mut PgConnection, instance_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM course_instance_languages WHERE instance_id = $1",
        instance_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM course_instance_instructors WHERE instance_id = $1",
        instance_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM course_instance_sessions WHERE instance_id = $1",
        instance_id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Reconstruct a domain [`CourseInstance`] from its row plus its child
/// collections, widening the stored `i32` counts back to `u32`.
async fn hydrate_instance(db: &PgPool, row: CourseInstanceRow) -> Result<CourseInstance> {
    let id = row.id;

    let in_language = sqlx::query_scalar!(
        "SELECT language FROM course_instance_languages WHERE instance_id = $1 ORDER BY position",
        id,
    )
    .fetch_all(db)
    .await?;

    let inst_rows = sqlx::query!(
        "SELECT instructor_id, instructor_name
         FROM course_instance_instructors
         WHERE instance_id = $1
         ORDER BY position",
        id,
    )
    .fetch_all(db)
    .await?;
    let instructor_ids = inst_rows.iter().filter_map(|r| r.instructor_id).collect();
    let instructor_names = inst_rows
        .into_iter()
        .filter_map(|r| r.instructor_name)
        .collect();

    let sessions: Vec<crate::models::course_instance::Session> = sqlx::query!(
        "SELECT start_at, end_at, label
         FROM course_instance_sessions
         WHERE instance_id = $1
         ORDER BY position",
        id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|s| crate::models::course_instance::Session {
        start: s.start_at,
        end: s.end_at,
        label: s.label,
    })
    .collect();

    let has_schedule = row.schedule_start_date.is_some()
        || row.schedule_end_date.is_some()
        || row.schedule_time_zone.is_some()
        || row.schedule_recurrence.is_some()
        || !sessions.is_empty();
    let schedule = has_schedule.then(|| Schedule {
        start_date: row.schedule_start_date,
        end_date: row.schedule_end_date,
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
        enrollment_opens: row.enrollment_opens,
        enrollment_closes: row.enrollment_closes,
        schedule,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

// ────────────────── Child-table round-trip ──────────────────

/// Insert each [`CourseIdentifier`] as a `course_identifiers` child row
/// under `course_id`, on the caller's connection so it runs inside a
/// transaction.
async fn insert_identifiers(
    conn: &mut PgConnection,
    course_id: Uuid,
    identifiers: &[CourseIdentifier],
) -> Result<()> {
    for (i, ident) in identifiers.iter().enumerate() {
        let (property_id, custom_label) = ident_type_parts(&ident.property_id)?;
        sqlx::query!(
            "INSERT INTO course_identifiers
                 (id, course_id, property_id, custom_label, value, name, url, position, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            Uuid::new_v4(),
            course_id,
            property_id,
            custom_label,
            ident.value,
            ident.name,
            ident.url,
            position(i),
            Utc::now(),
        )
        .execute(&mut *conn)
        .await?;
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
/// `course_id`, on the caller's connection so it runs in a transaction.
async fn insert_links(
    conn: &mut PgConnection,
    course_id: Uuid,
    links: &[CourseLink],
) -> Result<()> {
    for link in links {
        sqlx::query!(
            "INSERT INTO course_links (id, course_id, other_course_id, link_type, created_at)
             VALUES ($1, $2, $3, $4, $5)",
            Uuid::new_v4(),
            course_id,
            link.other_course_id,
            enum_to_string(&link.link_type)?,
            Utc::now(),
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Load and deserialise all `course_identifiers` rows for `course_id`.
async fn load_identifiers(db: &PgPool, course_id: Uuid) -> Result<Vec<CourseIdentifier>> {
    sqlx::query!(
        "SELECT property_id, custom_label, value, name, url
         FROM course_identifiers
         WHERE course_id = $1
         ORDER BY position",
        course_id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
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
async fn load_links(db: &PgPool, course_id: Uuid) -> Result<Vec<CourseLink>> {
    sqlx::query!(
        "SELECT other_course_id, link_type FROM course_links WHERE course_id = $1",
        course_id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|r| {
        Ok(CourseLink {
            other_course_id: r.other_course_id,
            link_type: enum_from_string::<LinkType>(&r.link_type)?,
        })
    })
    .collect()
}

// ────────────────── Helpers ──────────────────

/// A list index as the `INTEGER` `position` column.
fn position(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(i32::MAX)
}

/// A `LIMIT` / `OFFSET` bound: Postgres takes `BIGINT`, the callers count
/// in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
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

    /// `course_row` copies scalar fields onto the `courses` row.
    #[test]
    fn course_row_carries_all_scalar_fields() {
        let mut course = Course::new("Intro to CS");
        course.course_code = Some("CS101".into());
        course.number_of_credits = Some(3);
        course.educational_level = Some(EducationalLevel::Undergraduate);
        course.keywords = vec!["programming".into(), "algorithms".into()];
        let row = course_row(&course, false).unwrap();
        assert!(matches!(row.name, ref n if n == "Intro to CS"));
        assert!(matches!(row.course_code, Some(ref c) if c == "CS101"));
        assert!(matches!(row.number_of_credits, Some(3)));
        assert!(matches!(row.status, ref s if s == "published"));
    }
}

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path.
/// They require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped
/// by a bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod outbox_atomicity_tests {
    use super::{CourseRepository, PgCourseRepository};
    use crate::db::outbox;
    use crate::models::Course;
    use crate::streaming::EventTransport;

    async fn connect() -> sqlx::PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sqlx::PgPool::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Create writes a `created` outbox row in the same transaction as the
    /// entity insert (under the outbox transport).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = PgCourseRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let course = repo.create(&Course::new("Intro to CS")).await.unwrap();

        let rows = outbox::rows_for(&db, course.id, "created").await.unwrap();
        assert_eq!(rows.len(), 1, "one created outbox row for the new course");
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = PgCourseRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo.create(&Course::new("Survivor")).await.unwrap();
        let duplicate = repo.create(&Course::new("Duplicate")).await.unwrap();

        let merged = repo.merge(&survivor, &duplicate.id).await.unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = outbox::rows_for(&db, survivor.id, "merged").await.unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = outbox::rows_for(&db, duplicate.id, "deleted")
            .await
            .unwrap();
        assert_eq!(
            deleted_rows.len(),
            1,
            "one deleted outbox row for duplicate"
        );
    }
}
