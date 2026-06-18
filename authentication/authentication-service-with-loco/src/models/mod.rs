//! Domain models layered over the generated `SeaORM` entities.

/// Generated `SeaORM` entity definitions.
pub mod _entities;
/// Auth-event model — durable audit trail of authentication events.
pub mod auth_events;
/// Session model — token issuance and revocation (`jid` = JWT `jti`).
pub mod sessions;
/// User model — passwordless magic-link accounts.
pub mod users;
