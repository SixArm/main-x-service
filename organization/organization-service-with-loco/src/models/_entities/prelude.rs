//! `SeaORM` Entity prelude.
//!
//! Glob-importable aliases for the entity handles, so call sites can write
//! `AuditLogs` / `Organizations` instead of the fully-qualified paths.

/// The `audit_logs` entity handle.
pub use super::audit_logs::Entity as AuditLogs;
/// The `organizations` entity handle.
pub use super::organizations::Entity as Organizations;
