//! Loco CLI tasks.
//!
//! loco [`Task`](loco_rs::task::Task) implementations, registered in
//! `App::register_tasks` (see [`crate::app`]).
//!
//! - [`attributes::UserAttributes`] (`user_attributes`) — the operator
//!   surface for ABAC attribute assignment
//!   (`agents/share/authorization-attributes.md` §6).

pub mod attributes;
