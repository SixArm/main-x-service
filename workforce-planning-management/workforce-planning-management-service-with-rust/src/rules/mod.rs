//! The **pure core** (WPM-D3–D5): every lifecycle state machine, the
//! leave / time / scheduling arithmetic, the org-chart cycle check,
//! payslip arithmetic, and benchmark flags — DB-free, clock-free, and
//! exhaustively unit-tested. Controllers wire these; they never
//! re-implement them.

pub mod appraisal;
pub mod assessment;
pub mod benchmark;
pub mod learning;
pub mod leave;
pub mod lifecycle;
pub mod org;
pub mod payroll;
pub mod privacy;
pub mod talent;
pub mod pulse;
pub mod tokens;
pub mod wellbeing;
pub mod workforce;
pub mod working_time;
