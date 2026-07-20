//! The **pure core** (HCM-D3–D5): every lifecycle state machine, the
//! leave / time / scheduling arithmetic, the org-chart cycle check,
//! payslip arithmetic, and benchmark flags — DB-free, clock-free, and
//! exhaustively unit-tested. Controllers wire these; they never
//! re-implement them.

pub mod benchmark;
pub mod learning;
pub mod leave;
pub mod lifecycle;
pub mod org;
pub mod payroll;
pub mod tokens;
pub mod workforce;
