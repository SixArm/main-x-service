//! Migration: **time-based analysis** (TBA) — the recorded journey
//! segment (`instance_segments`) and the explicit pathway clock
//! (`pathway_instances.clock_start_at` / `clock_stop_at`).
//!
//! A segment is the missing primitive: everything the instance layer
//! recorded before was either a point in time (`instance_events`) or a
//! date with no start (`instance_steps`), and neither can answer "how
//! much of this journey was care?". See `spec/time-based-analysis.md`
//! §5 and §11.
//!
//! The clock columns are nullable and **backfilled** from the existing
//! `enrolled_on` / `closed_on` dates, so instances created before this
//! migration are analysable the moment it runs (at day resolution; the
//! analysis discloses which source it used).

use sea_orm_migration::prelude::*;

/// The time-based-analysis migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `instance_segments`; add + backfill the clock columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS instance_segments (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     instance_pid UUID NOT NULL,
                     label VARCHAR NOT NULL,
                     stage VARCHAR NOT NULL,
                     category VARCHAR NOT NULL,
                     waste VARCHAR NULL,
                     started_at TIMESTAMPTZ NOT NULL,
                     ended_at TIMESTAMPTZ NULL,
                     actor_ref VARCHAR NULL,
                     location_ref VARCHAR NULL,
                     note VARCHAR NULL,
                     position INTEGER NOT NULL DEFAULT 0
                 );
                 -- The per-instance read: every analysis loads one
                 -- instance's segments in time order.
                 CREATE INDEX IF NOT EXISTS instance_segments_instance
                     ON instance_segments (instance_pid, started_at);
                 -- Cohort aggregation groups by stage and by category.
                 CREATE INDEX IF NOT EXISTS instance_segments_stage
                     ON instance_segments (stage);
                 CREATE INDEX IF NOT EXISTS instance_segments_category
                     ON instance_segments (category);
                 -- At most one open segment per instance (spec §5.1
                 -- invariant 5): a partial UNIQUE index makes the rule a
                 -- database property, not only a handler check, so a
                 -- concurrent double-POST cannot open two.
                 CREATE UNIQUE INDEX IF NOT EXISTS instance_segments_one_open
                     ON instance_segments (instance_pid)
                     WHERE ended_at IS NULL;
                 ALTER TABLE pathway_instances
                     ADD COLUMN IF NOT EXISTS clock_start_at TIMESTAMPTZ NULL;
                 ALTER TABLE pathway_instances
                     ADD COLUMN IF NOT EXISTS clock_stop_at TIMESTAMPTZ NULL;
                 -- Backfill at day resolution from the existing dates.
                 UPDATE pathway_instances
                     SET clock_start_at = enrolled_on::timestamptz
                     WHERE clock_start_at IS NULL;
                 UPDATE pathway_instances
                     SET clock_stop_at = closed_on::timestamptz
                     WHERE clock_stop_at IS NULL AND closed_on IS NOT NULL;",
            )
            .await?;
        Ok(())
    }

    /// Drop the table and the two columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS instance_segments;
                 ALTER TABLE pathway_instances DROP COLUMN IF EXISTS clock_stop_at;
                 ALTER TABLE pathway_instances DROP COLUMN IF EXISTS clock_start_at;",
            )
            .await?;
        Ok(())
    }
}
