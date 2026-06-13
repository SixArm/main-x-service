#![allow(elided_lifetimes_in_paths, clippy::wildcard_imports)]

pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Case Tracking has no local tables. Folders live in
        // the Main Thing Service; move events live in the Main Event
        // Service; patients live in the Main Patient Service; places
        // live in the Main Place Service; workers live in the Main
        // Worker Service. Loco still requires a Migrator type even
        // when it has nothing to do.
        vec![]
    }
}
