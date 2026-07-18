//! Model layer: `SeaORM` entities (`_entities/`) plus helpers.
//!
//! CRM owns a **normalized relational schema** (CRM-D2) — pipelines,
//! consent history, SLA clocks and money sums are constraint-heavy.
//! Controllers query the entities directly; [`records`] provides the
//! shared finders and the `ActiveModelBehavior` impls; [`audit_logs`]
//! and [`event_outbox`] are the audit / durable-event side tables.

pub mod _entities;
pub mod audit_logs;
pub mod event_outbox;
pub mod records;
