//! The **pure flow core** (PF-D4): bed state machine, allocation
//! rules, and journey logic (`Red2Green`, DTOC, LOS) as DB-free pure
//! functions, exhaustively unit-tested. Controllers stay thin: they
//! load rows, call these functions, persist the outcome.

pub mod allocation;
pub mod bed_state;
pub mod journey;
pub mod tokens;
