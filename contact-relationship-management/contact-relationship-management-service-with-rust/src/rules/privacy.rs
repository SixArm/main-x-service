//! Pure rules for subject rights & retention (CRM-R20 / CRM-D14):
//! when erasure is allowed, how the retention horizon is read, and
//! which tables the sweep covers. No I/O.

/// A contact is erasable when it holds no **live commercial
/// engagement**: no open deal naming it primary contact, no open
/// support ticket, and no active nurture enrolment.
///
/// This deliberately does **not** gate on `Contact::status`
/// (`active`/`inactive`), unlike WPM's analogous `erasable(status)`
/// over the employee's employment status. WPM's `status` genuinely
/// transitions through an offboarding lifecycle and is the real
/// lawful-basis signal; CRM's `Contact::status` is set once at
/// creation (always `"active"`) and no endpoint in this crate ever
/// transitions it, so gating on it would refuse erasure forever. The
/// signals below — an open deal, an open ticket, an active nurture
/// enrolment — are the ones that actually change over a contact's
/// lifetime, so they are the real lawful-basis check: each represents
/// live business activity that still needs the contact's identity.
#[must_use]
pub fn erasable(has_open_deal: bool, has_open_ticket: bool, has_active_nurture: bool) -> bool {
    !has_open_deal && !has_open_ticket && !has_active_nurture
}

/// Default retention horizon (days) when `CRM_RETENTION_DAYS` is unset.
pub const RETENTION_DEFAULT_DAYS: i64 = 365;

/// Horizon floor: a sweep that could run at 0 days would silently turn
/// every soft-delete into a hard-delete (CRM-D14, mirroring WPM-D22).
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
/// sweep test counts this list against the entity modules). Unlike
/// WPM's 41-table list this is **not** scoped to one subject — it is
/// every soft-deleting table in the CRM schema, including account-only
/// tables (`memberships`, `partnerships`) that carry no `contact_pid`
/// at all. `consent_events` and `working_group_members` are excluded
/// on purpose: the former is append-only compliance evidence with no
/// `deleted_at` column (CRM-D6), the latter is a plain roster join row
/// with no soft-delete of its own.
pub const SOFT_DELETED_TABLES: &[&str] = &[
    "accounts",
    "activities",
    "articles",
    "campaigns",
    "contacts",
    "deals",
    "forecast_snapshots",
    "leads",
    "memberships",
    "nurture_enrollments",
    "nurture_sequences",
    "nurture_steps",
    "partnerships",
    "pipeline_stages",
    "pipelines",
    "segments",
    "sla_policies",
    "tickets",
    "working_groups",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Erasure requires no live engagement in any of the three forms;
    /// any one of them refuses it.
    #[test]
    fn erasure_requires_no_live_engagement() {
        assert!(erasable(false, false, false));
        assert!(!erasable(true, false, false), "open deal blocks erasure");
        assert!(!erasable(false, true, false), "open ticket blocks erasure");
        assert!(
            !erasable(false, false, true),
            "active nurture enrolment blocks erasure"
        );
        assert!(!erasable(true, true, true));
    }

    /// The horizon: default on unset/junk, floor-clamped, and a sane
    /// value passes through.
    #[test]
    fn retention_horizon_defaults_and_floors() {
        assert_eq!(retention_days(None), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("")), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("junk")), RETENTION_DEFAULT_DAYS);
        assert_eq!(retention_days(Some("730")), 730);
        assert_eq!(
            retention_days(Some("0")),
            RETENTION_FLOOR_DAYS,
            "0 would hard-delete"
        );
        assert_eq!(retention_days(Some("-5")), RETENTION_FLOOR_DAYS);
        assert_eq!(retention_days(Some("30")), 30);
    }

    /// The sweep list is sorted and duplicate-free (each table swept
    /// exactly once), and covers the known soft-deleting tables —
    /// while excluding the two tables that structurally cannot be
    /// swept this way.
    #[test]
    fn sweep_table_list_is_sound() {
        let mut sorted = SOFT_DELETED_TABLES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, SOFT_DELETED_TABLES, "sorted and unique");
        assert_eq!(SOFT_DELETED_TABLES.len(), 19);
        for table in ["contacts", "accounts", "deals", "tickets", "leads"] {
            assert!(SOFT_DELETED_TABLES.contains(&table));
        }
        for table in ["consent_events", "working_group_members"] {
            assert!(
                !SOFT_DELETED_TABLES.contains(&table),
                "{table} has no deleted_at column"
            );
        }
    }
}
