//! Request-level test suites, grouped by controller: the
//! `/api/plans` plan suite in [`plans`], the durable
//! outbox suite in [`event_outbox`], and the PPM Phase-A governance
//! suite (intake / gates / risks / budgets) in [`governance`].

mod capabilities;
mod event_outbox;
mod governance;
mod insights;
mod plans;
mod strategy;
mod visibility;
