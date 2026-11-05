//! Migration: add `auth_events.source_ip` (T-14).
//!
//! Companion to `m20260906_000001_sessions_source_ip`: the same
//! best-effort connecting-peer address, captured on every audit row
//! rather than only at session issuance. **Deliberately excluded** from
//! the keyed-integrity pre-image
//! (`crate::compliance::audit_integrity::AuditInput`/`AUDIT_MAC_VERSION`)
//! — that format has no per-row version marker, so widening it would
//! make every pre-existing row's stored digest unrecomputable and
//! report as tampered on the next `/api/compliance/audit/verify`, which
//! would be worse than the gap this migration closes. `source_ip` is
//! therefore metadata alongside the row, not content the MAC attests to
//! — the same posture the family gives `sessions.user_agent`, which
//! `auth_events` has no equivalent digest concern for since it carries
//! no MAC at all.

use sea_orm_migration::prelude::*;

/// The `auth_events.source_ip` migration (name derived from the file
/// name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column added to `auth_events`.
#[derive(DeriveIden)]
enum AuthEvents {
    /// The table itself.
    Table,
    /// Best-effort connecting-peer address captured when the row is written.
    SourceIp,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(AuthEvents::Table)
                .add_column_if_not_exists(ColumnDef::new(AuthEvents::SourceIp).string().null())
                .to_owned(),
        )
        .await
    }

    /// Drop the column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(AuthEvents::Table)
                .drop_column(AuthEvents::SourceIp)
                .to_owned(),
        )
        .await
    }
}
