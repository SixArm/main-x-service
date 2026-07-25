// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths, clippy::wildcard_imports)]
//! Database migrations crate.
//!
//! This consumer app defines **no** tables of its own — all domain data
//! lives in external Main-X services. Loco/SeaORM still require a
//! `Migrator` type to wire up, so this crate provides one whose
//! migration list is empty.

pub use sea_orm_migration::prelude::*;

/// The `SeaORM` migrator for this crate. Exists to satisfy Loco; it owns
/// no migrations because there are no local tables.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// Return the ordered list of migrations to apply.
    ///
    /// Always empty: this consumer app has no local schema. See the
    /// inline note for where each domain's data actually lives.
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
