//! Migration: add the BLAKE3 companion digests.
//!
//! The family keeps **two** integrity digests over the same pre-image —
//! SHA-256 and BLAKE3 — rather than replacing one with the other. See
//! `spec/12-compliance.md` §12.4z for the full rationale; briefly:
//!
//! - **SHA-256 stays** because it is the conservative choice: FIPS 180-4,
//!   NIST-approved, and what a compliance reviewer expects to see. Some
//!   regimes name it explicitly.
//! - **BLAKE3 is added** because it is several times faster (SIMD +
//!   parallel tree hashing), and because holding a second, independent
//!   digest is **algorithm agility**: if either function is ever weakened,
//!   the already-stored history can still be verified under the other
//!   without a flag day and without rehashing anything.
//!
//! The chain gets a *parallel* chain (`prev_hash_blake3` / `hash_blake3`),
//! not merely a second digest of the same row: the BLAKE3 digest binds the
//! BLAKE3 predecessor, so neither chain's linkage depends on the other
//! algorithm's collision resistance. A second digest that bound the
//! SHA-256 predecessor would inherit SHA-256's weaknesses and defeat the
//! point.
//!
//! All columns are nullable and existing rows stay NULL. Back-filling
//! would compute digests from current content, asserting that the current
//! content is authentic — the claim these columns exist to test.
//! Verification reports them as `blake3_unhashed`, an honest gap.

use sea_orm_migration::prelude::*;

/// The BLAKE3-digest migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Columns added to `audit_logs`.
#[derive(DeriveIden)]
enum AuditLogs {
    /// The table itself.
    Table,
    /// BLAKE3 digest of the preceding chain row.
    PrevHashBlake3,
    /// This row's BLAKE3 digest — the link successors bind to.
    HashBlake3,
}

/// Columns added to `care_pathways`.
#[derive(DeriveIden)]
enum CarePathways {
    /// The table itself.
    Table,
    /// BLAKE3 digest over the record's content and lifecycle state.
    ContentHashBlake3,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
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

    /// Drop the columns (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
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
}
