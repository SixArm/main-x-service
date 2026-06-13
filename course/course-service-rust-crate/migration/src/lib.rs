//! Loco migrator for course-service.
//!
//! Each migration wraps the corresponding hand-written SQL file under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL remains the single source of truth while loco gains a
//! Rust `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

mod m20260604_000001_create_providers;
mod m20260604_000002_create_courses;
mod m20260604_000003_create_course_instances;
mod m20260604_000004_create_audit_and_review;
mod m20260608_000001_normalize_course_collections;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260604_000001_create_providers::Migration),
            Box::new(m20260604_000002_create_courses::Migration),
            Box::new(m20260604_000003_create_course_instances::Migration),
            Box::new(m20260604_000004_create_audit_and_review::Migration),
            Box::new(m20260608_000001_normalize_course_collections::Migration),
        ]
    }
}
