//! Migration: drop the BLAKE3 companion digests.
//!
//! BLAKE3 was added as a second digest for speed and algorithm agility,
//! then removed: it is **not FIPS/NIST approved**, and these services are
//! built for regimes that require an approved hash. A digest that cannot
//! be named in a control document costs a column and a hash pass per
//! write while contributing nothing an auditor may rely on.
//!
//! The integrity property is unchanged. SHA-256 (FIPS 180-4) and SHA-3
//! (FIPS 202) both remain, and they are the better pair for the
//! structural-diversity argument anyway — Merkle-Damgård against sponge,
//! two unrelated design families, both approved.
//!
//! The columns are **dropped** rather than left in place: a digest column
//! nothing maintains is worse than no column, because it reads as
//! coverage that does not exist while its values rot from the first write
//! after this migration. Rollback restores the columns but not their
//! values — recomputing them from current content would certify whatever
//! that content now is, which is the claim they existed to test.

use sea_orm_migration::prelude::*;

/// The drop-BLAKE3 migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Columns dropped from `audit_logs`.
#[derive(DeriveIden)]
enum AuditLogs {
    /// The table itself.
    Table,
    /// BLAKE3 digest of the preceding chain row.
    PrevHashBlake3,
    /// This row's BLAKE3 digest.
    HashBlake3,
}

/// Columns dropped from `care_pathways`.
#[derive(DeriveIden)]
enum CarePathways {
    /// The table itself.
    Table,
    /// BLAKE3 digest over the record's content.
    ContentHashBlake3,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Drop the columns.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(AuditLogs::Table)
                .drop_column(AuditLogs::PrevHashBlake3)
                .drop_column(AuditLogs::HashBlake3)
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(CarePathways::Table)
                .drop_column(CarePathways::ContentHashBlake3)
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Recreate them, empty (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(AuditLogs::Table)
                .add_column_if_not_exists(ColumnDef::new(AuditLogs::PrevHashBlake3).string().null())
                .add_column_if_not_exists(ColumnDef::new(AuditLogs::HashBlake3).string().null())
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(CarePathways::Table)
                .add_column_if_not_exists(
                    ColumnDef::new(CarePathways::ContentHashBlake3)
                        .string()
                        .null(),
                )
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
