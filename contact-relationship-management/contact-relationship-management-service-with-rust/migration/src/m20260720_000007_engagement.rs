//! Migration: the stakeholder-engagement / innovation-partnership /
//! confederation-collaboration columns and tables — declared
//! stakeholder typing on contacts and accounts (role +
//! power–interest, all nullable: undeclared stays undeclared),
//! recorded interaction sentiment on activities, and the
//! `partnerships` / `memberships` / `working_groups` sub-resources.

use sea_orm_migration::prelude::*;

/// The engagement migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns; create the three tables (+ member join table).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE contacts ADD COLUMN IF NOT EXISTS stakeholder_role VARCHAR NULL;
                 ALTER TABLE contacts ADD COLUMN IF NOT EXISTS influence INTEGER NULL;
                 ALTER TABLE contacts ADD COLUMN IF NOT EXISTS interest INTEGER NULL;
                 ALTER TABLE accounts ADD COLUMN IF NOT EXISTS stakeholder_role VARCHAR NULL;
                 ALTER TABLE activities ADD COLUMN IF NOT EXISTS sentiment VARCHAR NULL;
                 CREATE TABLE IF NOT EXISTS partnerships (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     account_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL,
                     stage VARCHAR NOT NULL DEFAULT 'scouting',
                     summary VARCHAR NOT NULL,
                     started_on DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS partnerships_account
                     ON partnerships (account_pid);
                 CREATE TABLE IF NOT EXISTS memberships (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     account_pid UUID NOT NULL UNIQUE,
                     joined_on DATE NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'active',
                     renewal_on DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE TABLE IF NOT EXISTS working_groups (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL,
                     purpose VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE TABLE IF NOT EXISTS working_group_members (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     group_pid UUID NOT NULL,
                     contact_pid UUID NOT NULL,
                     UNIQUE (group_pid, contact_pid)
                 );",
            )
            .await?;
        Ok(())
    }

    /// Drop the tables + columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS working_group_members;
                 DROP TABLE IF EXISTS working_groups;
                 DROP TABLE IF EXISTS memberships;
                 DROP TABLE IF EXISTS partnerships;
                 ALTER TABLE activities DROP COLUMN IF EXISTS sentiment;
                 ALTER TABLE accounts DROP COLUMN IF EXISTS stakeholder_role;
                 ALTER TABLE contacts DROP COLUMN IF EXISTS interest;
                 ALTER TABLE contacts DROP COLUMN IF EXISTS influence;
                 ALTER TABLE contacts DROP COLUMN IF EXISTS stakeholder_role;",
            )
            .await?;
        Ok(())
    }
}
