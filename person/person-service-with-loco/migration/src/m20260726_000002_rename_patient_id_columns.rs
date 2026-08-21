//! Migration: rename the leftover `patient_id` foreign-key columns to
//! `person_id`.
//!
//! `m20260603_000001_rename_patient_tables_to_person` renamed the tables
//! but not their FK columns, so the schema kept `patient_id` while the
//! `SeaORM` entities declare `person_id` — every insert into a person child
//! table failed. Wraps the hand-written SQL under
//! `../../migrations/2026072600000002_rename_patient_id_columns/{up,down}.sql`,
//! matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The FK-column rename migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260726_000002_rename_patient_id_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Rename `patient_id` → `person_id` (and `other_patient_id` →
    /// `other_person_id`) wherever the old name survives.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000002_rename_patient_id_columns/up.sql"
            ))
            .await?;
        Ok(())
    }

    /// Rename them back (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000002_rename_patient_id_columns/down.sql"
            ))
            .await?;
        Ok(())
    }
}
