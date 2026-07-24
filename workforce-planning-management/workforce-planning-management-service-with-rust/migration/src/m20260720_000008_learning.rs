//! Migration: the learning & development tables — a `skills` catalog
//! and declared `employee_skills` (proficiency 1–5 + optional
//! target), `learning_paths` (+ ordered `learning_path_steps` of
//! course refs) with per-employee `path_enrollments`, and
//! `mentorships` (+ a `mentorship_sessions` log). All declared /
//! recorded data; the derived views live in the controller.

use sea_orm_migration::prelude::*;

/// The learning migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the seven tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS skills (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL UNIQUE,
                     category VARCHAR NOT NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE TABLE IF NOT EXISTS employee_skills (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     employee_pid UUID NOT NULL,
                     skill_pid UUID NOT NULL,
                     proficiency INTEGER NOT NULL,
                     target INTEGER NULL,
                     assessed_on DATE NOT NULL DEFAULT CURRENT_DATE,
                     deleted_at TIMESTAMPTZ NULL,
                     UNIQUE (employee_pid, skill_pid)
                 );
                 CREATE TABLE IF NOT EXISTS learning_paths (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL,
                     summary VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE TABLE IF NOT EXISTS learning_path_steps (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     path_pid UUID NOT NULL,
                     course_ref VARCHAR NOT NULL,
                     title VARCHAR NOT NULL,
                     position INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS learning_path_steps_path
                     ON learning_path_steps (path_pid);
                 CREATE TABLE IF NOT EXISTS path_enrollments (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     path_pid UUID NOT NULL,
                     employee_pid UUID NOT NULL,
                     enrolled_on DATE NOT NULL DEFAULT CURRENT_DATE,
                     deleted_at TIMESTAMPTZ NULL,
                     UNIQUE (path_pid, employee_pid)
                 );
                 CREATE TABLE IF NOT EXISTS mentorships (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     mentor_pid UUID NOT NULL,
                     mentee_pid UUID NOT NULL,
                     focus VARCHAR NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'proposed',
                     started_on DATE NULL,
                     ended_on DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS mentorships_mentor
                     ON mentorships (mentor_pid);
                 CREATE INDEX IF NOT EXISTS mentorships_mentee
                     ON mentorships (mentee_pid);
                 CREATE TABLE IF NOT EXISTS mentorship_sessions (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     mentorship_pid UUID NOT NULL,
                     held_on DATE NOT NULL,
                     notes VARCHAR NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS mentorship_sessions_mentorship
                     ON mentorship_sessions (mentorship_pid);",
            )
            .await?;
        Ok(())
    }

    /// Drop the seven tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS mentorship_sessions;
                 DROP TABLE IF EXISTS mentorships;
                 DROP TABLE IF EXISTS path_enrollments;
                 DROP TABLE IF EXISTS learning_path_steps;
                 DROP TABLE IF EXISTS learning_paths;
                 DROP TABLE IF EXISTS employee_skills;
                 DROP TABLE IF EXISTS skills;",
            )
            .await?;
        Ok(())
    }
}
