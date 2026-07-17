//! `sea-orm-migration` schema migrations for `patient-flow-service`.
//!
//! The [`Migrator`] runs the ordered list below at boot (or via
//! `cargo loco db migrate`): the physical topology (sites → wards →
//! bays → beds), the inpatient stays + transfers, the demand-side
//! tables (bed requests, Red2Green journal, infection flags), then the
//! `audit_logs` and `event_outbox` side tables. Stay data is personal
//! data, so the audit trail is the who/what/when record over every
//! change.

#![forbid(unsafe_code)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_topology;
mod m20220101_000002_stays;
mod m20220101_000003_demand;
mod m20220101_000004_audit_logs;
mod m20220101_000005_event_outbox;

/// The crate's migrator: drives the ordered migration set for the loco
/// CLI / boot-time migration.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered migration set. Order matters: topology first (stays
    /// reference beds/wards), then stays (the journal and flags
    /// reference them), then the side tables.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_topology::Migration),
            Box::new(m20220101_000002_stays::Migration),
            Box::new(m20220101_000003_demand::Migration),
            Box::new(m20220101_000004_audit_logs::Migration),
            Box::new(m20220101_000005_event_outbox::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
