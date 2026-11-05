//! Migration: tamper-evident audit history for `audit_log`.
//!
//! Adds the hash-chain columns (`seq`, `prev_hash`, `hash`) plus the
//! read/disclosure columns (`context`, `disclosure`, `redacted_at`),
//! adopting the care-pathway reference implementation per
//! [`spec/compliance` §8.5](../../../../spec/compliance/index.md) step 3.
//!
//! Wraps the hand-written SQL under
//! `../../migrations/2026072600000001_audit_chain/{up,down}.sql` via
//! `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The audit-chain migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260726_000001_audit_chain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the chain and disclosure columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000001_audit_chain/up.sql"
            ))
            .await?;
        Ok(())
    }

    /// Drop them again (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000001_audit_chain/down.sql"
            ))
            .await?;
        Ok(())
    }
}
