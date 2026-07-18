//! Migration: the relationship layer (CRM-R1, CRM-R2) -- `contacts`, `accounts`, `activities`, and the append-only `consent_events`. Explicit SQL (family lesson: the loco
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
            "CREATE TABLE IF NOT EXISTS contacts (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 person_ref VARCHAR NOT NULL,
                 account_pid UUID NULL,
                 owner_ref VARCHAR NULL,
                 display_name VARCHAR NOT NULL,
                 status VARCHAR NOT NULL,
                 job_title VARCHAR NULL,
                 preferred_channel VARCHAR NOT NULL,
                 marketing_consent VARCHAR NOT NULL,
                 consent_changed_at TIMESTAMPTZ NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS accounts (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 organization_ref VARCHAR NOT NULL,
                 owner_ref VARCHAR NULL,
                 display_name VARCHAR NOT NULL,
                 tier VARCHAR NOT NULL,
                 industry VARCHAR NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS activities (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 subject_kind VARCHAR NOT NULL,
                 subject_pid UUID NOT NULL,
                 kind VARCHAR NOT NULL,
                 occurred_at TIMESTAMPTZ NOT NULL,
                 actor_ref VARCHAR NULL,
                 summary VARCHAR NOT NULL,
                 due_on DATE NULL,
                 done BOOLEAN NOT NULL DEFAULT FALSE,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS consent_events (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 contact_pid UUID NOT NULL,
                 action VARCHAR NOT NULL,
                 source VARCHAR NOT NULL,
                 occurred_at TIMESTAMPTZ NOT NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS contacts_account ON contacts (account_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS activities_subject ON activities (subject_kind, subject_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS consent_events_contact ON consent_events (contact_pid)",
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
        for table in ["consent_events", "activities", "accounts", "contacts"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
