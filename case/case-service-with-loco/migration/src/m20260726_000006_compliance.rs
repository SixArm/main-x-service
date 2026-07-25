//! Migration: extend `audit_logs` with the compliance columns —
//! the tamper-evident **hash chain** (`prev_hash` / `hash`), the
//! per-access **context** (purpose-of-use, residency, lawful basis),
//! the **disclosure** flag that separates an internal access from an
//! outward disclosure (HIPAA §164.528 accounting), and `redacted_at`,
//! the GDPR Art. 17 marker for a row whose content has been destroyed
//! while its chain linkage survives.
//!
//! All added columns are nullable (or defaulted), so rows written before
//! this migration remain valid — chain verification reports them as
//! `unchained` rather than as a break. See
//! `agents/share/compliance-for-healthcare.md` §2.1–§2.2.

use sea_orm_migration::prelude::*;

/// The compliance-columns migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column identifiers for the `audit_logs` table touched here.
#[derive(DeriveIden)]
enum AuditLogs {
    /// The table itself.
    Table,
    /// The care-pathway `pid` an entry concerns (indexed here).
    EntityPid,
    /// Hash of the preceding chain row (`NULL` for the genesis row).
    PrevHash,
    /// This row's content hash — the chain link successors bind to.
    Hash,
    /// Request/processing context: purpose-of-use, residency, lawful basis.
    Context,
    /// Whether this access was an outward **disclosure** (§164.528).
    Disclosure,
    /// When the row's content was destroyed under GDPR Art. 17.
    RedactedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the compliance columns and the per-entity audit index.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(AuditLogs::Table)
                .add_column(ColumnDef::new(AuditLogs::PrevHash).string().null())
                .add_column(ColumnDef::new(AuditLogs::Hash).string().null())
                .add_column(ColumnDef::new(AuditLogs::Context).json_binary().null())
                .add_column(
                    ColumnDef::new(AuditLogs::Disclosure)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .add_column(
                    ColumnDef::new(AuditLogs::RedactedAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .to_owned(),
        )
        .await?;
        // Per-record audit + disclosure queries (`/{pid}/audit`,
        // `/{pid}/disclosures`, and the Art. 17 redaction sweep) all filter
        // on `entity_pid`; without this they are sequential scans.
        m.create_index(
            Index::create()
                .name("audit_logs_entity_pid_idx")
                .table(AuditLogs::Table)
                .col(AuditLogs::EntityPid)
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Drop the index and the compliance columns (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("audit_logs_entity_pid_idx")
                .table(AuditLogs::Table)
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(AuditLogs::Table)
                .drop_column(AuditLogs::PrevHash)
                .drop_column(AuditLogs::Hash)
                .drop_column(AuditLogs::Context)
                .drop_column(AuditLogs::Disclosure)
                .drop_column(AuditLogs::RedactedAt)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
