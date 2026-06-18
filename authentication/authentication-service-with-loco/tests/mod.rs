//! Integration-test entry point (loco convention).
//!
//! Aggregates the test submodules into one test binary. The model and
//! HTTP-request suites boot the loco app and so require PostgreSQL —
//! those tests are `#[ignore]`d to keep a DB-free `cargo test` green
//! (run them with `cargo test -- --ignored`). The DB-free contract /
//! shape assertions inside `requests` run by default.

mod models;
mod requests;
mod tasks;
mod workers;
