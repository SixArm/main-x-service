//! Migration: the talent-acquisition tables (WPM-R1–R3) —
//! `requisitions`, `candidates` (consent-bounded), `applications`,
//! `interviews`, and the `onboarding_items` checklists.

use sea_orm_migration::prelude::*;

/// The acquisition-tables migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the five acquisition tables + lookup indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS requisitions (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 organization_ref VARCHAR NOT NULL,
                 department VARCHAR NOT NULL,
                 job_title VARCHAR NOT NULL,
                 headcount INTEGER NOT NULL,
                 salary_min_minor BIGINT NULL,
                 salary_max_minor BIGINT NULL,
                 salary_currency VARCHAR NULL,
                 status VARCHAR NOT NULL,
                 opened_on DATE NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS candidates (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 person_ref VARCHAR NULL,
                 display_name VARCHAR NOT NULL,
                 email VARCHAR NOT NULL,
                 source VARCHAR NOT NULL,
                 consent_until DATE NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS applications (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 requisition_pid UUID NOT NULL,
                 candidate_pid UUID NOT NULL,
                 stage VARCHAR NOT NULL,
                 notes VARCHAR NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS applications_requisition \
             ON applications (requisition_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS interviews (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 application_pid UUID NOT NULL,
                 scheduled_at TIMESTAMPTZ NOT NULL,
                 interviewer_ref VARCHAR NOT NULL,
                 outcome VARCHAR NOT NULL,
                 notes VARCHAR NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS onboarding_items (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 employee_pid UUID NOT NULL,
                 name VARCHAR NOT NULL,
                 mandatory BOOLEAN NOT NULL,
                 status VARCHAR NOT NULL,
                 waived_reason VARCHAR NULL,
                 completed_at TIMESTAMPTZ NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS onboarding_items_employee \
             ON onboarding_items (employee_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the acquisition tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in [
            "onboarding_items",
            "interviews",
            "applications",
            "candidates",
            "requisitions",
        ] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
