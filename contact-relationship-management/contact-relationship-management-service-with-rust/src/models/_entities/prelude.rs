//! Re-exports of every entity for `use ..._entities::prelude::*`.

pub use super::accounts::Entity as Accounts;
pub use super::activities::Entity as Activities;
pub use super::articles::Entity as Articles;
pub use super::audit_logs::Entity as AuditLogs;
pub use super::campaigns::Entity as Campaigns;
pub use super::consent_events::Entity as ConsentEvents;
pub use super::contacts::Entity as Contacts;
pub use super::deals::Entity as Deals;
pub use super::event_outbox::Entity as EventOutbox;
pub use super::forecast_snapshots::Entity as ForecastSnapshots;
pub use super::leads::Entity as Leads;
pub use super::nurture_enrollments::Entity as NurtureEnrollments;
pub use super::nurture_sequences::Entity as NurtureSequences;
pub use super::nurture_steps::Entity as NurtureSteps;
pub use super::pipeline_stages::Entity as PipelineStages;
pub use super::pipelines::Entity as Pipelines;
pub use super::segments::Entity as Segments;
pub use super::sla_policies::Entity as SlaPolicies;
pub use super::tickets::Entity as Tickets;
