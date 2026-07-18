//! Migration: the support tables (CRM-R10--R12) -- `sla_policies`, `tickets`, `articles`. Explicit SQL (family lesson: the loco
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
            "CREATE TABLE IF NOT EXISTS sla_policies (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 priority VARCHAR NOT NULL,
                 first_response_minutes INTEGER NOT NULL,
                 resolution_minutes INTEGER NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS tickets (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 contact_pid UUID NULL,
                 account_pid UUID NULL,
                 assignee_ref VARCHAR NULL,
                 title VARCHAR NOT NULL,
                 priority VARCHAR NOT NULL,
                 channel VARCHAR NOT NULL,
                 status VARCHAR NOT NULL,
                 opened_at TIMESTAMPTZ NOT NULL,
                 first_response_due_at TIMESTAMPTZ NULL,
                 resolution_due_at TIMESTAMPTZ NULL,
                 first_responded_at TIMESTAMPTZ NULL,
                 resolved_at TIMESTAMPTZ NULL,
                 first_response_breached BOOLEAN NOT NULL DEFAULT FALSE,
                 resolution_breached BOOLEAN NOT NULL DEFAULT FALSE,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS articles (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 title VARCHAR NOT NULL,
                 body VARCHAR NOT NULL,
                 keywords VARCHAR NULL,
                 status VARCHAR NOT NULL,
                 version INTEGER NOT NULL DEFAULT 1,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS tickets_status ON tickets (status, priority)",
        )
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
        for table in ["articles", "tickets", "sla_policies"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
