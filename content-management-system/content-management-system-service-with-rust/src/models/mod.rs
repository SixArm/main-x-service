//! Model layer: `SeaORM` entities (`_entities/`) plus helpers.
//!
//! CMS uses a **hybrid** schema (CMS-D2): operator-declared shapes
//! (a content type's `fields`, a site's locale lists, a template's
//! regions) are JSONB validated in the pure core, while everything
//! carrying an invariant is a normalized, constraint-backed table.
//! Controllers query the entities directly; [`records`] provides the
//! shared finders and the `ActiveModelBehavior` impls; [`audit_logs`]
//! and [`event_outbox`] are the audit / durable-event side tables.

pub mod _entities;
pub mod audit_logs;
pub mod event_outbox;
pub mod records;
pub mod usage;
