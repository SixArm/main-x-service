//! Integration-test crate root: declares the request-level test suite.
//! The DB-free matcher/storage tests live in the sibling `matching.rs`
//! binary; the request suite under `requests/` is Postgres-gated.

mod requests;
