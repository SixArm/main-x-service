//! Migration: the care-pathway **instance layer** — a patient enrolled
//! on a pathway template (`pathway_instances`; references a `person:`
//! URN and the template pid), the per-instance step-completion log
//! (`instance_steps`), the care-team roster (`instance_team`), and the
//! recorded instance events (`instance_events`). Operational state:
//! deliberately **not** part of the matcher payload (the registry owns
//! pathway identities; instances reference a person by URN).

use sea_orm_migration::prelude::*;

/// The instance-layer migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the four tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS pathway_instances (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     pathway_pid UUID NOT NULL,
                     subject_ref VARCHAR NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'active',
                     urgency VARCHAR NOT NULL DEFAULT 'routine',
                     enrolled_on DATE NOT NULL DEFAULT CURRENT_DATE,
                     next_review_on DATE NULL,
                     closed_on DATE NULL,
                     closure_reason VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS pathway_instances_pathway
                     ON pathway_instances (pathway_pid);
                 CREATE INDEX IF NOT EXISTS pathway_instances_subject
                     ON pathway_instances (subject_ref);
                 CREATE INDEX IF NOT EXISTS pathway_instances_status
                     ON pathway_instances (status);
                 CREATE TABLE IF NOT EXISTS instance_steps (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     instance_pid UUID NOT NULL,
                     label VARCHAR NOT NULL,
                     done BOOLEAN NOT NULL DEFAULT false,
                     done_on DATE NULL,
                     position INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS instance_steps_instance
                     ON instance_steps (instance_pid);
                 CREATE TABLE IF NOT EXISTS instance_team (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     instance_pid UUID NOT NULL,
                     member_ref VARCHAR NOT NULL,
                     role VARCHAR NOT NULL,
                     UNIQUE (instance_pid, member_ref, role)
                 );
                 CREATE INDEX IF NOT EXISTS instance_team_instance
                     ON instance_team (instance_pid);
                 CREATE TABLE IF NOT EXISTS instance_events (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     instance_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL,
                     occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     note VARCHAR NULL,
                     actor VARCHAR NULL
                 );
                 CREATE INDEX IF NOT EXISTS instance_events_instance
                     ON instance_events (instance_pid);",
            )
            .await?;
        Ok(())
    }

    /// Drop the four tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS instance_events;
                 DROP TABLE IF EXISTS instance_team;
                 DROP TABLE IF EXISTS instance_steps;
                 DROP TABLE IF EXISTS pathway_instances;",
            )
            .await?;
        Ok(())
    }
}
