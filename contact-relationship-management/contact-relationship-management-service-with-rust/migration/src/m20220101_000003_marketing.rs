//! Migration: the marketing tables (CRM-R6--R9) -- `segments`, `campaigns`, `nurture_sequences` / `nurture_steps` / `nurture_enrollments`. Explicit SQL (family lesson: the loco
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
            "CREATE TABLE IF NOT EXISTS segments (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 name VARCHAR NOT NULL,
                 filter JSONB NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS campaigns (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 kind VARCHAR NOT NULL,
                 name VARCHAR NOT NULL,
                 status VARCHAR NOT NULL,
                 cost_minor BIGINT NOT NULL DEFAULT 0,
                 currency VARCHAR NOT NULL,
                 segment_pid UUID NULL,
                 recipients INTEGER NOT NULL DEFAULT 0,
                 delivered INTEGER NOT NULL DEFAULT 0,
                 opened INTEGER NOT NULL DEFAULT 0,
                 clicked INTEGER NOT NULL DEFAULT 0,
                 unsubscribed INTEGER NOT NULL DEFAULT 0,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS nurture_sequences (
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
            "CREATE TABLE IF NOT EXISTS nurture_steps (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 sequence_pid UUID NOT NULL,
                 position INTEGER NOT NULL,
                 delay_hours INTEGER NOT NULL,
                 template_ref VARCHAR NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS nurture_enrollments (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 sequence_pid UUID NOT NULL,
                 contact_pid UUID NOT NULL,
                 current_step INTEGER NOT NULL DEFAULT 0,
                 next_due_at TIMESTAMPTZ NULL,
                 status VARCHAR NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS nurture_enrollments_due ON nurture_enrollments (next_due_at) WHERE status = 'active'",
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
        for table in ["nurture_enrollments", "nurture_steps", "nurture_sequences", "campaigns", "segments"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
