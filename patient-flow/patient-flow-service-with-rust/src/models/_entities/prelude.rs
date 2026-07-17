//! Re-exports of every entity for `use ..._entities::prelude::*`.

pub use super::audit_logs::Entity as AuditLogs;
pub use super::bays::Entity as Bays;
pub use super::bed_requests::Entity as BedRequests;
pub use super::beds::Entity as Beds;
pub use super::event_outbox::Entity as EventOutbox;
pub use super::infection_flags::Entity as InfectionFlags;
pub use super::red_green_days::Entity as RedGreenDays;
pub use super::sites::Entity as Sites;
pub use super::stays::Entity as Stays;
pub use super::transfers::Entity as Transfers;
pub use super::wards::Entity as Wards;
