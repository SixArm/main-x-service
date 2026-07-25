//! Migration: create the `audit_logs` table — the who/what/when trail.
//! Stay data is personal data, so every mutation **and every sensitive
//! read** (patient locate, stay detail) writes one row here recording
//! the entity kind, the action, the (optional) actor, and a snapshot.

use loco_rs::schema::{create_table, drop_table, ColType};
use sea_orm_migration::prelude::*;

/// The `audit_logs`-table migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `audit_logs` table.
    ///
    /// # Errors
    ///
    /// When the `CREATE TABLE` fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "audit_logs",
            &[
                ("id", ColType::PkAuto),
                // Which record kind the entry concerns: site | ward | bay
                // | bed | stay | bed_request | infection_flag | red_green.
                ("entity", ColType::String),
                // The record pid the entry concerns.
                ("entity_pid", ColType::Uuid),
                // created / updated / deleted / state actions
                // (bed_state_changed, stay_admitted, …) / sensitive reads
                // (locate_read, stay_read).
                ("action", ColType::String),
                // Optional actor (verified caller `sub`); null until a
                // bearer token is presented.
                ("actor", ColType::StringNull),
                // Snapshot / detail of the action (old + new state, the
                // override reason, …).
                ("snapshot", ColType::JsonBinaryNull),
            ],
            &[],
        )
        .await?;
        // The handover query: audit for one ward since a timestamp rides
        // the snapshot's ward_pid; the common per-record query is indexed.
        m.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS audit_logs_entity \
                 ON audit_logs (entity_pid)",
            )
            .await?;
        Ok(())
    }

    /// Drop the `audit_logs` table (rollback).
    ///
    /// # Errors
    ///
    /// When the `DROP TABLE` fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "audit_logs").await?;
        Ok(())
    }
}
