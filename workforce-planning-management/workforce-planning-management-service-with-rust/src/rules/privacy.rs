//! Pure rules for subject rights & retention (WPM-R30 / WPM-D22):
//! when erasure is allowed, how the retention horizon is read, and
//! which tables the sweep covers. No I/O.

/// Employee statuses in which erasure is allowed: the employment
/// relationship is the lawful basis for the data, so an active (or
/// on-leave, or onboarding) employee cannot be erased.
#[must_use]
pub fn erasable(status: &str) -> bool {
    matches!(status, "terminated" | "retired")
}

/// Default retention horizon (days) when `WPM_RETENTION_DAYS` is unset.
pub const RETENTION_DEFAULT_DAYS: i64 = 365;

/// Horizon floor: a sweep that could run at 0 days would silently turn
/// every soft-delete into a hard-delete (WPM-D22).
pub const RETENTION_FLOOR_DAYS: i64 = 30;

/// Parse the retention horizon from the raw env value: unset / blank /
/// junk ⇒ the default; anything below the floor is clamped up to it.
#[must_use]
pub fn retention_days(raw: Option<&str>) -> i64 {
    raw.and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(RETENTION_DEFAULT_DAYS)
        .max(RETENTION_FLOOR_DAYS)
}

/// Every table with a `deleted_at` column, for the retention sweep.
/// Kept in one place so a new soft-deleting table is added here (the
/// sweep test counts this list against the entity modules).
pub const SOFT_DELETED_TABLES: &[&str] = &[
    "adjustment_requests",
    "applications",
    "appraisals",
    "assessment_instruments",
    "assessments",
    "benchmarks",
    "benefit_enrollments",
    "benefit_plans",
    "candidates",
    "development_plans",
    "early_career_programs",
    "employee_skills",
    "employees",
    "ergonomic_assessments",
    "ergonomic_items",
    "feedback_entries",
    "goals",
    "interviews",
    "learning_paths",
    "leave_entitlements",
    "leave_requests",
    "mentorships",
    "onboarding_items",
    "path_enrollments",
    "payroll_runs",
    "payslips",
    "pipeline_members",
    "program_placements",
    "pulse_surveys",
    "requisitions",
    "review_cycles",
    "reviews",
    "shift_assignments",
    "shifts",
    "skills",
    "succession_candidates",
    "succession_plans",
    "talent_pipelines",
    "time_entries",
    "training_enrollments",
    "wellbeing_entitlements",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erasure_requires_a_closed_employment() {
        assert!(erasable("terminated") && erasable("retired"));
        for status in ["onboarding", "active", "on_leave", "offboarding"] {
            assert!(!erasable(status), "{status} still has a lawful basis");
        }
    }

    /// The horizon: default on unset/junk, floor-clamped, and a sane
    /// value passes through.
    #[test]
    fn retention_horizon_defaults_and_floors() {
        assert_eq!(retention_days(None), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("")), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("junk")), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("730")), 730);
        assert_eq!(retention_days(Some("0")), RETENTION_FLOOR_DAYS, "0 would hard-delete");
        assert_eq!(retention_days(Some("-5")), RETENTION_FLOOR_DAYS);
        assert_eq!(retention_days(Some("30")), 30);
    }

    /// The sweep list is sorted and duplicate-free (each table swept
    /// exactly once), and covers the known soft-deleting tables.
    #[test]
    fn sweep_table_list_is_sound() {
        let mut sorted = SOFT_DELETED_TABLES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, SOFT_DELETED_TABLES, "sorted and unique");
        assert_eq!(SOFT_DELETED_TABLES.len(), 41);
        for table in ["employees", "payslips", "candidates", "appraisals"] {
            assert!(SOFT_DELETED_TABLES.contains(&table));
        }
    }
}
