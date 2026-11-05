//! Migration: create the `entity_links` table — the **write side** of
//! cross-service entity linking
//! (`agents/share/cross-service-linking.md` §4.1) for the care-pathway
//! service.
//!
//! Each row is one **outbound** edge this service originates. The v1
//! kind here is `continues_as` (§9): one subject's journey passing from
//! a care-pathway instance into the next episode — another pathway
//! instance (a transfer), an inpatient stay, or a case. It is what lets
//! time-based analysis measure a journey that crosses a service
//! boundary instead of stopping at it.
//!
//! **The `from_pid` is a `pathway_instances.pid`, not a
//! `care_pathways.pid`.** A journey belongs to an enrolment, not to the
//! template — the template is a document, the instance is a patient's
//! passage through it. That is why `entity-ref` gained a
//! `care_pathway_instance` type rather than reusing `care_pathway`.
//!
//! The write is **optimistic**: it stores the assertion and emits a
//! `linked` event, never calling the target service. Verification is the
//! read-model aggregator's concern (§5).

use sea_orm_migration::prelude::*;

/// The `entity_links`-table migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table plus the idempotent-upsert unique index.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS entity_links (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id UUID PRIMARY KEY,
                     from_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL,
                     to_ref VARCHAR NOT NULL,
                     role VARCHAR NULL,
                     confidence DOUBLE PRECISION NULL,
                     provenance VARCHAR NOT NULL,
                     valid_from DATE NULL,
                     valid_to DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 -- The idempotent-upsert key (§4.1). NULLS NOT DISTINCT
                 -- so a null `valid_from` still collides on re-assertion:
                 -- Postgres otherwise treats each NULL as distinct and the
                 -- upsert would insert duplicates, which is exactly what
                 -- makes a retried write safe here.
                 CREATE UNIQUE INDEX IF NOT EXISTS entity_links_upsert
                     ON entity_links (from_pid, kind, to_ref, valid_from)
                     NULLS NOT DISTINCT;
                 CREATE INDEX IF NOT EXISTS entity_links_from
                     ON entity_links (from_pid) WHERE deleted_at IS NULL;
                 -- The aggregator's reconciliation pull walks by
                 -- creation order.
                 CREATE INDEX IF NOT EXISTS entity_links_created
                     ON entity_links (created_at) WHERE deleted_at IS NULL;",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS entity_links;")
            .await?;
        Ok(())
    }
}
