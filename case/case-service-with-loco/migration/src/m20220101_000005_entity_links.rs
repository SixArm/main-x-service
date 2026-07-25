//! Migration: create the `entity_links` table — the **write side** of
//! cross-service entity linking
//! (`agents/share/cross-service-linking.md` §4.1). Each row is one
//! **outbound** edge this service originates from a case: for v1 the
//! `subject_of` (case → person) edge, the highest-governance v1 kind
//! (§9, §10). The write is optimistic — it stores the assertion and emits
//! a `linked` event; it never calls the target service, and verification
//! is the read-model aggregator's concern.

use loco_rs::schema::{create_table, drop_table, ColType};
use sea_orm_migration::prelude::*;

/// The `entity_links`-table migration. `DeriveMigrationName` derives the
/// name from the timestamped module path.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `entity_links` table plus the idempotent-upsert unique
    /// index on `(from_pid, kind, to_ref, valid_from)`.
    ///
    /// The index is declared `NULLS NOT DISTINCT` so a `NULL` `valid_from`
    /// (a non-temporal edge) still collides on re-assertion — otherwise
    /// Postgres treats each `NULL` as distinct and the upsert would insert
    /// duplicates. This is what makes the write idempotent.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "entity_links",
            &[
                // The edge id (a UUID; also the `event`'s `edge_id`). Set
                // by the app, not auto-increment.
                ("id", ColType::PkUuid),
                // This service's originating case record (its `pid`).
                ("from_pid", ColType::Uuid),
                // The edge kind token (§9 registry), e.g. `subject_of`.
                ("kind", ColType::Text),
                // The far record's `EntityRef` URN, e.g. `person:<uuid>`.
                ("to_ref", ColType::Text),
                // Optional role label (unused by `subject_of`).
                ("role", ColType::TextNull),
                // Optional confidence: 1.0 operator-asserted, <1 suggested.
                ("confidence", ColType::DoubleNull),
                // How the edge arose: operator | import | matcher_suggested.
                ("provenance", ColType::Text),
                // Affiliation start (nullable — `subject_of` is temporal).
                ("valid_from", ColType::DateNull),
                // Affiliation end ("former subject of" once past).
                ("valid_to", ColType::DateNull),
                // Soft-delete timestamp (a withdrawn edge); null while live.
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        // Idempotent-upsert key (§4.1). `NULLS NOT DISTINCT` so a null
        // `valid_from` still conflicts on re-assertion (Postgres 15+).
        m.get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS entity_links_upsert \
                 ON entity_links (from_pid, kind, to_ref, valid_from) \
                 NULLS NOT DISTINCT",
            )
            .await?;
        // Inverse-lookup / list-active support.
        m.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS entity_links_from \
                 ON entity_links (from_pid) WHERE deleted_at IS NULL",
            )
            .await?;
        Ok(())
    }

    /// Drop the `entity_links` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "entity_links").await?;
        Ok(())
    }
}
