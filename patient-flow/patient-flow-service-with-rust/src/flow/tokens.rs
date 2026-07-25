//! Closed token vocabularies for the string-typed domain columns.
//!
//! Each `const` slice is the closed set the validators accept; the
//! database stores the token verbatim (PF-D3 keeps the schema plain
//! strings so the vocabulary can grow by data migration, not DDL).

/// Ward kinds. Virtual wards skip the cleaning cycle (PF-D8).
pub const WARD_KINDS: &[&str] = &["inpatient", "assessment", "virtual"];

/// Bay sex designations (allocation rule 2).
pub const SEX_DESIGNATIONS: &[&str] = &["male", "female", "mixed", "flexible"];

/// Patient sex tokens accepted on a bed request.
pub const SEXES: &[&str] = &["male", "female", "other", "unknown"];

/// Bed closure reasons.
pub const CLOSURE_REASONS: &[&str] = &["infection", "maintenance", "staffing", "other"];

/// Stay admission sources.
pub const STAY_SOURCES: &[&str] = &["ed", "elective", "transfer_in", "virtual_admission"];

/// Stay statuses.
pub const STAY_STATUSES: &[&str] = &["admitted", "discharge_ready", "discharged"];

/// Discharge-to-assess pathways (P0–P3).
pub const DISCHARGE_PATHWAYS: &[&str] = &["p0", "p1", "p2", "p3"];

/// Discharge destinations.
pub const DISCHARGE_DESTINATIONS: &[&str] = &[
    "home",
    "home_with_support",
    "community_hospital",
    "care_home",
    "other_acute",
    "deceased",
    "self_discharge",
];

/// Transfer reasons.
pub const TRANSFER_REASONS: &[&str] = &[
    "admission",
    "clinical",
    "capacity",
    "isolation",
    "patient_request",
    "discharge",
    "step_up",
    "step_down",
];

/// Bed-request origins.
pub const REQUEST_ORIGINS: &[&str] = &[
    "ed",
    "elective",
    "ward_transfer",
    "external",
    "virtual_step_up",
];

/// Bed-request priorities.
pub const REQUEST_PRIORITIES: &[&str] = &["emergency", "urgent", "routine"];

/// Bed-request statuses.
pub const REQUEST_STATUSES: &[&str] = &["open", "allocated", "fulfilled", "cancelled"];

/// Infection precaution classes.
pub const PRECAUTIONS: &[&str] = &["contact", "droplet", "airborne", "protective"];

/// Infection flag statuses.
pub const FLAG_STATUSES: &[&str] = &["suspected", "confirmed", "cleared"];

/// `Red2Green` day classifications.
pub const RED_GREEN: &[&str] = &["red", "green"];

/// Coded `Red2Green` delay reasons (spec `domain-model.md`).
pub const DELAY_REASONS: &[&str] = &[
    "awaiting_senior_review",
    "awaiting_diagnostics",
    "awaiting_pharmacy",
    "awaiting_transport",
    "awaiting_therapy_assessment",
    "awaiting_social_care",
    "awaiting_community_bed",
    "awaiting_care_package",
    "family_choice",
    "internal_process",
    "other",
];

/// Whether `value` is a member of the closed set `set`.
#[must_use]
pub fn is_token(set: &[&str], value: &str) -> bool {
    set.contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_is_exact_and_case_sensitive() {
        assert!(is_token(WARD_KINDS, "virtual"));
        assert!(!is_token(WARD_KINDS, "Virtual"));
        assert!(!is_token(WARD_KINDS, "icu"));
        assert!(is_token(DELAY_REASONS, "awaiting_transport"));
        assert!(!is_token(DELAY_REASONS, ""));
    }
}
