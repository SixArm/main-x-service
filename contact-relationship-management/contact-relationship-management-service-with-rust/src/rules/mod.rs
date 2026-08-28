//! The **pure core** (CRM-D3–D5): lifecycle machines, deterministic
//! lead scoring with breakdown, KPI arithmetic (forecast / ROI / CLV
//! / win rate), SLA deadline + breach derivation, and segment
//! evaluation with the structural consent gate — DB-free and
//! exhaustively unit-tested. Controllers wire these; they never
//! re-implement them.

pub mod analytics;
pub mod engagement;
pub mod lifecycle;
pub mod privacy;
pub mod scoring;
pub mod segment;
pub mod sla;
pub mod tokens;
