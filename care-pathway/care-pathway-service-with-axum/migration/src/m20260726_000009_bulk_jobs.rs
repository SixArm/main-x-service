//! Migration: create the `bulk_jobs` table — durable state for
//! asynchronous bulk operations.
//!
//! Adopts the family schema in
//! [`agents/share/bulk-import-export.md`](../../../../agents/share/bulk-import-export.md)
//! §3, matching the person service's existing table so the two do not
//! drift. Care-pathway's first consumer is the FHIR Bulk Data `$export`
//! operation, which until now kept its jobs in a process-local registry:
//! they did not survive a restart and were invisible to another replica,
//! so a client polling through a load balancer could get a `404` for a
//! job that had genuinely succeeded.
//!
//! The import-side columns (`rows_created` / `rows_upserted` /
//! `rows_to_review` / `rows_errored`, `input_url`, `error_report_url`) are
//! created now even though only export is implemented, so the import half
//! (§13 T-10) needs no second migration and both halves share one shape.

use sea_orm_migration::prelude::*;

/// The `bulk_jobs` migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column identifiers for the `bulk_jobs` table.
#[derive(DeriveIden)]
enum BulkJobs {
    /// The table itself.
    Table,
    /// Job id (also the `$export-status` path segment).
    Id,
    /// `import` | `export`.
    Kind,
    /// The entity the job operates on.
    Entity,
    /// Output/input format (`ndjson` for FHIR Bulk Data).
    Format,
    /// `queued` | `running` | `completed` | `completed_with_errors` | `failed` | `cancelled`.
    Status,
    /// Request parameters, kept so a job is interpretable after the fact.
    Params,
    /// Rows the job expects to handle, once known.
    RowsTotal,
    /// Rows handled so far — the Bulk Data `X-Progress` source.
    RowsProcessed,
    /// Import: rows created.
    RowsCreated,
    /// Import: rows upserted onto an existing record.
    RowsUpserted,
    /// Import: rows routed to the duplicate-review queue.
    RowsToReview,
    /// Rows that failed.
    RowsErrored,
    /// The submitting caller's `sub`, when a verified token was presented.
    Actor,
    /// Client-supplied key that makes a retried submit return the same job.
    IdempotencyKey,
    /// Reference to the uploaded input artifact (import).
    InputUrl,
    /// Reference to the output artifact (export).
    ResultUrl,
    /// Reference to the per-row error report.
    ErrorReportUrl,
    /// Free-text failure reason when `status = failed`.
    Error,
    /// When the job was submitted.
    CreatedAt,
    /// When the job last changed state.
    UpdatedAt,
    /// When the job row and its artifacts may be swept.
    ExpiresAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table plus the status and idempotency indexes.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(BulkJobs::Table)
                .if_not_exists()
                .col(ColumnDef::new(BulkJobs::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(BulkJobs::Kind).string().not_null())
                .col(ColumnDef::new(BulkJobs::Entity).string().not_null())
                .col(ColumnDef::new(BulkJobs::Format).string().not_null())
                .col(ColumnDef::new(BulkJobs::Status).string().not_null())
                .col(ColumnDef::new(BulkJobs::Params).json_binary().not_null())
                .col(ColumnDef::new(BulkJobs::RowsTotal).big_integer().null())
                .col(
                    ColumnDef::new(BulkJobs::RowsProcessed)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .col(
                    ColumnDef::new(BulkJobs::RowsCreated)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .col(
                    ColumnDef::new(BulkJobs::RowsUpserted)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .col(
                    ColumnDef::new(BulkJobs::RowsToReview)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .col(
                    ColumnDef::new(BulkJobs::RowsErrored)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .col(ColumnDef::new(BulkJobs::Actor).string().null())
                .col(ColumnDef::new(BulkJobs::IdempotencyKey).string().null())
                .col(ColumnDef::new(BulkJobs::InputUrl).string().null())
                .col(ColumnDef::new(BulkJobs::ResultUrl).string().null())
                .col(ColumnDef::new(BulkJobs::ErrorReportUrl).string().null())
                .col(ColumnDef::new(BulkJobs::Error).string().null())
                .col(
                    ColumnDef::new(BulkJobs::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(BulkJobs::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(BulkJobs::ExpiresAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        // A retried submit must return the same job rather than starting a
        // second export of the same data (shared doc §3). Partial, so the
        // many jobs with no key do not collide with each other.
        m.create_index(
            Index::create()
                .name("bulk_jobs_idempotency_idx")
                .table(BulkJobs::Table)
                .col(BulkJobs::Entity)
                .col(BulkJobs::Kind)
                .col(BulkJobs::IdempotencyKey)
                .unique()
                .to_owned(),
        )
        .await?;

        // The status poll and the retention sweep both filter on status.
        m.create_index(
            Index::create()
                .name("bulk_jobs_status_idx")
                .table(BulkJobs::Table)
                .col(BulkJobs::Status)
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Drop the table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(BulkJobs::Table).to_owned())
            .await?;
        Ok(())
    }
}
