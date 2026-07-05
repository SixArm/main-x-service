//! Add `users.attributes` for ABAC authorization sourcing (shared
//! `agents/share/authorization-attributes.md` §6). A `JSONB NOT NULL
//! DEFAULT '{}'` string→strings map (e.g. `{"access": ["write"]}`) of the
//! subject attributes the auth service mints into the PASETO `attrs`
//! claim. Existing users default to `{}` — read-only under the family's
//! default policy — until an operator assigns attributes.

use sea_orm_migration::prelude::*;

/// The `users.attributes`-column migration (name from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the `users.attributes` column (`JSONB NOT NULL DEFAULT '{}'`).
    /// Written with sea-query directly (not the loco `add_column` helper)
    /// because the helper cannot express the non-null-with-default shape
    /// an already-populated table needs.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("users"))
                .add_column(
                    ColumnDef::new(Alias::new("attributes"))
                        .json_binary()
                        .not_null()
                        .default("{}"),
                )
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Remove the `users.attributes` column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("users"))
                .drop_column(Alias::new("attributes"))
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
