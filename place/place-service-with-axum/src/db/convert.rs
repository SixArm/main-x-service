//! Date/time conversions across the persistence boundary.
//!
//! Domain models and the row structs in [`super::models`] both use
//! `chrono` (`DateTime<Utc>`), which is what sqlx maps `TIMESTAMPTZ` to, so
//! most reads and writes need no conversion. The outbox's pure
//! envelope→row mapping ([`super::outbox::OutboxInsert`]) is stamped with
//! a `time::OffsetDateTime`; these helpers translate between the two.

use chrono::{DateTime, Utc};
use time::OffsetDateTime;

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC), or `None` when
/// the instant is outside what [`time::OffsetDateTime`] holds (years
/// −9999 to 9999). Goes through whole seconds and the sub-second part
/// rather than an `i64` of nanoseconds, which would stop at 2262.
///
/// Use this for an instant a caller supplied: a request's `since=2999-…`
/// is a `422`, not a panic.
#[must_use]
pub fn try_ts_to_offset(ts: DateTime<Utc>) -> Option<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp(ts.timestamp())
        .ok()?
        .replace_nanosecond(ts.timestamp_subsec_nanos())
        .ok()
}

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC), for an instant
/// that came from the database or the clock.
///
/// # Panics
///
/// Panics outside the years −9999 to 9999, which a stored timestamp or the
/// clock does not reach. A caller-supplied instant goes through
/// [`try_ts_to_offset`] instead.
#[must_use]
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    try_ts_to_offset(ts).expect("DateTime within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`, through whole
/// seconds and the sub-second part. Total: chrono's range (±262 143
/// years) contains `time`'s (±9 999), so unlike an `i64` of nanoseconds,
/// which stops at 2262, no `OffsetDateTime` is out of reach.
///
/// # Panics
///
/// Never in practice: only if chrono's documented range stopped
/// containing `time`'s.
#[must_use]
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp(odt.unix_timestamp(), odt.nanosecond())
        .expect("chrono's range contains time::OffsetDateTime's")
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    /// A caller's instant converts across the whole range `time` holds, not
    /// only the nanosecond range that ends in 2262, and keeps its sub-second
    /// part; past year 9999 it is `None`, not a panic.
    #[test]
    fn try_ts_to_offset_is_total_over_caller_input() {
        use chrono::TimeZone;
        let far = Utc.with_ymd_and_hms(2999, 1, 1, 0, 0, 0).unwrap();
        let odt = super::try_ts_to_offset(far).expect("2999 is representable");
        assert_eq!(odt.year(), 2999);
        let precise = DateTime::from_timestamp(1_700_000_000, 123_456_789).unwrap();
        let odt = super::try_ts_to_offset(precise).unwrap();
        assert_eq!(odt.nanosecond(), 123_456_789);
        assert_eq!(super::offset_to_ts(odt), precise, "round trip");
        let far_back = super::offset_to_ts(super::try_ts_to_offset(far).unwrap());
        assert_eq!(far_back, far, "2999 round-trips");
        let beyond = Utc.with_ymd_and_hms(10_000, 1, 1, 0, 0, 0).unwrap();
        assert!(super::try_ts_to_offset(beyond).is_none());
    }
}
