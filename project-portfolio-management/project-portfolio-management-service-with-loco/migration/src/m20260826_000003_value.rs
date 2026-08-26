//! Migration: **realized gains** and **stakeholder sentiment** (entity
//! spec §5.9.6 / FR-33, FR-36).
//!
//! ## `approved_at` has no update path
//!
//! It is the Time-to-Value clock start. A clock start that can move is
//! not a measurement — a project could be made to look fast by
//! re-approving it. Written on insert, never touched again.
//!
//! ## Adoption stores its own definition
//!
//! "Active user" is the most quietly redefinable term in this whole
//! surface. A rate whose denominator and activity window are not stored
//! beside it cannot be compared across two quarters, let alone two
//! departments — so `definition` and `window_days` are columns, not
//! documentation. `target_users > 0` is enforced at **write**: a rate
//! with a zero denominator is refused rather than divided at read.
//!
//! ## Sentiment stores a role, never an identity
//!
//! A response is sentiment about a plan, not a record about a person.
//! Preventing double submission is a per-survey token's job, not a
//! reason to store who said what.

use sea_orm_migration::prelude::*;

/// The value migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the value and sentiment tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS business_case_targets (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     metric VARCHAR NOT NULL,
                     baseline_value BIGINT NOT NULL,
                     target_value BIGINT NOT NULL,
                     unit VARCHAR NULL,
                     currency VARCHAR NULL,
                     promised_by DATE NULL,
                     source VARCHAR NOT NULL DEFAULT 'charter'
                         CHECK (source IN ('charter', 'gate_review')),
                     -- The Time-to-Value clock start. Never updated.
                     approved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     approved_by_ref VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS business_case_targets_plan
                     ON business_case_targets (plan_pid);

                 CREATE TABLE IF NOT EXISTS value_points (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     benefit_pid UUID NULL,
                     observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     value BIGINT NOT NULL,
                     currency VARCHAR NULL,
                     is_first_measurable BOOLEAN NOT NULL DEFAULT false,
                     -- A measured figure and an asserted one are
                     -- different kinds of evidence; a realized-value
                     -- number that cannot say which has no audit standing.
                     method VARCHAR NOT NULL DEFAULT 'asserted'
                         CHECK (method IN ('measured', 'estimated', 'asserted')),
                     evidence_ref VARCHAR NULL,
                     actor VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS value_points_plan
                     ON value_points (plan_pid, observed_at);
                 -- At most one first-measurable point per plan: the
                 -- clock stops once.
                 CREATE UNIQUE INDEX IF NOT EXISTS value_points_first
                     ON value_points (plan_pid)
                     WHERE deleted_at IS NULL AND is_first_measurable;

                 CREATE TABLE IF NOT EXISTS adoption_snapshots (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     active_users BIGINT NOT NULL CHECK (active_users >= 0),
                     -- Refused at write, not divided at read.
                     target_users BIGINT NOT NULL CHECK (target_users > 0),
                     window_days INTEGER NOT NULL CHECK (window_days > 0),
                     -- Stored because 'active user' is the term most
                     -- easily redefined between two readings.
                     definition VARCHAR NOT NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS adoption_snapshots_plan
                     ON adoption_snapshots (plan_pid, observed_at DESC);

                 CREATE TABLE IF NOT EXISTS satisfaction_responses (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     surveyed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     instrument VARCHAR NOT NULL CHECK (instrument IN ('nps', 'csat')),
                     score SMALLINT NOT NULL CHECK (score BETWEEN 0 AND 10),
                     -- A role, never an identity.
                     respondent_role VARCHAR NOT NULL
                         CHECK (respondent_role IN ('sponsor', 'user', 'team', 'other')),
                     comment VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS satisfaction_responses_plan
                     ON satisfaction_responses (plan_pid, instrument);",
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
                "DROP TABLE IF EXISTS satisfaction_responses;
                 DROP TABLE IF EXISTS adoption_snapshots;
                 DROP TABLE IF EXISTS value_points;
                 DROP TABLE IF EXISTS business_case_targets;",
            )
            .await?;
        Ok(())
    }
}
