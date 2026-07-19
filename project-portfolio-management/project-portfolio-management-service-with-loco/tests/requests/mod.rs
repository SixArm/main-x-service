//! Request-level test suites, grouped by controller: the
//! `/api/{collection}` work-item suite in [`work_items`], the durable
//! outbox suite in [`event_outbox`], and the PPM Phase-A governance
//! suite (intake / gates / risks / budgets) in [`governance`].

mod event_outbox;
mod governance;
mod insights;
mod strategy;
mod visibility;
mod work_items;
