//! Date/time conversion across the persistence boundary.
//!
//! Domain models and the row structs in [`super::models`] both use
//! `chrono` (`DateTime<Utc>`), which is what sqlx maps `TIMESTAMPTZ` to, so
//! reads and writes need no conversion. The one exception is the outbox's
//! pure envelope→row mapping ([`super::outbox::OutboxInsert`]), which is
//! stamped with a `time::OffsetDateTime`; this helper translates it.

use chrono::{DateTime, Utc};
use time::OffsetDateTime;

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`.
///
/// # Panics
///
/// Panics if the instant falls outside the nanosecond range of an `i64`
/// (years before 1677 or after 2262), which no real timestamp does.
#[must_use]
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp_nanos(
        i64::try_from(odt.unix_timestamp_nanos())
            .expect("time::OffsetDateTime within nanosecond range"),
    )
}
