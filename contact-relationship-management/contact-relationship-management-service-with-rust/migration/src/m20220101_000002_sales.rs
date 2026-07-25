//! Migration: the sales tables (CRM-R3--R5) -- `leads`, `pipelines` + `pipeline_stages`, `deals`, `forecast_snapshots`. Explicit SQL (family lesson: the loco
//! `create_table` helper pluralizes names).

use sea_orm_migration::prelude::*;

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables + lookup indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS leads (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 source VARCHAR NOT NULL,
                 campaign_pid UUID NULL,
                 contact_pid UUID NULL,
                 display_name VARCHAR NOT NULL,
                 email VARCHAR NULL,
                 email_domain VARCHAR NULL,
                 score INTEGER NOT NULL DEFAULT 0,
                 campaign_click BOOLEAN NOT NULL DEFAULT FALSE,
                 unsubscribed BOOLEAN NOT NULL DEFAULT FALSE,
                 status VARCHAR NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS pipelines (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS pipeline_stages (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 pipeline_pid UUID NOT NULL,
                 name VARCHAR NOT NULL,
                 position INTEGER NOT NULL,
                 probability_percent INTEGER NOT NULL,
                 is_won BOOLEAN NOT NULL DEFAULT FALSE,
                 is_lost BOOLEAN NOT NULL DEFAULT FALSE,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS deals (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 account_pid UUID NULL,
                 primary_contact_pid UUID NULL,
                 owner_ref VARCHAR NULL,
                 pipeline_pid UUID NOT NULL,
                 stage_pid UUID NOT NULL,
                 name VARCHAR NOT NULL,
                 amount_minor BIGINT NOT NULL,
                 currency VARCHAR NOT NULL,
                 expected_close_on DATE NULL,
                 kanban_position INTEGER NOT NULL DEFAULT 0,
                 source_campaign_pid UUID NULL,
                 closed_at TIMESTAMPTZ NULL,
                 won BOOLEAN NOT NULL DEFAULT FALSE,
                 lost_reason VARCHAR NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS forecast_snapshots (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 taken_on DATE NOT NULL,
                 totals JSONB NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS pipeline_stages_pipeline ON pipeline_stages (pipeline_pid, position)",
        )
        .await?;
        conn.execute_unprepared("CREATE INDEX IF NOT EXISTS deals_stage ON deals (stage_pid)")
            .await?;
        conn.execute_unprepared("CREATE INDEX IF NOT EXISTS deals_account ON deals (account_pid)")
            .await?;
        Ok(())
    }

    /// Drop the tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in [
            "forecast_snapshots",
            "deals",
            "pipeline_stages",
            "pipelines",
            "leads",
        ] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
