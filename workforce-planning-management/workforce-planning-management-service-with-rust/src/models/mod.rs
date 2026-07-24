//! Model layer: `SeaORM` entities (`_entities/`) plus helpers.
//!
//! HCM owns a **normalized relational schema** (HCM-D2) — not the
//! matcher-DTO-as-JSONB shape of the entity registries — because its
//! value is constraints and lifecycles. Controllers query the entities
//! directly; [`records`] provides the shared finders and the
//! `ActiveModelBehavior` impls; [`audit_logs`] and [`event_outbox`] are
//! the audit / durable-event side tables.

pub mod _entities;
pub mod audit_logs;
pub mod event_outbox;
pub mod records;
