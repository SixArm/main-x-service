#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_care_pathways;
mod m20220101_000002_audit_logs;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_care_pathways::Migration),
            Box::new(m20220101_000002_audit_logs::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
