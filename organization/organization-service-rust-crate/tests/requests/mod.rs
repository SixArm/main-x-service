//! Request-level (HTTP) integration tests. Declares the organizations
//! suite, which boots the real loco app against the `test` config and is
//! therefore `#[ignore]`-gated on PostgreSQL.

mod organizations;
